use crate::project::*;
use itertools::Itertools;

pub fn show(ui: &mut Ui, assets: &mut Assets, status: &mut RichText) -> Result<()> {
    if let Some(view) = assets.map_selected.as_mut() {
        let map = assets
            .maps
            .get_mut(&view.map)
            .context("[PROBABLY A BUG] Map was not found! Perhaps it was deleted?")?;
        let atlas = assets.atlases.get(&map.atlas).context(
            "[PROBABLY A BUG] Atlas for selected map was not found! Perhaps it was deleted?",
        )?;
        let mut image = EguiImage::new(ui.available_size().x as _, ui.available_size().y as _);

        let tile_size = (atlas.tile_size.as_type() * view.scale).as_type();
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
            let camera_tile = &max(&view.camera.component_div(&tile_size.as_type()), 0);
            let br_tile = min2(
                &(view.camera + canvas.size().as_type())
                    .component_div(&tile_size.as_type())
                    .add_scalar(1),
                &map.size.as_type(),
            )
            .as_type();
            for y in camera_tile.y..br_tile.y {
                for x in camera_tile.x..br_tile.x {
                    let tile = map.get_tile(TVec2::new(x, y).as_type());
                    if tile < TVec2::zeros() {
                        continue;
                    }
                    atlas.draw_tile(
                        canvas,
                        TVec2::new(x, y).component_mul(&tile_size).as_type() - view.camera,
                        tile.as_type(),
                        tile_size.as_type(),
                    );
                }
            }

            for (uuid, object) in &mut map.objects {
                if let Some(atlas) = object.altas(&assets.atlases)? {
                    let (pos, size) = (
                        (object.pos.as_type() * view.scale).as_type() - view.camera,
                        (atlas.tile_size.as_type() * view.scale).as_type(),
                    );
                    canvas.draw_subimage(
                        &atlas.image,
                        pos,
                        size,
                        TVec2::zeros(),
                        atlas.tile_size.as_type(),
                    );
                    if assets.object_selected == Some(*uuid) {
                        canvas.draw_rect(pos, size, image::Rgba([255, 0, 0, 255]), 3);
                    }
                } else {
                    canvas.fill_rect(
                        ((object.pos - 1.as_type()).as_type() * view.scale).as_type() - view.camera,
                        (view.scale * 3.0).as_type(),
                        if assets.object_selected == Some(*uuid) {
                            image::Rgba([255, 0, 0, 255])
                        } else {
                            image::Rgba([0, 0, 255, 255])
                        },
                    );
                };
            }

            if let Some(hover_tile) = view.hover_tile {
                canvas.draw_rect(
                    hover_tile.component_mul(&tile_size.as_type()).as_type() - view.camera,
                    tile_size.as_type(),
                    image::Rgba([255, 0, 0, 255]),
                    3,
                );
            }

            Ok(())
        })?;

        let response = image.ui(ui);
        if let Some(pos) = response.hover_pos() {
            let pos = pos - response.rect.min;
            let pixel = ((pos.as_type() + view.camera).as_type() / view.scale).as_type();
            let hover_tile = (pos.as_type() + view.camera)
                .as_type()
                .component_div(&tile_size);
            if hover_tile >= TVec2::zeros() && hover_tile < map.size.as_type() {
                view.hover_tile = Some(hover_tile.as_type());
                *status = RichText::new(format!(
                    "Tile: ({:.2}, {:.2})",
                    pixel.x as f32 / atlas.tile_size.x as f32,
                    pixel.y as f32 / atlas.tile_size.y as f32
                ));

                // * Panning and zooming
                if ui.input(|input| input.pointer.button_down(PointerButton::Middle)) {
                    view.camera -= ui.input(|input| input.pointer.delta().as_type());
                }
                let wheel = ui.input(|input| input.scroll_delta.y) / 50.0;
                if wheel != 0.0 {
                    let zoom_factor = 1.7f32;
                    let zoom_delta = zoom_factor.powf(wheel);
                    view.scale *= zoom_delta;
                    view.camera = ((view.camera + pos.as_type()).as_type() * zoom_delta).as_type()
                        - pos.as_type();
                }

                // * Placing
                if ui.input(|input| input.modifiers.shift) {
                    // Object
                    if ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
                        map.objects.insert(
                            Uuid::new_v4(),
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
                            },
                        );
                    }
                } else if ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
                    // Select Object
                    assets.object_selected =
                        map.objects
                            .iter_mut()
                            .find_map(|(uuid, object)| -> Option<Uuid> {
                                if let Some(atlas) =
                                    object.altas(&assets.atlases).expect("Failed to get atlas!")
                                {
                                    if pixel >= object.pos
                                        && pixel <= object.pos + atlas.tile_size.as_type()
                                    {
                                        return Some(*uuid);
                                    }
                                } else {
                                    if pixel >= object.pos.add_scalar(-1)
                                        && pixel <= object.pos.add_scalar(1)
                                    {
                                        return Some(*uuid);
                                    }
                                }
                                None
                            });
                    assets.component_selected = None;
                } else if ui.input(|input| input.pointer.button_down(PointerButton::Primary)) {
                    if let Some(atlas_view) = atlas_view {
                        // Tiles
                        for x in 0..block_size.x {
                            for y in 0..block_size.y {
                                let offset = TVec2::new(x, y).as_type();
                                map.set_tile(
                                    hover_tile.as_type() + offset,
                                    atlas_view.selection_pos.as_type() + offset,
                                );
                            }
                        }
                    }
                } else if ui.input(|input| input.pointer.button_down(PointerButton::Secondary)) {
                    // Erase
                    map.set_tile(hover_tile.as_type(), TVec2::new(-1, -1));
                }
            } else {
                view.hover_tile = None;
            }
        } else {
            view.hover_tile = None;
        }
    } else {
        ui.label("Click on map in content viewer to select it!");
    }

    Ok(())
}

pub fn export<W: std::io::Write>(assets: &Assets, file: &mut W) -> Result<()> {
    file.write_u16::<LittleEndian>(assets.maps.len() as u16)
        .context("Failed to write map count to exported file!")?;
    for (_uuid, map) in assets.maps.iter().sorted_by_key(|x| &x.1.path) {
        file.write_u16::<LittleEndian>(map.size.x as u16)
            .context("Failed to write map width to exported file!")?;
        file.write_u16::<LittleEndian>(map.size.y as u16)
            .context("Failed to write map height to exported file!")?;
        file.write_u16::<LittleEndian>(assets.atlas_indices[&map.atlas])
            .context("Failed to write atlas index for map to exported file!")?;
        for y in 0..map.size.y {
            for x in 0..map.size.x {
                let tile = map.get_tile(TVec2::new(x, y).as_type());
                file.write_u16::<BigEndian>(if tile < TVec2::zeros() {
                    0xFFFF
                } else {
                    (tile.x + tile.y * assets.atlases[&map.atlas].width() as i16) as _
                })
                .context("Failed to write map tile to exported file!")?;
            }
        }
    }
    Ok(())
}
