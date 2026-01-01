//! Perspective projection parameters.
//!
//! The [`Projection`] struct is the single source of truth for all perspective
//! projection parameters (FOV, aspect ratio, near/far planes). It can generate
//! the projection matrix and view-space frustum planes for clipping.

use crate::clipping::ViewFrustum;
use crate::math::mat4::Mat4;

/// Perspective projection parameters.
///
/// Stores the canonical projection parameters and provides methods to derive
/// the projection matrix and view-space frustum for clipping.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    /// Vertical field of view in radians.
    fov_y: f32,
    /// Aspect ratio (width / height).
    aspect_ratio: f32,
    /// Near clipping plane distance.
    z_near: f32,
    /// Far clipping plane distance.
    z_far: f32,
}

impl Projection {
    /// Creates a new projection with the given parameters.
    ///
    /// # Arguments
    /// * `fov_y` - Vertical field of view in radians
    /// * `aspect_ratio` - Width divided by height
    /// * `z_near` - Near clipping plane distance (must be > 0)
    /// * `z_far` - Far clipping plane distance (must be > z_near)
    pub fn new(fov_y: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        Self {
            fov_y,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    /// Creates a projection from degrees instead of radians.
    pub fn from_degrees(fov_y_degrees: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        Self::new(fov_y_degrees.to_radians(), aspect_ratio, z_near, z_far)
    }

    /// Returns the vertical field of view in radians.
    pub fn fov_y(&self) -> f32 {
        self.fov_y
    }

    /// Returns the horizontal field of view in radians.
    ///
    /// Computed from the vertical FOV and aspect ratio.
    pub fn fov_x(&self) -> f32 {
        2.0 * (self.aspect_ratio * (self.fov_y / 2.0).tan()).atan()
    }

    /// Returns the aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    /// Returns the near clipping plane distance.
    pub fn z_near(&self) -> f32 {
        self.z_near
    }

    /// Returns the far clipping plane distance.
    pub fn z_far(&self) -> f32 {
        self.z_far
    }

    /// Updates the aspect ratio (typically called on window resize).
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Generates the left-handed perspective projection matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov_y, self.aspect_ratio, self.z_near, self.z_far)
    }

    /// Builds view-space frustum planes for clipping.
    ///
    /// The frustum planes are positioned in view/camera space and can be used
    /// to clip geometry before projection.
    pub fn view_frustum(&self) -> ViewFrustum {
        ViewFrustum::new(self.fov_x(), self.fov_y, self.z_near, self.z_far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::FRAC_PI_4;

    #[test]
    fn fov_x_matches_aspect_ratio() {
        // With aspect ratio 1:1, fov_x should equal fov_y
        let proj = Projection::new(FRAC_PI_4, 1.0, 0.1, 100.0);
        assert_relative_eq!(proj.fov_x(), proj.fov_y(), epsilon = 1e-6);
    }

    #[test]
    fn fov_x_wider_with_higher_aspect() {
        // With wider aspect ratio, fov_x should be larger than fov_y
        let proj = Projection::new(FRAC_PI_4, 16.0 / 9.0, 0.1, 100.0);
        assert!(proj.fov_x() > proj.fov_y());
    }

    #[test]
    fn from_degrees_converts_correctly() {
        let proj = Projection::from_degrees(45.0, 1.0, 0.1, 100.0);
        assert_relative_eq!(proj.fov_y(), FRAC_PI_4, epsilon = 1e-6);
    }
}
