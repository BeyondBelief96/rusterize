//! Geometric plane primitive.
//!
//! Defined by a point on the plane and an inward-pointing normal. Used by the
//! frustum culler and any future geometry that needs half-space tests.

use super::vec3::Vec3;

/// A plane defined by a point on the plane and its normal vector.
/// The normal points toward the "inside" (visible) half-space.
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
}

impl Plane {
    pub fn new(point: Vec3, normal: Vec3) -> Self {
        Self { point, normal }
    }

    /// Build a plane from its general equation `a*x + b*y + c*z + d = 0`,
    /// where `(a, b, c)` points toward the "inside" half-space. The equation
    /// is normalized so that `signed_distance` returns a true Euclidean
    /// signed distance (not a scaled value).
    pub fn from_equation(a: f32, b: f32, c: f32, d: f32) -> Self {
        let len = (a * a + b * b + c * c).sqrt();
        let nx = a / len;
        let ny = b / len;
        let nz = c / len;
        let d_norm = d / len;
        // Any point satisfying n·p + d = 0 lies on the plane. Picking p = -d·n
        // gives a canonical representative: the foot of the perpendicular from
        // the origin.
        Self {
            point: Vec3::new(-d_norm * nx, -d_norm * ny, -d_norm * nz),
            normal: Vec3::new(nx, ny, nz),
        }
    }

    /// Returns the signed distance from a point to this plane.
    /// Positive = inside (same side as normal), Negative = outside.
    pub fn signed_distance(&self, position: Vec3) -> f32 {
        (position - self.point).dot(self.normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_equation_normalizes() {
        // Plane: x >= 5. Equation (1, 0, 0, -5) scaled by 100 to verify normalization.
        let plane = Plane::from_equation(100.0, 0.0, 0.0, -500.0);
        let d = plane.signed_distance(Vec3::new(10.0, 0.0, 0.0));
        assert!((d - 5.0).abs() < 1e-5, "got {}", d);
    }

    #[test]
    fn signed_distance_is_symmetric() {
        // Point on the "outside" side should give the negative of the matching inside point.
        let plane = Plane::from_equation(0.0, 1.0, 0.0, 0.0); // y >= 0
        assert!((plane.signed_distance(Vec3::new(0.0, 3.0, 0.0)) - 3.0).abs() < 1e-6);
        assert!((plane.signed_distance(Vec3::new(0.0, -3.0, 0.0)) + 3.0).abs() < 1e-6);
    }
}
