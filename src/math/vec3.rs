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

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn add(&self, other: &Vec3) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }

    pub fn sub(&self, other: &Vec3) -> Self {
        Self { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }

    pub fn rotate_x(&self, angle: f32) -> Self {
        let sin = angle.sin();
        let cos = angle.cos();
        Self { x: self.x, y: self.y * cos - self.z * sin, z: self.y * sin + self.z * cos }
    }

    pub fn rotate_y(&self, angle: f32) -> Self {
        let sin = angle.sin();
        let cos = angle.cos();
        Self { x: self.x * cos + self.z * sin, y: self.y, z: -self.x * sin + self.z * cos }
    }

    pub fn rotate_z(&self, angle: f32) -> Self {
        let sin = angle.sin();
        let cos = angle.cos();
        Self { x: self.x * cos - self.y * sin, y: self.x * sin + self.y * cos, z: self.z }
    }
}

impl Add<Vec3> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Self::Output {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Self::Output {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs }
    }
}