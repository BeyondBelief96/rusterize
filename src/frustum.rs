//! Frustum culling primitives.
//!
//! A [`Frustum`] is 6 planes derived from a projection or view-projection
//! matrix via the Gribb-Hartmann technique. The space of the planes follows
//! the input matrix — feed `projection` for view-space planes, `projection *
//! view` for world-space.
//!
//! Three cull tests are provided, ordered by cost:
//!
//! - [`Frustum::contains_sphere_cached`] — cheapest, plane-coherency cache
//!   makes off-screen rejection ~1 plane test.
//! - [`Frustum::classify_sphere`] — three-state in/out/intersecting, used for
//!   hierarchical culling (model-level early-out).
//! - [`Frustum::aabb_outside`] — tighter secondary test, layered after the
//!   sphere when bounds are loose on elongated meshes.

use std::cell::Cell;

use crate::math::mat4::Mat4;
use crate::math::plane::Plane;
use crate::math::vec3::Vec3;
use crate::mesh::CullCache;

/// Three-state result of a sphere-vs-frustum classifying.
/// Used for hierarchical culling where a fully inside parent lets children
/// skip their own frustum tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrustumTest {
    /// Fully outside the frustum — can be culled entirely.
    Outside,
    /// Fully inside the frustum — descendants are guaranteed visible.
    FullyInside,
    /// Straddles at least one plane — descendants must be tested individually.
    Intersecting,
}

/// View/world/model-space frustum defined by 6 half-space planes.
///
/// Inward-pointing normals: a point is inside the frustum when
/// `signed_distance(p) >= 0` for every plane.
pub(crate) struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    /// Extract the 6 frustum planes from a transformation matrix using the
    /// Gribb-Hartmann technique.
    ///
    /// The output plane space follows the input matrix:
    ///   - `projection`            → view-space planes
    ///   - `projection * view`     → world-space planes
    ///   - `projection * view * M` → model-space planes (rebuild per object)
    ///
    /// Assumes clip-space z ∈ [-1, 1] (OpenGL / left-handed `perspective_lh`
    /// convention used in this codebase). For DX-style [0, 1] z, the near
    /// plane would be just `row2` instead of `row3 + row2`.
    pub fn from_matrix(m: &Mat4) -> Self {
        // Rows of m. Row i dotted with (p, 1) is the i-th clip coordinate.
        let (r00, r01, r02, r03) = (m.get(0, 0), m.get(0, 1), m.get(0, 2), m.get(0, 3));
        let (r10, r11, r12, r13) = (m.get(1, 0), m.get(1, 1), m.get(1, 2), m.get(1, 3));
        let (r20, r21, r22, r23) = (m.get(2, 0), m.get(2, 1), m.get(2, 2), m.get(2, 3));
        let (r30, r31, r32, r33) = (m.get(3, 0), m.get(3, 1), m.get(3, 2), m.get(3, 3));

        Self {
            planes: [
                // Left:   row3 + row0   ⇔   x_clip + w_clip ≥ 0
                Plane::from_equation(r30 + r00, r31 + r01, r32 + r02, r33 + r03),
                // Right:  row3 - row0   ⇔   w_clip - x_clip ≥ 0
                Plane::from_equation(r30 - r00, r31 - r01, r32 - r02, r33 - r03),
                // Top:    row3 - row1   ⇔   w_clip - y_clip ≥ 0
                Plane::from_equation(r30 - r10, r31 - r11, r32 - r12, r33 - r13),
                // Bottom: row3 + row1   ⇔   y_clip + w_clip ≥ 0
                Plane::from_equation(r30 + r10, r31 + r11, r32 + r12, r33 + r13),
                // Near:   row3 + row2   ⇔   z_clip + w_clip ≥ 0   (LH [-1,1] z)
                Plane::from_equation(r30 + r20, r31 + r21, r32 + r22, r33 + r23),
                // Far:    row3 - row2   ⇔   w_clip - z_clip ≥ 0
                Plane::from_equation(r30 - r20, r31 - r21, r32 - r22, r33 - r23),
            ],
        }
    }

    /// Basic sphere-vs-frustum test. No cache — every call pays up to 6 plane
    /// tests. Prefer `contains_sphere_cached` in hot loops; this variant is
    /// useful for tests and benchmarks that want the simpler path.
    #[allow(dead_code)]
    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.signed_distance(center) < -radius {
                return false;
            }
        }
        true
    }

    /// Like `contains_sphere`, but tests the plane that rejected this object
    /// on the previous frame first. Over successive frames, off-screen objects
    /// tend to be rejected by the same plane, so this drops the expected
    /// test count from 6 to ~1 for outside meshes.
    ///
    /// Meshes fully inside still pay the full 6 tests; the cache is cleared
    /// for them, so a stale index can't mask a later rejection.
    pub(crate) fn contains_sphere_cached(
        &self,
        center: Vec3,
        radius: f32,
        cache: &Cell<CullCache>,
    ) -> bool {
        let cached = cache.get();

        // Fast path: the last rejecting plane still rejects → one test, done.
        if let Some(idx) = cached.last_rejecting_plane {
            if self.planes[idx as usize].signed_distance(center) < -radius {
                return false;
            }
        }

        for (i, plane) in self.planes.iter().enumerate() {
            if Some(i as i8) == cached.last_rejecting_plane {
                continue;
            }
            if plane.signed_distance(center) < -radius {
                cache.set(CullCache {
                    last_rejecting_plane: Some(i as i8),
                });
                return false;
            }
        }

        // Fully inside — clear so a stale index can't mask a future rejection.
        cache.set(CullCache {
            last_rejecting_plane: None,
        });
        true
    }

    /// Three-state classify used for hierarchical culling. A `FullyInside`
    /// parent lets children skip their own frustum tests entirely.
    pub(crate) fn classify_sphere(&self, center: Vec3, radius: f32) -> FrustumTest {
        let mut fully_inside_all = true;
        for plane in &self.planes {
            let d = plane.signed_distance(center);
            if d < -radius {
                return FrustumTest::Outside;
            }
            if d < radius {
                // Sphere straddles this plane → at best Intersecting.
                fully_inside_all = false;
            }
        }
        if fully_inside_all {
            FrustumTest::FullyInside
        } else {
            FrustumTest::Intersecting
        }
    }

    /// Returns true if the axis-aligned box is fully outside the frustum.
    /// Uses the n/p-vertex trick: for each plane, pick the box corner farthest
    /// along the plane's inward normal; if that corner is outside, the whole
    /// box is outside.
    ///
    /// Intended as a tighter secondary test *after* the sphere test — spheres
    /// are loose on elongated meshes; this closes the gap.
    pub(crate) fn aabb_outside(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.normal.x >= 0.0 { max.x } else { min.x },
                if plane.normal.y >= 0.0 { max.y } else { min.y },
                if plane.normal.z >= 0.0 { max.z } else { min.z },
            );
            if plane.signed_distance(p) < 0.0 {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    /// `from_matrix(projection)` should produce view-space planes where a
    /// point dead-center in the frustum is inside and points behind the
    /// camera / past the far plane are outside.
    #[test]
    fn from_matrix_produces_valid_frustum() {
        let proj = Mat4::perspective_lh(FRAC_PI_4, 16.0 / 9.0, 0.1, 100.0);
        let frustum = Frustum::from_matrix(&proj);

        // Dead center, mid-frustum → inside.
        assert!(frustum.contains_sphere(Vec3::new(0.0, 0.0, 50.0), 0.0));

        // Past the far plane → outside.
        assert!(!frustum.contains_sphere(Vec3::new(0.0, 0.0, 1000.0), 0.0));

        // Behind the camera → outside (near plane rejects).
        assert!(!frustum.contains_sphere(Vec3::new(0.0, 0.0, -10.0), 0.0));

        // Way off to the side → outside.
        assert!(!frustum.contains_sphere(Vec3::new(1000.0, 0.0, 10.0), 0.0));
    }

    #[test]
    fn classify_returns_three_states() {
        let frustum =
            Frustum::from_matrix(&Mat4::perspective_lh(FRAC_PI_4, 1.0, 0.1, 100.0));

        // Small sphere in the middle → fully inside.
        assert_eq!(
            frustum.classify_sphere(Vec3::new(0.0, 0.0, 50.0), 1.0),
            FrustumTest::FullyInside,
        );

        // Huge sphere encompassing the whole frustum → intersecting.
        assert_eq!(
            frustum.classify_sphere(Vec3::new(0.0, 0.0, 50.0), 500.0),
            FrustumTest::Intersecting,
        );

        // Sphere far behind the camera → outside.
        assert_eq!(
            frustum.classify_sphere(Vec3::new(0.0, 0.0, -1000.0), 1.0),
            FrustumTest::Outside,
        );
    }
}
