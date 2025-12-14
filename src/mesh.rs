use crate::math::vec3::Vec3;
use crate::triangle::Face;

pub const N_NUM_VERTICES: usize = 8;
pub const N_NUM_FACES: usize = 12;

// OWNERSHIP: Static/constant data - owned by the program itself ('static lifetime)
// This data is compiled into the binary and can be borrowed by anyone, anytime
// No single owner - it's shared across the entire program lifetime
pub const MESH_VERTICES: [Vec3; N_NUM_VERTICES] = [
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new(-1.0, 1.0, -1.0),
    Vec3::new(1.0, 1.0, -1.0),
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(-1.0, -1.0, 1.0),
];

// OWNERSHIP: Same as MESH_VERTICES - static data, 'static lifetime
pub const MESH_FACES: [Face; N_NUM_FACES] = [
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