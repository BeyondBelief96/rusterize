//! 3D mesh representation and loading.
//!
//! Provides the [`Mesh`] struct for storing vertices, normals, and faces, along with
//! OBJ file loading support via the `tobj` crate.

use std::fmt;

use crate::math::vec3::Vec3;

/// Represents a triangle face with indices into the vertex array.
/// Uses 0-based indexing.
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

/// A vertex with position and normal attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
    rotation: Vec3,
    scale: Vec3,
    translation: Vec3,
}

impl Mesh {
    pub(crate) fn new(
        vertices: Vec<Vertex>,
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
        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let (models, _materials) = tobj::load_obj(file_path, &load_options)?;

        let model = models.into_iter().next().ok_or(LoadError::NoModels)?;
        let mesh = model.mesh;

        if mesh.positions.is_empty() {
            return Err(LoadError::NoVertices);
        }

        if mesh.indices.len() % 3 != 0 {
            return Err(LoadError::InvalidFaces);
        }

        // With single_index: true, positions and normals are aligned
        let has_normals = !mesh.normals.is_empty();
        let vertices: Vec<Vertex> = mesh
            .positions
            .chunks_exact(3)
            .enumerate()
            .map(|(i, p)| {
                let normal = if has_normals {
                    let n = &mesh.normals[i * 3..i * 3 + 3];
                    Vec3::new(n[0], n[1], n[2])
                } else {
                    Vec3::ZERO
                };
                Vertex {
                    position: Vec3::new(p[0], p[1], p[2]),
                    normal,
                }
            })
            .collect();

        let faces: Vec<Face> = mesh
            .indices
            .chunks_exact(3)
            .map(|c| Face::new(c[0], c[1], c[2]))
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
    pub(crate) fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Get a reference to the faces
    pub(crate) fn faces(&self) -> &[Face] {
        &self.faces
    }

    /// Get a vertex for a specific face and vertex position (0, 1, or 2)
    pub(crate) fn get_face_vertex(&self, face_idx: usize, vertex_pos: usize) -> &Vertex {
        let idx = match vertex_pos {
            0 => self.faces[face_idx].a,
            1 => self.faces[face_idx].b,
            2 => self.faces[face_idx].c,
            _ => panic!("vertex_pos must be 0, 1, or 2"),
        };
        &self.vertices[idx as usize]
    }
}
