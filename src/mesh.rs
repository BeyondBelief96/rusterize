use crate::math::vec3::Vec3;
use crate::triangle::Face;

pub const N_CUBE_VERTICES: usize = 8;
pub const N_CUBE_FACES: usize = 12;

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The vertices of the mesh
    pub vertices: Vec<Vec3>,
    /// The faces of the mesh
    pub faces: Vec<Face>,
    /// The rotation about the x, y, and z axes for the mesh
    pub rotation: Vec3,
}

impl Mesh {
    pub fn new(vertices: Vec<Vec3>, faces: Vec<Face>, rotation: Vec3) -> Self {
        Self { vertices, faces, rotation }
    }

    /// Get a reference to the rotation vector
    pub fn rotation(&self) -> &Vec3 {
        &self.rotation
    }

    /// Get a mutable reference to the rotation vector
    pub fn rotation_mut(&mut self) -> &mut Vec3 {
        &mut self.rotation
    }

    /// Get a reference to the vertices
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// Get a reference to the faces
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
}

pub const CUBE_VERTICES: [Vec3; N_CUBE_VERTICES] = [
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new(-1.0, 1.0, -1.0),
    Vec3::new(1.0, 1.0, -1.0),
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(-1.0, -1.0, 1.0),
];

pub const CUBE_FACES: [Face; N_CUBE_FACES] = [
    // Front face
    Face { a: 1, b: 2, c: 3},
    Face { a: 1, b: 3, c: 4},
    // Right face
    Face { a: 4, b: 3, c: 5},
    Face { a: 4, b: 5, c: 6},
    // Back face
    Face { a: 6, b: 5, c: 7},
    Face { a: 6, b: 7, c: 8},
    // Left face
    Face { a: 8, b: 7, c: 2},
    Face { a: 8, b: 2, c: 1},
    // Top face
    Face { a: 2, b: 7, c: 5 },
    Face { a: 2, b: 5, c: 3 },
    // Bottom face
    Face { a: 6, b: 8, c: 1 },
    Face { a: 6, b: 1, c: 4 },
];