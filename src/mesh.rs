//! 3D mesh representation and loading.
//!
//! Provides the [`Mesh`] struct for storing vertices and faces, along with
//! OBJ file loading support via the `tobj` crate.

use std::fmt;

use crate::math::vec3::Vec3;

pub(crate) const N_CUBE_VERTICES: usize = 8;
pub(crate) const N_CUBE_FACES: usize = 12;

/// Represents a triangle face defined by three vertex indices.
/// The indices are 1-based into the mesh's vertex array.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Face {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Face {
    pub const fn new(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Tobj(tobj::LoadError),
    NoModels,
    NoVertices,
    InvalidFaces,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Tobj(e) => write!(f, "failed to load OBJ: {}", e),
            LoadError::NoModels => write!(f, "OBJ file contains no models"),
            LoadError::NoVertices => write!(f, "mesh has no vertices"),
            LoadError::InvalidFaces => write!(f, "face indices not divisible by 3"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Tobj(e) => Some(e),
            _ => None,
        }
    }
}

impl From<tobj::LoadError> for LoadError {
    fn from(e: tobj::LoadError) -> Self {
        LoadError::Tobj(e)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
    rotation: Vec3,
    scale: Vec3,
    translation: Vec3,
}

impl Mesh {
    pub(crate) fn new(
        vertices: Vec<Vec3>,
        faces: Vec<Face>,
        rotation: Vec3,
        scale: Vec3,
        translation: Vec3,
    ) -> Self {
        Self {
            vertices,
            faces,
            rotation,
            scale,
            translation,
        }
    }

    pub(crate) fn from_obj(file_path: &str) -> Result<Self, LoadError> {
        let (models, _materials) = tobj::load_obj(file_path, &tobj::GPU_LOAD_OPTIONS)?;

        // For now we only support a single model
        let model = models.into_iter().next().ok_or(LoadError::NoModels)?;

        // For now we assume a single mesh per model.
        let mesh = model.mesh;

        if mesh.positions.is_empty() {
            return Err(LoadError::NoVertices);
        }

        if mesh.indices.len() % 3 != 0 {
            return Err(LoadError::InvalidFaces);
        }

        // Convert flat [x, y, z, x, y, z, ...] to Vec3
        let vertices: Vec<Vec3> = mesh
            .positions
            .chunks_exact(3)
            .map(|c| Vec3::new(c[0], c[1], c[2]))
            .collect();

        // Convert flat indices to Face (tobj is 0-based, add 1 for 1-based convention)
        let faces: Vec<Face> = mesh
            .indices
            .chunks_exact(3)
            .map(|c| Face::new(c[0] + 1, c[1] + 1, c[2] + 1))
            .collect();

        Ok(Self::new(
            vertices,
            faces,
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::ZERO,
        ))
    }

    /// Get the rotation vector
    pub fn rotation(&self) -> Vec3 {
        self.rotation
    }

    /// Get a mutable reference to the rotation vector
    pub fn rotation_mut(&mut self) -> &mut Vec3 {
        &mut self.rotation
    }

    /// Get the scale vector
    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    /// Get a mutable reference to the scale vector
    pub fn scale_mut(&mut self) -> &mut Vec3 {
        &mut self.scale
    }

    /// Get the translation vector
    pub fn translation(&self) -> Vec3 {
        self.translation
    }

    /// Get a mutable reference to the translation vector
    pub fn translation_mut(&mut self) -> &mut Vec3 {
        &mut self.translation
    }

    /// Get a reference to the vertices
    pub(crate) fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// Get a reference to the faces
    pub(crate) fn faces(&self) -> &[Face] {
        &self.faces
    }
}

pub(crate) const CUBE_VERTICES: [Vec3; N_CUBE_VERTICES] = [
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new(-1.0, 1.0, -1.0),
    Vec3::new(1.0, 1.0, -1.0),
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(-1.0, -1.0, 1.0),
];

pub(crate) const CUBE_FACES: [Face; N_CUBE_FACES] = [
    // Front face
    Face { a: 1, b: 2, c: 3 },
    Face { a: 1, b: 3, c: 4 },
    // Right face
    Face { a: 4, b: 3, c: 5 },
    Face { a: 4, b: 5, c: 6 },
    // Back face
    Face { a: 6, b: 5, c: 7 },
    Face { a: 6, b: 7, c: 8 },
    // Left face
    Face { a: 8, b: 7, c: 2 },
    Face { a: 8, b: 2, c: 1 },
    // Top face
    Face { a: 2, b: 7, c: 5 },
    Face { a: 2, b: 5, c: 3 },
    // Bottom face
    Face { a: 6, b: 8, c: 1 },
    Face { a: 6, b: 1, c: 4 },
];
