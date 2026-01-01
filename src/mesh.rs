//! 3D mesh representation and loading.
//!
//! Provides the [`Mesh`] struct for storing vertices, normals, and faces, along with
//! OBJ file loading support via the `tobj` crate.

use std::fmt;

use crate::{math::vec3::Vec3, prelude::Vec2};

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

pub type Texel = Vec2;

/// A vertex with position and normal attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub texel: Texel,
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

        // With single_index: true, tobj aligns all vertex attributes by index.
        // This means vertex i's data is found at:
        //   - positions[i*3 .. i*3+3]  (x, y, z)
        //   - normals[i*3 .. i*3+3]    (nx, ny, nz)
        //   - texcoords[i*2 .. i*2+2]  (u, v)
        //
        // The flat arrays look like:
        //   positions:  [x0, y0, z0, x1, y1, z1, x2, y2, z2, ...]
        //   normals:    [nx0, ny0, nz0, nx1, ny1, nz1, ...]
        //   texcoords:  [u0, v0, u1, v1, u2, v2, ...]
        let has_normals = !mesh.normals.is_empty();
        let has_texcoords = !mesh.texcoords.is_empty();
        let vertices: Vec<Vertex> = mesh
            .positions
            // chunks_exact(3) yields [x, y, z] slices for each vertex
            .chunks_exact(3)
            // enumerate gives (vertex_index, position_slice)
            .enumerate()
            .map(|(i, p)| {
                // Normals have 3 components, so vertex i starts at i * 3
                let normal = if has_normals {
                    let n = &mesh.normals[i * 3..i * 3 + 3];
                    Vec3::new(n[0], n[1], n[2])
                } else {
                    Vec3::ZERO
                };

                // Texcoords have 2 components (u, v), so vertex i starts at i * 2
                let texel = if has_texcoords {
                    let t = &mesh.texcoords[i * 2..i * 2 + 2];
                    Vec2::new(t[0], t[1])
                } else {
                    Vec2::ZERO
                };

                Vertex {
                    position: Vec3::new(p[0], p[1], p[2]),
                    normal,
                    texel,
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
}
