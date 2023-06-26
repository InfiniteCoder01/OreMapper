use crate::project::*;
use itertools::Itertools;

// * ---------------------------------------------------------------------------------- ATLAS --------------------------------------------------------------------------------- * //
#[derive(Serialize, Deserialize)]
pub struct Atlas {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub image: image::RgbaImage,
    #[serde(default)]
    pub tile_size: U16Vec2,
}

impl Atlas {
    pub fn load(path: &Path) -> Result<Self> {
        let image = image::open(path.to_str().unwrap())
            .context("Failed to load IMAGE!")?
            .to_rgba8();

        let atl_path = path.with_extension("atl");
        if atl_path.exists() {
            Ok(Self {
                path: path.to_path_buf(),
                image,
                ..serde_json::from_str(
                    &std::fs::read_to_string(atl_path)
                        .context(format!("Failed to load atlas from file {:?}!", path))?,
                )
                .context(format!("Failed to deserialize atlas from file {:?}!", path))?
            })
        } else {
            let tile_size = TVec2::new(image.width(), image.height()).casted();
            Ok(Self {
                path: path.to_path_buf(),
                image,
                tile_size,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        std::fs::write(
            self.path.with_extension("atl"),
            serde_json::to_string(&self)
                .context(format!("Failed to serialize atlas! File: {:?}!", self.path))?,
        )
        .context(format!("Failed to save atlas to file {:?}!", self.path))?;
        Ok(())
    }

    pub fn width(&self) -> u16 {
        (self.image.width() / self.tile_size.x as u32) as u16
    }

    pub fn height(&self) -> u16 {
        (self.image.height() / self.tile_size.y as u32) as u16
    }

    pub fn draw_tile(&self, to: &mut image::RgbaImage, pos: I32Vec2, tile: U32Vec2, size: I32Vec2) {
        to.draw_subimage(
            &self.image,
            pos.casted(),
            size,
            tile.component_mul(&self.tile_size.casted()),
            self.tile_size.casted(),
        );
    }
}

#[derive(Clone)]
pub struct AtlasView {
    pub atlas: Uuid,
    pub selection_pos: U32Vec2,
    pub selection_size: U32Vec2,
}

impl AtlasView {
    pub fn new(atlas: Uuid) -> Self {
        Self {
            atlas,
            selection_pos: U32Vec2::zeros(),
            selection_size: U32Vec2::new(1, 1),
        }
    }
}

// * ---------------------------------------------------------------------------------- SHOW ---------------------------------------------------------------------------------- * //
pub fn show(ui: &mut Ui, assets: &mut Assets) {
    if let Some(view) = assets.atlas_selected.as_mut() {
        if let Some(atlas) = assets.atlases.get_mut(&view.atlas) {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut atlas.tile_size.x).clamp_range(1..=65535));
                ui.add(egui::DragValue::new(&mut atlas.tile_size.y).clamp_range(1..=65535));
            });

            let scale = (ui.available_size().x / atlas.image.width() as f32)
                .min(ui.available_size().y / atlas.image.height() as f32);
            let tile_size = atlas.tile_size.casted() * scale;

            let mut image = EguiImage::new(
                (atlas.image.width() as f32 * scale) as u32,
                (atlas.image.height() as f32 * scale) as u32,
            );

            image.draw(|canvas| {
                canvas.draw_image(
                    &atlas.image,
                    I32Vec2::zeros(),
                    Some(I32Vec2::new(canvas.width() as _, canvas.height() as _)),
                );
                canvas.draw_rect(
                    view.selection_pos
                        .casted()
                        .component_mul(&tile_size)
                        .casted(),
                    view.selection_size
                        .casted()
                        .component_mul(&tile_size)
                        .casted(),
                    image::Rgba([255, 0, 0, 255]),
                    3,
                );
            });
            let response = image.ui(ui);
            if let Some(pos) = response.hover_pos() {
                let pos = pos - response.rect.min;
                let tile_pos = min2(&max(&pos.casted(), 0), &response.rect.max.casted())
                    .casted()
                    .component_div(&tile_size)
                    .casted();
                if ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
                    view.selection_pos = tile_pos;
                }
                if ui.input(|input| input.pointer.button_down(PointerButton::Primary)) {
                    view.selection_size = tile_pos - view.selection_pos + 1.casted();
                }
            }
        } else {
            assets.atlas_selected = None;
        }
    } else {
        ui.label("Click on atlas in content viewer to select it!");
    }
}

pub fn export<W: std::io::Write>(assets: &mut Assets, file: &mut W) -> Result<()> {
    file.write_u16::<LittleEndian>(assets.atlases.len() as _)?;
    assets.atlas_indices.clear();
    for (index, (uuid, atlas)) in assets
        .atlases
        .iter()
        .sorted_by_key(|x| &x.1.path)
        .enumerate()
    {
        assets.atlas_indices.insert(*uuid, index as u16);
        file.write_u16::<LittleEndian>(atlas.tile_size.x)?;
        file.write_u16::<LittleEndian>(atlas.tile_size.y)?;
        file.write_u16::<LittleEndian>(
            (atlas.image.width() / atlas.tile_size.x as u32 * atlas.image.height()
                / atlas.tile_size.y as u32) as _,
        )?;

        // Export the image itself
        for y in 0..atlas.height() {
            for x in 0..atlas.width() {
                for pixel_y in 0..atlas.tile_size.y {
                    for pixel_x in 0..atlas.tile_size.x {
                        let pixel = atlas.image.get_pixel(
                            x as u32 * atlas.tile_size.x as u32 + pixel_x as u32,
                            y as u32 * atlas.tile_size.y as u32 + pixel_y as u32,
                        );
                        file.write_u16::<byteorder::BigEndian>(if pixel[3] < 128 {
                            0xF81F
                        } else {
                            let r = (pixel[0] >> 3) as u16;
                            let g = (pixel[1] >> 2) as u16;
                            let b = (pixel[2] >> 3) as u16;
                            r << 11 | g << 5 | b
                        })
                        .context("Failed to write atlas RGB pixel to exported file!")?;
                    }
                }
            }
        }
    }
    Ok(())
}
