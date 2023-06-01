#![allow(dead_code)]
pub use nalgebra_glm::*;
pub type F32Vec2 = TVec2<f32>;

pub trait Vec2Cast {
    fn as_type<T: num::NumCast>(&self) -> TVec2<T>;
}

impl<T1: num::ToPrimitive + Copy + Scalar> Vec2Cast for TVec2<T1> {
    fn as_type<T2: num::NumCast>(&self) -> TVec2<T2> {
        TVec2::new(T2::from(self.x).expect("Failed to cast!"), T2::from(self.y).expect("Failed to cast!"))
    }
}
