use crate::project::*;
use itertools::Itertools;
use std::collections::HashMap;

// * ----------------------------------------------------------------------------------- MAP ---------------------------------------------------------------------------------- * //
#[derive(Serialize, Deserialize)]
pub struct Object {
    #[serde(default)]
    pub pos: I32Vec2,
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub components: HashMap<Uuid, HashMap<String, String>>,
}

impl Object {
    pub fn new(pos: I32Vec2, components: &[(Uuid, HashMap<String, String>)]) -> Self {
        Self {
            pos,
            always_on_top: false,
            components: components.iter().cloned().collect(),
        }
    }

    pub fn altas<'a>(&mut self, atlases: &'a HashMap<Uuid, Atlas>) -> Result<Option<&'a Atlas>> {
        if let Some(atlas) = self.components.get(&ATLAS_RENDERER_UUID) {
            if let Some(atlas) = atlases.get(
                &Uuid::parse_str(
                    atlas
                        .get("Atlas")
                        .context("[PROBABLY A BUG] Atlas component without atlas!")?,
                )
                .context("[PROBABLY A BUG] Atlas UUID in atlas renderer is invalid!")?,
            ) {
                Ok(Some(atlas))
            } else {
                self.components.remove(&ATLAS_RENDERER_UUID);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Map {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(default)]
    pub size: U16Vec2,
    #[serde(default)]
    pub data: Vec<I16Vec2>,
    #[serde(default)]
    pub atlas: Uuid,
    #[serde(default)]
    pub objects: HashMap<Uuid, Object>,
}

impl Map {
    pub fn new(path: &Path, size: U16Vec2, atlas: Uuid) -> Self {
        Self {
            path: path.to_path_buf(),
            size,
            data: vec![TVec2::new(-1, -1); size.x as usize * size.y as usize],
            atlas,
            objects: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            ..serde_json::from_str(
                &std::fs::read_to_string(path)
                    .context(format!("Failed to load map from file {:?}!", path))?,
            )
            .context(format!("Failed to deserialize map from file {:?}!", path))?
        })
    }

    pub fn save(&self) -> Result<()> {
        std::fs::write(
            &self.path,
            serde_json::to_string(&self)
                .context(format!("Failed to serialize map! File: {:?}!", self.path))?,
        )
        .context(format!("Failed to save map to file {:?}!", self.path))?;
        Ok(())
    }

    pub fn get_tile(&self, pos: I16Vec2) -> I16Vec2 {
        if pos < TVec2::zeros() || pos >= self.size.casted() {
            TVec2::new(-1, -1)
        } else {
            self.data[pos.x as usize + pos.y as usize * self.size.x as usize]
        }
    }

    pub fn set_tile(&mut self, pos: I16Vec2, tile: I16Vec2) {
        if pos >= TVec2::zeros() && pos < self.size.casted() {
            self.data[pos.x as usize + pos.y as usize * self.size.x as usize] = tile;
        }
    }
}

#[derive(Clone)]
pub enum EditingMode {
    None,
    Tile,
    Object { uuid: Uuid, drag_offset: I32Vec2 },
}

#[derive(Clone)]
pub struct MapView {
    pub map: Uuid,
    pub camera: I32Vec2,
    pub scale: f32,
    pub hover_tile: Option<U32Vec2>,
    pub mode: EditingMode,
}

impl MapView {
    pub fn new(map: Uuid) -> Self {
        Self {
            map,
            camera: TVec2::zeros(),
            scale: 1.0,
            hover_tile: None,
            mode: EditingMode::None,
        }
    }
}

// * ---------------------------------------------------------------------------------- SHOW ---------------------------------------------------------------------------------- * //
pub fn show(ui: &mut Ui, assets: &mut Assets, status: &mut RichText) -> Result<()> {
    if let Some(view) = assets.map_selected.as_mut() {
        let map = assets
            .maps
            .get_mut(&view.map)
            .context("[PROBABLY A BUG] Map was not found! Perhaps it was deleted?")?;
        let atlas = assets.atlases.get(&map.atlas).context(
            "[PROBABLY A BUG] Atlas for selected map was not found! Perhaps it was deleted?",
        )?;

        ui.horizontal(|ui| {
            let old_size = map.size;
            ui.add(egui::DragValue::new(&mut map.size.x).speed(0.05));
            ui.add(egui::DragValue::new(&mut map.size.y).speed(0.05));

            if map.size != old_size {
                let mut new_data =
                    vec![TVec2::new(-1, -1); map.size.x as usize * map.size.y as usize];
                for x in 0..map.size.x.min(old_size.x) {
                    for y in 0..map.size.y.min(old_size.y) {
                        new_data[x as usize + y as usize * map.size.x as usize] =
                            map.data[x as usize + y as usize * old_size.x as usize];
                    }
                }
                map.data = new_data;
            }
        });

        let mut image = EguiImage::new(ui.available_size().x as _, ui.available_size().y as _);

        let tile_size = atlas.tile_size.casted() * view.scale;
        let atlas_view = if let Some(atlas_view) = assets.atlas_selected.as_ref() {
            if atlas_view.atlas == map.atlas {
                Some(atlas_view)
            } else {
                None
            }
        } else {
            None
        };
        let block_size = if let Some(atlas_view) = atlas_view {
            atlas_view.selection_size
        } else {
            TVec2::new(1, 1)
        };

        image.draw(|canvas| -> Result<()> {
            let camera_tile =
                &max(&view.camera.casted().component_div(&tile_size), 0.0).casted::<i32>();
            let br_tile = min2(
                &((view.camera.casted() + canvas.size().casted()).component_div(&tile_size)
                    + 1.casted()),
                &map.size.casted(),
            )
            .casted();
            for y in camera_tile.y..br_tile.y {
                for x in camera_tile.x..br_tile.x {
                    let tile = map.get_tile(TVec2::new(x, y).casted());
                    if tile < TVec2::zeros() {
                        continue;
                    }
                    atlas.draw_tile(
                        canvas,
                        TVec2::new(x, y).casted().component_mul(&tile_size).casted() - view.camera,
                        tile.casted(),
                        ceil(&tile_size).casted(),
                    );
                }
            }

            let mut draw_object = |uuid: &Uuid, object: &mut Object| -> Result<()> {
                if let Some(atlas) = object.altas(&assets.atlases)? {
                    let (pos, size) = (
                        (object.pos.casted() * view.scale).casted() - view.camera,
                        (atlas.tile_size.casted() * view.scale).casted(),
                    );
                    canvas.draw_subimage(
                        &atlas.image,
                        pos,
                        size,
                        TVec2::zeros(),
                        atlas.tile_size.casted(),
                    );
                    if assets.object_selected == Some(*uuid) {
                        canvas.draw_rect(pos, size, image::Rgba([255, 0, 0, 255]), 3);
                    }
                } else {
                    canvas.fill_rect(
                        ((object.pos - 1.casted()).casted() * view.scale).casted() - view.camera,
                        (view.scale * 3.0).casted(),
                        if assets.object_selected == Some(*uuid) {
                            image::Rgba([255, 0, 0, 255])
                        } else {
                            image::Rgba([0, 0, 255, 255])
                        },
                    );
                };
                Ok(())
            };

            for (uuid, object) in &mut map.objects {
                if !object.always_on_top {
                    draw_object(uuid, object)?;
                }
            }

            for (uuid, object) in &mut map.objects {
                if object.always_on_top {
                    draw_object(uuid, object)?;
                }
            }

            if let Some(hover_tile) = view.hover_tile {
                canvas.draw_rect(
                    hover_tile.casted().component_mul(&tile_size).casted() - view.camera,
                    tile_size.casted(),
                    image::Rgba([255, 0, 0, 255]),
                    3,
                );
            }

            Ok(())
        })?;

        let response = image.ui(ui);
        if let Some(pos) = response.hover_pos() {
            let pos = pos - response.rect.min;
            let pixel = ((pos.casted() + view.camera).casted() / view.scale).casted();
            let hover_tile = (pos.casted() + view.camera)
                .casted()
                .component_div(&tile_size)
                .casted::<i32>();
            if hover_tile >= TVec2::zeros() && hover_tile < map.size.casted() {
                view.hover_tile = Some(hover_tile.casted());
                *status = RichText::new(format!(
                    "Tile: ({:.2}, {:.2})",
                    pixel.x as f32 / atlas.tile_size.x as f32,
                    pixel.y as f32 / atlas.tile_size.y as f32
                ));

                // * Panning and zooming
                if ui.input(|input| input.pointer.button_down(PointerButton::Middle)) {
                    view.camera -= ui.input(|input| input.pointer.delta().casted());
                }
                let wheel = ui.input(|input| input.scroll_delta.y) / 50.0;
                if wheel != 0.0 {
                    let zoom_factor = 1.7f32;
                    let zoom_delta = zoom_factor.powf(wheel);
                    view.scale *= zoom_delta;
                    view.camera = ((view.camera + pos.casted()).casted() * zoom_delta).casted()
                        - pos.casted();
                }

                // * Placing
                if ui.input(|input| input.modifiers.shift) {
                    // Object
                    if ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
                        map.objects.insert(
                            Uuid::new_v4(),
                            if ui.input(|input| input.modifiers.ctrl) {
                                if let Some(atlas) = &assets.atlas_selected {
                                    Object::new(
                                        pixel,
                                        &[(
                                            ATLAS_RENDERER_UUID,
                                            [("Atlas".to_owned(), atlas.atlas.to_string())]
                                                .into_iter()
                                                .collect(),
                                        )],
                                    )
                                } else {
                                    Object::new(pixel, &[])
                                }
                            } else {
                                Object::new(pixel, &[])
                            },
                        );
                    }
                } else {
                    if ui.input(|input| {
                        input.pointer.button_pressed(PointerButton::Primary)
                            || input.pointer.button_pressed(PointerButton::Secondary)
                    }) {
                        view.mode = map
                            .objects
                            .iter_mut()
                            .find_map(|(uuid, object)| -> Option<EditingMode> {
                                let rect = if let Some(atlas) =
                                    object.altas(&assets.atlases).expect("Failed to get atlas!")
                                {
                                    (object.pos, object.pos + atlas.tile_size.casted())
                                } else {
                                    (object.pos - 1.casted(), object.pos + 2.casted())
                                };
                                if pixel >= rect.0 && pixel < rect.1 {
                                    assets.object_selected = Some(*uuid);
                                    assets.component_selected = None;
                                    return Some(EditingMode::Object {
                                        uuid: *uuid,
                                        drag_offset: pixel - rect.0,
                                    });
                                }

                                None
                            })
                            .unwrap_or(EditingMode::Tile);
                    }

                    // Place Tile / Drag object
                    if ui.input(|input| input.pointer.button_down(PointerButton::Primary)) {
                        // Object
                        // if let EditingMode::Object { uuid, drag_offset } = view.mode {
                        //     map.objects
                        //         .get_mut(&uuid)
                        //         .context("[PROBABLY A BUG] dragging non-existing object!")?
                        //         .pos = pixel - drag_offset;
                        // } else
                        if matches!(view.mode, EditingMode::Tile) {
                            if let Some(atlas_view) = atlas_view {
                                // Tiles
                                for x in 0..block_size.x {
                                    for y in 0..block_size.y {
                                        let offset = TVec2::new(x, y).casted();
                                        map.set_tile(
                                            hover_tile.casted() + offset,
                                            atlas_view.selection_pos.casted() + offset,
                                        );
                                    }
                                }
                            }
                        }
                    } else if ui.input(|input| input.pointer.button_down(PointerButton::Secondary))
                    {
                        // Erase Tile / Remove object
                        // Object
                        if let EditingMode::Object {
                            uuid,
                            drag_offset: _,
                        } = view.mode
                        {
                            map.objects
                                .remove(&uuid)
                                .context("[PROBABLY A BUG] removing non-existing object!")?;
                            assets.object_selected = None;
                        } else if matches!(view.mode, EditingMode::Tile) {
                            map.set_tile(hover_tile.casted(), TVec2::new(-1, -1));
                        }
                    }
                }
            } else {
                view.hover_tile = None;
            }
        } else {
            view.hover_tile = None;
        }

        if let Some(uuid) = assets.object_selected {
            let object = map.objects.get_mut(&uuid).unwrap();
            object.pos += ui.input(|input| {
                TVec2::new(
                    input.key_pressed(Key::ArrowRight) as i32
                        - input.key_pressed(Key::ArrowLeft) as i32,
                    input.key_pressed(Key::ArrowDown) as i32
                        - input.key_pressed(Key::ArrowUp) as i32,
                )
            });
        }
    } else {
        ui.label("Click on map in content viewer to select it!");
    }

    Ok(())
}

pub fn export<W: std::io::Write>(assets: &mut Assets, file: &mut W) -> Result<()> {
    file.write_u16::<LittleEndian>(assets.maps.len() as _)
        .context("Failed to export map count!")?;

    let valid_maps = assets.maps.keys().copied().collect::<Vec<_>>();
    let mut component_indices = HashMap::new();
    let mut map_indices = HashMap::new();

    // * Fill component indices
    for (index, (uuid, _)) in assets
        .components
        .iter_mut()
        .sorted_by_key(|x| x.1.path.clone())
        .enumerate()
    {
        component_indices.insert(*uuid, index);
    }

    // * Fill map indices
    for (index, (uuid, _)) in assets
        .maps
        .iter_mut()
        .sorted_by_key(|x| x.1.path.clone())
        .enumerate()
    {
        map_indices.insert(*uuid, index);
    }

    // * Export maps
    for (_uuid, map) in assets.maps.iter_mut().sorted_by_key(|x| x.1.path.clone()) {
        file.write_u16::<LittleEndian>(map.size.x)?;
        file.write_u16::<LittleEndian>(map.size.y)?;
        file.write_u16::<LittleEndian>(assets.atlas_indices[&map.atlas])?;
        for y in 0..map.size.y {
            for x in 0..map.size.x {
                let tile = map.get_tile(TVec2::new(x, y).casted());
                file.write_u16::<LittleEndian>(if tile < TVec2::zeros() {
                    0xFFFF
                } else {
                    (tile.x + tile.y * assets.atlases[&map.atlas].width() as i16) as _
                })?;
            }
        }

        // * Export objects
        file.write_u16::<LittleEndian>(map.objects.len() as _)?;
        for object in map.objects.values_mut() {
            file.write_i32::<LittleEndian>(object.pos.x)?;
            file.write_i32::<LittleEndian>(object.pos.y)?;
            file.write_u8(if object.always_on_top { 1 } else { 0 })?;

            file.write_u16::<LittleEndian>(object.components.len() as _)?;
            for (uuid, properties) in &mut object.components {
                let component = assets
                    .components
                    .get(uuid)
                    .context("[PROBABLY A BUG] Failed to get component while exporting map!")?;
                component.fix_instance(
                    properties,
                    assets.atlases.keys().copied(),
                    valid_maps.iter().copied(),
                );

                // * Export component
                file.write_u16::<LittleEndian>(*component_indices.get(uuid).context(
                    "[PROBABLY A BUG] Failed to get component index while exporting map!",
                )? as _)?;
                for (name, property_type) in &component.properties {
                    let value = properties.get(name).context("[PROBABLY A BUG] Failed to get object's component property while exporting map!")?;
                    let context = format!(
                        "[PROBABLY A BUG] Failed to parse numeric property {}!",
                        value
                    );

                    // * Export property value
                    match property_type {
                        Property::I8 => {
                            file.write_i8(value.parse().context(context)?)?;
                        }
                        Property::U8 => {
                            file.write_u8(value.parse().context(context)?)?;
                        }
                        Property::I16 => {
                            file.write_i16::<LittleEndian>(value.parse().context(context)?)?;
                        }
                        Property::U16 => {
                            file.write_u16::<LittleEndian>(value.parse().context(context)?)?;
                        }
                        Property::I32 => {
                            file.write_i32::<LittleEndian>(value.parse().context(context)?)?;
                        }
                        Property::U32 => {
                            file.write_u32::<LittleEndian>(value.parse().context(context)?)?;
                        }
                        Property::F32 => {
                            file.write_f32::<LittleEndian>(value.parse().context(context)?)?;
                        }
                        Property::String => {
                            file.write_u16::<LittleEndian>(value.as_bytes().len() as _)?;
                            file.write_all(value.as_bytes())?;
                        }
                        Property::Atlas => {
                            file.write_u16::<LittleEndian>(
                                assets.atlas_indices[&Uuid::parse_str(value).context(format!(
                                    "[PROBABLY A BUG] Failed to parse atlas property {}!",
                                    value
                                ))?],
                            )?;
                        }
                        Property::Map => {
                            file.write_u16::<LittleEndian>(
                                map_indices[&Uuid::parse_str(value).context(format!(
                                    "[PROBABLY A BUG] Failed to parse map property {}!",
                                    value
                                ))?] as _,
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
