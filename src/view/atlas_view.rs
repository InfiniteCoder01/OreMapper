use crate::project::*;
use itertools::Itertools;

pub fn show(ui: &mut Ui, assets: &mut Assets) {
    if let Some(view) = assets.atlas_selected.as_mut() {
        if let Some(atlas) = assets.atlases.get(&view.atlas) {
            let scale = (ui.available_size().x as f32 / atlas.image.width() as f32)
                .min(ui.available_size().y as f32 / atlas.image.height() as f32);
            let tile_size = &atlas.tile_size.as_type() * scale;

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
                        .as_type()
                        .component_mul(&tile_size)
                        .as_type(),
                    view.selection_size
                        .as_type()
                        .component_mul(&tile_size)
                        .as_type(),
                    image::Rgba([255, 0, 0, 255]),
                    3,
                );
            });
            let response = image.ui(ui);
            if let Some(pos) = response.hover_pos() {
                let pos = pos - response.rect.min;
                let tile_pos = min2(&max(&pos.as_type(), 0), &response.rect.max.as_type())
                    .as_type()
                    .component_div(&tile_size)
                    .as_type();
                if ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
                    view.selection_pos = tile_pos;
                }
                if ui.input(|input| input.pointer.button_down(PointerButton::Primary)) {
                    view.selection_size = (tile_pos - view.selection_pos).add_scalar(1);
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
    file.write_u16::<LittleEndian>(assets.atlases.len() as u16)
        .context("Failed to write atlas count to exported file!")?;
    assets.atlas_indices.clear();
    for (index, (uuid, atlas)) in assets
        .atlases
        .iter()
        .sorted_by_key(|x| &x.1.path)
        .enumerate()
    {
        assets.atlas_indices.insert(uuid.clone(), index as u16);
        file.write_u16::<LittleEndian>(atlas.tile_size.x as u16)
            .context("Failed to write atlas tile width to exported file!")?;
        file.write_u16::<LittleEndian>(atlas.tile_size.y as u16)
            .context("Failed to write atlas tile height to exported file!")?;
        file.write_u16::<LittleEndian>(
            (atlas.image.width() / atlas.tile_size.x as u32 * atlas.image.height()
                / atlas.tile_size.y as u32) as u16,
        )
        .context("Failed to write atlas frame count to exported file!")?;

        // Export image itself
        for y in 0..atlas.height() {
            for x in 0..atlas.width() {
                for pixel_y in 0..atlas.tile_size.y {
                    for pixel_x in 0..atlas.tile_size.x {
                        let pixel = atlas.image.get_pixel(
                            x as u32 * atlas.tile_size.x as u32 + pixel_x as u32,
                            y as u32 * atlas.tile_size.y as u32 + pixel_y as u32,
                        );
                        file.write_u16::<BigEndian>(if pixel[3] < 128 {
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
