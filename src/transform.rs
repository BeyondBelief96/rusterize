//! Transform component for 3D objects.
//!
//! Provides a [`Transform`] struct with a fluent API for managing position,
//! rotation (Euler angles), and scale.

use crate::math::{mat4::Mat4, vec3::Vec3};

/// A 3D transform with position, rotation (Euler angles), and scale.
///
/// Provides a fluent API where mutating methods return `&mut Self` for chaining:
///
/// ```ignore
/// transform
///     .set_position_xyz(5.0, 2.0, 0.0)
///     .rotate_y(0.1)
///     .set_scale_uniform(2.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    position: Vec3,
    rotation: Vec3, // Euler angles in radians: x=pitch, y=yaw, z=roll
    scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Create a new transform with default values (position=0, rotation=0, scale=1).
    pub fn new() -> Self {
        Self::default()
    }

    // ============ Position ============

    /// Get the position.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Set the position.
    pub fn set_position(&mut self, position: Vec3) -> &mut Self {
        self.position = position;
        self
    }

    /// Set the position from x, y, z components.
    pub fn set_position_xyz(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        self.position = Vec3::new(x, y, z);
        self
    }

    /// Translate by a delta vector.
    pub fn translate(&mut self, delta: Vec3) -> &mut Self {
        self.position = self.position + delta;
        self
    }

    /// Translate along the X axis.
    pub fn translate_x(&mut self, dx: f32) -> &mut Self {
        self.position.x += dx;
        self
    }

    /// Translate along the Y axis.
    pub fn translate_y(&mut self, dy: f32) -> &mut Self {
        self.position.y += dy;
        self
    }

    /// Translate along the Z axis.
    pub fn translate_z(&mut self, dz: f32) -> &mut Self {
        self.position.z += dz;
        self
    }

    // ============ Rotation ============

    /// Get the rotation (Euler angles in radians).
    pub fn rotation(&self) -> Vec3 {
        self.rotation
    }

    /// Set the rotation (Euler angles in radians).
    pub fn set_rotation(&mut self, rotation: Vec3) -> &mut Self {
        self.rotation = rotation;
        self
    }

    /// Set the rotation from x, y, z components (radians).
    pub fn set_rotation_xyz(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        self.rotation = Vec3::new(x, y, z);
        self
    }

    /// Add a delta rotation (Euler angles in radians).
    pub fn rotate(&mut self, delta: Vec3) -> &mut Self {
        self.rotation = self.rotation + delta;
        self
    }

    /// Rotate around the X axis (pitch).
    pub fn rotate_x(&mut self, angle: f32) -> &mut Self {
        self.rotation.x += angle;
        self
    }

    /// Rotate around the Y axis (yaw).
    pub fn rotate_y(&mut self, angle: f32) -> &mut Self {
        self.rotation.y += angle;
        self
    }

    /// Rotate around the Z axis (roll).
    pub fn rotate_z(&mut self, angle: f32) -> &mut Self {
        self.rotation.z += angle;
        self
    }

    // ============ Scale ============

    /// Get the scale.
    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    /// Set the scale.
    pub fn set_scale(&mut self, scale: Vec3) -> &mut Self {
        self.scale = scale;
        self
    }

    /// Set uniform scale (same value for x, y, z).
    pub fn set_scale_uniform(&mut self, s: f32) -> &mut Self {
        self.scale = Vec3::new(s, s, s);
        self
    }

    /// Multiply the current scale by a factor vector.
    pub fn scale_by(&mut self, factor: Vec3) -> &mut Self {
        self.scale.x *= factor.x;
        self.scale.y *= factor.y;
        self.scale.z *= factor.z;
        self
    }

    /// Multiply the current scale uniformly.
    pub fn scale_uniform(&mut self, factor: f32) -> &mut Self {
        self.scale.x *= factor;
        self.scale.y *= factor;
        self.scale.z *= factor;
        self
    }

    // ============ Matrix Generation ============

    /// Generate the transformation matrix.
    ///
    /// Order: Translation * RotationX * RotationY * RotationZ * Scale
    /// (Scale applied first, then rotations, then translation)
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::translation(self.position.x, self.position.y, self.position.z)
            * Mat4::rotation_x(self.rotation.x)
            * Mat4::rotation_y(self.rotation.y)
            * Mat4::rotation_z(self.rotation.z)
            * Mat4::scaling(self.scale.x, self.scale.y, self.scale.z)
    }

    /// Generate the normal matrix for lighting calculations.
    ///
    /// This is the inverse transpose of the rotation+scale matrix (excludes translation).
    /// Correctly handles non-uniform scaling.
    pub fn normal_matrix(&self) -> Mat4 {
        let rotation_scale = Mat4::rotation_x(self.rotation.x)
            * Mat4::rotation_y(self.rotation.y)
            * Mat4::rotation_z(self.rotation.z)
            * Mat4::scaling(self.scale.x, self.scale.y, self.scale.z);

        rotation_scale
            .inverse()
            .unwrap_or(Mat4::identity())
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default() {
        let t = Transform::default();
        assert_eq!(t.position(), Vec3::ZERO);
        assert_eq!(t.rotation(), Vec3::ZERO);
        assert_eq!(t.scale(), Vec3::ONE);
    }

    #[test]
    fn test_fluent_api() {
        let mut t = Transform::new();
        t.set_position_xyz(1.0, 2.0, 3.0)
            .rotate_y(0.5)
            .set_scale_uniform(2.0);

        assert_eq!(t.position(), Vec3::new(1.0, 2.0, 3.0));
        assert_relative_eq!(t.rotation().y, 0.5);
        assert_eq!(t.scale(), Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_translate() {
        let mut t = Transform::new();
        t.set_position_xyz(1.0, 0.0, 0.0).translate_x(2.0);
        assert_eq!(t.position().x, 3.0);
    }

    #[test]
    fn test_scale_by() {
        let mut t = Transform::new();
        t.set_scale(Vec3::new(2.0, 3.0, 4.0)).scale_uniform(2.0);
        assert_eq!(t.scale(), Vec3::new(4.0, 6.0, 8.0));
    }

    #[test]
    fn test_to_matrix_identity() {
        let t = Transform::default();
        let m = t.to_matrix();
        // Default transform should produce identity matrix
        assert_eq!(m, Mat4::identity());
    }
}
