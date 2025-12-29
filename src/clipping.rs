use crate::colors;
use crate::prelude::{Vec2, Vec3};

type Point = Vec3;
type Normal = Vec3;

pub type Plane = (Point, Normal);

/// A vertex with all attributes needed for clipping interpolation.
/// This is an intermediate representation used during the clipping process.
#[derive(Clone, Copy)]
pub(crate) struct ClipVertex {
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

    /// Returns the signed distance from this vertex to a plane.
    /// Positive = inside (same side as normal), Negative = outside.
    fn signed_distance(&self, plane: Plane) -> f32 {
        let (plane_point, plane_normal) = plane;
        (self.position - plane_point).dot(plane_normal)
    }
}

/// A polygon represented as a list of vertices.
/// Used as an intermediate representation during clipping.
/// After clipping against all frustum planes, this is triangulated back
/// into triangles for rasterization.
pub(crate) struct ClipPolygon {
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
    pub fn clip_against_plane(&self, plane: Plane) -> Self {
        if self.vertices.is_empty() {
            return Self { vertices: vec![] };
        }

        let mut output = Vec::new();

        for i in 0..self.vertices.len() {
            let current = &self.vertices[i];
            let next = &self.vertices[(i + 1) % self.vertices.len()];

            let d1 = current.signed_distance(plane);
            let d2 = next.signed_distance(plane);

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

pub enum ClippingPlane {
    Left(Plane),
    Right(Plane),
    Top(Plane),
    Bottom(Plane),
    Near(Plane),
    Far(Plane),
}

impl ClippingPlane {
    /// Extract the plane (point, normal) from this clipping plane.
    pub fn plane(&self) -> Plane {
        match self {
            ClippingPlane::Left(p)
            | ClippingPlane::Right(p)
            | ClippingPlane::Top(p)
            | ClippingPlane::Bottom(p)
            | ClippingPlane::Near(p)
            | ClippingPlane::Far(p) => *p,
        }
    }

    fn new_frustum_left(fov: f32) -> Self {
        let half_fov = fov / 2.0;
        let normal = Vec3::new(half_fov.cos(), 0.0, half_fov.sin());
        ClippingPlane::Left((Vec3::new(0.0, 0.0, 0.0), normal))
    }

    fn new_frustum_right(fov: f32) -> Self {
        let half_fov = fov / 2.0;
        let normal = Vec3::new(-half_fov.cos(), 0.0, half_fov.sin());
        ClippingPlane::Right((Vec3::new(0.0, 0.0, 0.0), normal))
    }

    fn new_frustum_top(fov: f32) -> Self {
        let half_fov = fov / 2.0;
        let normal = Vec3::new(0.0, -half_fov.cos(), half_fov.sin());
        ClippingPlane::Top((Vec3::new(0.0, 0.0, 0.0), normal))
    }

    fn new_frustum_bottom(fov: f32) -> Self {
        let half_fov = fov / 2.0;
        let normal = Vec3::new(0.0, half_fov.cos(), half_fov.sin());
        ClippingPlane::Bottom((Vec3::new(0.0, 0.0, 0.0), normal))
    }

    fn new_frustum_near(znear: f32) -> Self {
        let point = Vec3::new(0.0, 0.0, znear);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        ClippingPlane::Near((point, normal))
    }

    fn new_frustum_far(zfar: f32) -> Self {
        let point = Vec3::new(0.0, 0.0, zfar);
        let normal = Vec3::new(0.0, 0.0, -1.0);
        ClippingPlane::Far((point, normal))
    }
}

pub struct Frustum {
    pub planes: [ClippingPlane; 6],
}

impl Frustum {
    pub fn new(fov: f32, aspect: f32, znear: f32, zfar: f32) -> Self {
        // Horizontal FOV derived from vertical FOV and aspect ratio
        // tan(fov_x / 2) = aspect * tan(fov_y / 2)
        let fov_x = 2.0 * (aspect * (fov / 2.0).tan()).atan();

        Self {
            planes: [
                ClippingPlane::new_frustum_left(fov_x),
                ClippingPlane::new_frustum_right(fov_x),
                ClippingPlane::new_frustum_top(fov),
                ClippingPlane::new_frustum_bottom(fov),
                ClippingPlane::new_frustum_near(znear),
                ClippingPlane::new_frustum_far(zfar),
            ],
        }
    }

    /// Clip a polygon against all frustum planes.
    /// Returns the clipped polygon, which may be empty if fully outside.
    pub(crate) fn clip_polygon(&self, polygon: ClipPolygon) -> ClipPolygon {
        let mut result = polygon;

        for clipping_plane in &self.planes {
            if result.is_empty() {
                break;
            }
            result = result.clip_against_plane(clipping_plane.plane());
        }

        result
    }
}
