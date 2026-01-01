//! 4D vector for homogeneous coordinates.

use std::ops::{Add, Div, Mul, Neg, Sub};

use super::vec3::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Create a point (w=1) from x, y, z coordinates.
    pub const fn point(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z, 1.0)
    }

    /// Create a direction vector (w=0) from x, y, z coordinates.
    pub const fn direction(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z, 0.0)
    }

    /// Create a Vec4 from a Vec3 with specified w component.
    pub const fn from_vec3(v: Vec3, w: f32) -> Self {
        Self::new(v.x, v.y, v.z, w)
    }

    /// Convert to Vec3, discarding w.
    pub const fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Convert to Vec3 with perspective division (divide by w).
    pub fn to_vec3_perspective(self) -> Vec3 {
        if self.w != 0.0 && self.w != 1.0 {
            Vec3::new(self.x / self.w, self.y / self.w, self.z / self.w)
        } else {
            Vec3::new(self.x, self.y, self.z)
        }
    }

    pub fn magnitude(&self) -> f32 {
        (self.x.powi(2) + self.y.powi(2) + self.z.powi(2) + self.w.powi(2)).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        Self::new(self.x / mag, self.y / mag, self.z / mag, self.w / mag)
    }

    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    pub fn scale(&self, scalar: f32) -> Self {
        Self::new(
            self.x * scalar,
            self.y * scalar,
            self.z * scalar,
            self.w * scalar,
        )
    }

    /// Linearly interpolate between two vectors.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
            self.z + (other.z - self.z) * t,
            self.w + (other.w - self.w) * t,
        )
    }
}

impl Add<Vec4> for Vec4 {
    type Output = Vec4;

    fn add(self, rhs: Vec4) -> Self::Output {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl Sub<Vec4> for Vec4 {
    type Output = Vec4;

    fn sub(self, rhs: Vec4) -> Self::Output {
        Self::new(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
            self.w - rhs.w,
        )
    }
}

impl Mul<f32> for Vec4 {
    type Output = Vec4;

    fn mul(self, rhs: f32) -> Self::Output {
        self.scale(rhs)
    }
}

impl Div<f32> for Vec4 {
    type Output = Vec4;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
    }
}

impl Neg for Vec4 {
    type Output = Vec4;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl From<Vec3> for Vec4 {
    /// Convert Vec3 to Vec4 as a point (w=1).
    fn from(v: Vec3) -> Self {
        Self::point(v.x, v.y, v.z)
    }
}

impl From<Vec4> for Vec3 {
    /// Convert Vec4 to Vec3, discarding w.
    fn from(v: Vec4) -> Self {
        v.to_vec3()
    }
}
