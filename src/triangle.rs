use crate::math::vec2::Vec2;

// This struct represents a triangle defined by three vertices
// The members a, b, and c are indices into the vertex array
// of the mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

// This struct represents a triangle defined by three points
// The points are 2D coordinates in screen space
#[derive(Clone, Debug, PartialEq)]
pub struct Triangle {
    pub points: Vec<Vec2>,
    pub color: u32,
}