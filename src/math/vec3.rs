use std::ops::{Add, Sub, Mul, Div};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0, z: 0.0};
    pub const LEFT: Self = Self { x: -1.0, y: 0.0, z: 0.0};
    pub const UP: Self = Self { x: 0.0, y: 1.0, z: 0.0};
    pub const DOWN: Self = Self { x: 0.0, y: -1.0, z: 0.0 };
    pub const FORWARD: Self = Self { x: 0.0, y: 0.0, z: 1.0};
    pub const BACK: Self = Self { x: 0.0, y: 0.0, z: -1.0};
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Self { x, y, z}
    }

    
}