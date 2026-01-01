//! View-space frustum clipping.
//!
//! This module provides clipping against the view-space frustum using the
//! Sutherland-Hodgman algorithm. Planes are defined by point + normal pairs.
//!
//! Note: This implementation is kept for reference. The engine now uses
//! clip-space clipping (see `clip_space` module) which is simpler and
//! doesn't require rebuilding planes when projection parameters change.

#![allow(dead_code)]

use crate::colors;
use crate::prelude::{Vec2, Vec3};

/// A plane defined by a point on the plane and its normal vector.
/// The normal points toward the "inside" (visible) half-space.
#[derive(Clone, Copy)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
}

impl Plane {
    pub fn new(point: Vec3, normal: Vec3) -> Self {
        Self { point, normal }
    }

    /// Returns the signed distance from a point to this plane.
    /// Positive = inside (same side as normal), Negative = outside.
    pub fn signed_distance(&self, position: Vec3) -> f32 {
        (position - self.point).dot(self.normal)
    }
}

/// A vertex with all attributes needed for clipping interpolation.
/// This is an intermediate representation used during the clipping process.
#[derive(Clone, Copy)]
pub struct ClipVertex {
    pub position: Vec3,
    pub texcoord: Vec2,
    pub color: u32,
}

impl ClipVertex {
    pub fn new(position: Vec3, texcoord: Vec2, color: u32) -> Self {
        Self {
            position,
            texcoord,
            color,
        }
    }

    /// Linearly interpolate between two vertices.
    /// Used when a polygon edge crosses a clipping plane.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let position = self.position + (other.position - self.position) * t;
        let texcoord = self.texcoord + (other.texcoord - self.texcoord) * t;

        // Interpolate color components
        let c1 = colors::unpack_color(self.color);
        let c2 = colors::unpack_color(other.color);
        let (r, g, b) = colors::lerp_color(c1, c2, t);
        let color = colors::pack_color(r, g, b, 1.0);

        Self {
            position,
            texcoord,
            color,
        }
    }
}

/// A polygon represented as a list of vertices.
/// Used as an intermediate representation during clipping.
/// After clipping against all planes, this is triangulated back
/// into triangles for rasterization.
pub struct ClipPolygon {
    pub vertices: Vec<ClipVertex>,
}

impl ClipPolygon {
    /// Create a polygon from a triangle (3 vertices).
    pub fn from_triangle(v0: ClipVertex, v1: ClipVertex, v2: ClipVertex) -> Self {
        Self {
            vertices: vec![v0, v1, v2],
        }
    }

    /// Returns true if the polygon has been completely clipped away.
    pub fn is_empty(&self) -> bool {
        self.vertices.len() < 3
    }

    /// Clip this polygon against a single plane using the Sutherland-Hodgman algorithm.
    /// Returns a new polygon with the clipped vertices.
    pub fn clip_against_plane(&self, plane: &Plane) -> Self {
        if self.vertices.len() < 3 {
            return Self { vertices: vec![] };
        }

        let mut output = Vec::new();

        for i in 0..self.vertices.len() {
            let current = &self.vertices[i];
            let next = &self.vertices[(i + 1) % self.vertices.len()];

            let d1 = plane.signed_distance(current.position);
            let d2 = plane.signed_distance(next.position);

            let current_inside = d1 >= 0.0;
            let next_inside = d2 >= 0.0;

            if current_inside {
                // Current vertex is inside, add it
                output.push(*current);

                if !next_inside {
                    // Going from inside to outside, add intersection
                    let t = d1 / (d1 - d2);
                    output.push(current.lerp(next, t));
                }
            } else if next_inside {
                // Going from outside to inside, add intersection
                let t = d1 / (d1 - d2);
                output.push(current.lerp(next, t));
            }
            // If both outside, add nothing
        }

        Self { vertices: output }
    }

    /// Triangulate this convex polygon using fan triangulation.
    /// Returns an iterator of (v0, v1, v2) triangles.
    /// Assumes the polygon is convex (which is guaranteed after clipping).
    pub fn triangulate(&self) -> impl Iterator<Item = (&ClipVertex, &ClipVertex, &ClipVertex)> {
        (1..self.vertices.len().saturating_sub(1))
            .map(move |i| (&self.vertices[0], &self.vertices[i], &self.vertices[i + 1]))
    }
}

/// View-space frustum defined by 6 clipping planes.
///
/// The planes are constructed from the projection parameters (FOV, near/far)
/// and positioned in view/camera space. Use this to clip geometry before
/// projection to avoid issues with vertices behind the camera.
pub struct ViewFrustum {
    planes: [Plane; 6],
}

impl ViewFrustum {
    /// Creates a new view frustum from projection parameters.
    ///
    /// # Arguments
    /// * `fov_x` - Horizontal field of view in radians
    /// * `fov_y` - Vertical field of view in radians
    /// * `z_near` - Near clipping plane distance
    /// * `z_far` - Far clipping plane distance
    pub fn new(fov_x: f32, fov_y: f32, z_near: f32, z_far: f32) -> Self {
        let half_fov_x = fov_x / 2.0;
        let half_fov_y = fov_y / 2.0;
        let origin = Vec3::ZERO;

        Self {
            planes: [
                // Left plane: normal points right-ish, into the frustum
                Plane::new(
                    origin,
                    Vec3::new(half_fov_x.cos(), 0.0, half_fov_x.sin()),
                ),
                // Right plane: normal points left-ish, into the frustum
                Plane::new(
                    origin,
                    Vec3::new(-half_fov_x.cos(), 0.0, half_fov_x.sin()),
                ),
                // Top plane: normal points down-ish, into the frustum
                Plane::new(
                    origin,
                    Vec3::new(0.0, -half_fov_y.cos(), half_fov_y.sin()),
                ),
                // Bottom plane: normal points up-ish, into the frustum
                Plane::new(
                    origin,
                    Vec3::new(0.0, half_fov_y.cos(), half_fov_y.sin()),
                ),
                // Near plane: normal points forward (+Z)
                Plane::new(Vec3::new(0.0, 0.0, z_near), Vec3::new(0.0, 0.0, 1.0)),
                // Far plane: normal points backward (-Z)
                Plane::new(Vec3::new(0.0, 0.0, z_far), Vec3::new(0.0, 0.0, -1.0)),
            ],
        }
    }

    /// Clip a polygon against all frustum planes.
    /// Returns the clipped polygon, which may be empty if fully outside.
    pub fn clip_polygon(&self, polygon: ClipPolygon) -> ClipPolygon {
        let mut result = polygon;

        for plane in &self.planes {
            if result.is_empty() {
                break;
            }
            result = result.clip_against_plane(plane);
        }

        result
    }
}
