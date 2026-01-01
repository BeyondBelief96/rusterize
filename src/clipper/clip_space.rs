//! Clip-space clipping against the homogeneous clip cube.
//!
//! Clipping occurs after projection (in homogeneous clip space), before the
//! perspective divide. The clip volume is defined by:
//!
//! ```text
//! -w <= x <= w
//! -w <= y <= w
//! -w <= z <= w   (for [-1, 1] depth range, OpenGL-style)
//! ```
//!
//! This approach is simpler than view-space clipping because:
//! - The planes are fixed (no FOV-dependent angles)
//! - No need to rebuild when projection parameters change
//! - This is how GPU hardware performs clipping

use crate::colors;
use crate::prelude::{Vec2, Vec4};

/// A vertex in homogeneous clip space with interpolatable attributes.
#[derive(Clone, Copy)]
pub struct ClipSpaceVertex {
    /// Position in clip space (x, y, z, w) - before perspective divide
    pub position: Vec4,
    /// Texture coordinates
    pub texcoord: Vec2,
    /// Packed ARGB color
    pub color: u32,
}

impl ClipSpaceVertex {
    pub fn new(position: Vec4, texcoord: Vec2, color: u32) -> Self {
        Self {
            position,
            texcoord,
            color,
        }
    }

    /// Linearly interpolate all attributes between two vertices.
    /// Used when a polygon edge crosses a clipping plane.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            texcoord: self.texcoord + (other.texcoord - self.texcoord) * t,
            color: {
                let c1 = colors::unpack_color(self.color);
                let c2 = colors::unpack_color(other.color);
                let (r, g, b) = colors::lerp_color(c1, c2, t);
                colors::pack_color(r, g, b, 1.0)
            },
        }
    }
}

/// The 6 planes of the canonical clip-space cube.
///
/// Each plane is defined implicitly by a linear inequality on (x, y, z, w).
/// The signed distance is positive when inside the clip volume.
#[derive(Clone, Copy, Debug)]
pub enum ClipPlane {
    /// Left plane: x >= -w
    Left,
    /// Right plane: x <= w
    Right,
    /// Bottom plane: y >= -w
    Bottom,
    /// Top plane: y <= w
    Top,
    /// Near plane: z >= -w (for [-1, 1] depth range, OpenGL-style)
    Near,
    /// Far plane: z <= w
    Far,
}

impl ClipPlane {
    /// Returns the signed distance from a vertex to this plane.
    /// Positive = inside the clip volume, Negative = outside.
    pub fn signed_distance(&self, v: &ClipSpaceVertex) -> f32 {
        let p = v.position;
        match self {
            Self::Left => p.w + p.x,   // x >= -w  =>  w + x >= 0
            Self::Right => p.w - p.x,  // x <= w   =>  w - x >= 0
            Self::Bottom => p.w + p.y, // y >= -w  =>  w + y >= 0
            Self::Top => p.w - p.y,    // y <= w   =>  w - y >= 0
            Self::Near => p.w + p.z,   // z >= -w  =>  w + z >= 0
            Self::Far => p.w - p.z,    // z <= w   =>  w - z >= 0
        }
    }
}

/// A polygon in clip space, represented as a list of vertices.
///
/// Used as an intermediate representation during clipping. After clipping
/// against all planes, this is triangulated back into triangles for
/// rasterization.
pub struct ClipSpacePolygon {
    pub vertices: Vec<ClipSpaceVertex>,
}

impl ClipSpacePolygon {
    /// Create a polygon from a triangle (3 vertices).
    pub fn from_triangle(v0: ClipSpaceVertex, v1: ClipSpaceVertex, v2: ClipSpaceVertex) -> Self {
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
    pub fn clip_against_plane(&self, plane: ClipPlane) -> Self {
        if self.vertices.len() < 3 {
            return Self { vertices: vec![] };
        }

        let mut output = Vec::new();

        for i in 0..self.vertices.len() {
            let current = &self.vertices[i];
            let next = &self.vertices[(i + 1) % self.vertices.len()];

            let d1 = plane.signed_distance(current);
            let d2 = plane.signed_distance(next);

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
    ///
    /// Returns an iterator of (v0, v1, v2) triangles.
    /// Assumes the polygon is convex (which is guaranteed after clipping).
    pub fn triangulate(
        &self,
    ) -> impl Iterator<Item = (&ClipSpaceVertex, &ClipSpaceVertex, &ClipSpaceVertex)> {
        (1..self.vertices.len().saturating_sub(1))
            .map(move |i| (&self.vertices[0], &self.vertices[i], &self.vertices[i + 1]))
    }
}

/// Clips polygons against the canonical clip-space cube.
///
/// The clip cube is defined by: -w <= x,y <= w and 0 <= z <= w.
/// This clipper is stateless and doesn't need to be rebuilt when
/// projection parameters change.
pub struct ClipSpaceClipper {
    planes: [ClipPlane; 6],
}

impl ClipSpaceClipper {
    /// Creates a new clip-space clipper.
    ///
    /// The clipper uses the standard 6 planes of the clip cube.
    pub fn new() -> Self {
        Self {
            planes: [
                ClipPlane::Left,
                ClipPlane::Right,
                ClipPlane::Bottom,
                ClipPlane::Top,
                ClipPlane::Near,
                ClipPlane::Far,
            ],
        }
    }

    /// Clip a polygon against all 6 planes of the clip cube.
    ///
    /// Returns the clipped polygon, which may be empty if the original
    /// polygon was entirely outside the clip volume.
    pub fn clip_polygon(&self, polygon: ClipSpacePolygon) -> ClipSpacePolygon {
        let mut result = polygon;

        for &plane in &self.planes {
            if result.is_empty() {
                break;
            }
            result = result.clip_against_plane(plane);
        }

        result
    }
}

impl Default for ClipSpaceClipper {
    fn default() -> Self {
        Self::new()
    }
}
