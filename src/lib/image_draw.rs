use nalgebra_glm::*; // TODO: negative sizes

pub trait DrawTarget {
    fn size(&self) -> U32Vec2;
    fn fill_rect(&mut self, pos: I32Vec2, size: I32Vec2, color: image::Rgba<u8>);
    fn draw_rect(&mut self, pos: I32Vec2, size: I32Vec2, color: image::Rgba<u8>, thickness: u32);
    fn draw_image(&mut self, image: &image::RgbaImage, pos: I32Vec2, size: Option<I32Vec2>);
    fn draw_subimage(
        &mut self,
        image: &image::RgbaImage,
        pos: I32Vec2,
        size: I32Vec2,
        subpos: U32Vec2,
        subsize: U32Vec2,
    );
}

impl DrawTarget for image::RgbaImage {
    fn size(&self) -> U32Vec2 {
        U32Vec2::new(self.width(), self.height())
    }

    fn fill_rect(&mut self, pos: I32Vec2, size: I32Vec2, color: image::Rgba<u8>) {
        let top_left = max(&pos, 0);
        let bottom_right = min2(
            &(pos + size),
            &I32Vec2::new(self.width() as _, self.height() as _),
        );
        for y in top_left.y..bottom_right.y {
            for x in top_left.x..bottom_right.x {
                self.put_pixel(x as _, y as _, color);
            }
        }
    }

    fn draw_rect(&mut self, pos: I32Vec2, size: I32Vec2, color: image::Rgba<u8>, thickness: u32) {
        self.fill_rect(pos, I32Vec2::new(size.x, thickness as _), color);
        self.fill_rect(pos, I32Vec2::new(thickness as _, size.y), color);
        self.fill_rect(
            pos + I32Vec2::new(size.x - thickness as i32, 0),
            I32Vec2::new(thickness as _, size.y),
            color,
        );
        self.fill_rect(
            pos + I32Vec2::new(0, size.y - thickness as i32),
            I32Vec2::new(size.x, thickness as _),
            color,
        );
    }

    fn draw_image(&mut self, image: &image::RgbaImage, pos: I32Vec2, size: Option<I32Vec2>) {
        self.draw_subimage(
            image,
            pos,
            size.unwrap_or(I32Vec2::new(image.width() as _, image.height() as _)),
            U32Vec2::zeros(),
            image.size(),
        )
    }

    fn draw_subimage(
        &mut self,
        image: &image::RgbaImage,
        pos: I32Vec2,
        size: I32Vec2,
        subpos: U32Vec2,
        subsize: U32Vec2,
    ) {
        let top_left = max(&pos, 0);
        let bottom_right = min2(
            &(pos + size),
            &I32Vec2::new(self.width() as _, self.height() as _),
        );
        for y in top_left.y..bottom_right.y {
            for x in top_left.x..bottom_right.x {
                let uv = (I32Vec2::new(x, y) - pos)
                    .component_mul(&subsize.cast())
                    .component_div(&size)
                    + subpos.cast();
                let pixel = image.get_pixel(uv.x as _, uv.y as _);
                if pixel[3] > 128 {
                    self.put_pixel(x as _, y as _, *pixel);
                }
            }
        }
    }
}
