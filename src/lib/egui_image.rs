#![allow(dead_code)]
pub use egui::*;
pub struct EguiImage {
    image: image::RgbaImage,
    texture: Option<TextureHandle>,
    modified: bool,
}

impl EguiImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            image: image::RgbaImage::new(width, height),
            texture: None,
            modified: false,
        }
    }

    pub fn load(path: &str) -> Result<Self, image::ImageError> {
        Ok(Self {
            image: image::open(path)?.to_rgba8(),
            texture: None,
            modified: false,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
        Ok(Self {
            image: image::load_from_memory(bytes)?.to_rgba8(),
            texture: None,
            modified: false,
        })
    }

    pub fn draw<R, F: FnOnce(&mut image::RgbaImage) -> R>(&mut self, draw: F) -> R {
        draw(&mut self.image)
    }

    pub fn image(&self) -> &image::RgbaImage {
        &self.image
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        let texture = self.texture.get_or_insert_with(|| {
            self.modified = false;
            ui.ctx().load_texture(
                "egui-image",
                ColorImage::from_rgba_unmultiplied(
                    [self.image.width() as _, self.image.height() as _],
                    self.image.as_flat_samples().as_slice(),
                ),
                Default::default(),
            )
        });
        if self.modified {
            texture.set(
                ColorImage::from_rgba_unmultiplied(
                    [self.image.width() as _, self.image.height() as _],
                    self.image.as_flat_samples().as_slice(),
                ),
                Default::default(),
            );
        }
        ui.add(Image::new(
            texture as &TextureHandle,
            texture.size_vec2(),
        ))
    }
}
