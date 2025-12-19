//! Lighting types for the renderer.

use crate::prelude::Vec3;

/// A directional light that illuminates the scene uniformly from a direction.
///
/// Directional lights are ideal for simulating distant light sources like the sun,
/// where all rays are effectively parallel.
pub struct DirectionalLight {
    /// The normalized direction the light is pointing (not where it comes from).
    pub direction: Vec3,
    pub ambient_intensity: f32,
    /// Multiplier for the diffuse lighting contribution (default: 1.0)
    pub diffuse_strength: f32,
}

impl DirectionalLight {
    /// Create a new directional light pointing in the given direction.
    /// The direction will be normalized automatically.
    pub fn new(direction: Vec3) -> Self {
        DirectionalLight {
            direction: direction.normalize(),
            ambient_intensity: 0.1,
            diffuse_strength: 1.0,
        }
    }

    /// Calculate light intensity for flat shading.
    ///
    /// Returns intensity in [0.0, 1.0] range based on the angle between
    /// the surface normal and the light direction.
    pub fn intensity(&self, normal: Vec3) -> f32 {
        // Negate direction: light pointing at surface = positive dot product
        (-self.direction).dot(normal.normalize()).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_illumination() {
        // Light pointing toward -Z, normal facing +Z (toward the light)
        let light = DirectionalLight::new(Vec3::new(0.0, 0.0, -1.0));
        let normal = Vec3::new(0.0, 0.0, 1.0);
        assert!((light.intensity(normal) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_no_illumination() {
        // Light pointing toward -Z, normal facing -Z (away from light)
        let light = DirectionalLight::new(Vec3::new(0.0, 0.0, -1.0));
        let normal = Vec3::new(0.0, 0.0, -1.0);
        assert!(light.intensity(normal) == 0.0);
    }

    #[test]
    fn test_angled_illumination() {
        // Light pointing straight down (-Y), normal at 45 degrees
        let light = DirectionalLight::new(Vec3::new(0.0, -1.0, 0.0));
        let normal = Vec3::new(0.0, 1.0, 1.0).normalize();
        // cos(45) ≈ 0.707
        let intensity = light.intensity(normal);
        assert!((intensity - 0.707).abs() < 0.01);
    }
}
