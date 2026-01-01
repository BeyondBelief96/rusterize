//! 3D mesh representation and loading.
//!
//! Provides the [`Mesh`] struct for storing vertices, normals, and faces, along with
//! OBJ file loading support via the `tobj` crate.

use std::fmt;

use crate::{math::vec3::Vec3, prelude::Vec2, transform::Transform};

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
    name: String,
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
    transform: Transform,
}

impl Mesh {
    pub(crate) fn new(name: String, vertices: Vec<Vertex>, faces: Vec<Face>) -> Self {
        Self {
            name,
            vertices,
            faces,
            transform: Transform::default(),
        }
    }

    /// Get the mesh name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Load all meshes from an OBJ file.
    /// Each object/group in the OBJ becomes a separate Mesh.
    pub(crate) fn load_all_from_obj(file_path: &str) -> Result<Vec<Self>, LoadError> {
        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let (models, _materials) = tobj::load_obj(file_path, &load_options)?;

        if models.is_empty() {
            return Err(LoadError::NoModels);
        }

        let mut meshes = Vec::with_capacity(models.len());

        for (index, model) in models.into_iter().enumerate() {
            let tobj_mesh = model.mesh;

            if tobj_mesh.positions.is_empty() {
                continue; // Skip empty meshes
            }

            if tobj_mesh.indices.len() % 3 != 0 {
                return Err(LoadError::InvalidFaces);
            }

            // Use the model name from OBJ, or generate a fallback
            let name = if model.name.is_empty() {
                format!("mesh_{}", index)
            } else {
                model.name
            };

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
            let has_normals = !tobj_mesh.normals.is_empty();
            let has_texcoords = !tobj_mesh.texcoords.is_empty();
            let vertices: Vec<Vertex> = tobj_mesh
                .positions
                // chunks_exact(3) yields [x, y, z] slices for each vertex
                .chunks_exact(3)
                // enumerate gives (vertex_index, position_slice)
                .enumerate()
                .map(|(i, p)| {
                    // Normals have 3 components, so vertex i starts at i * 3
                    let normal = if has_normals {
                        let n = &tobj_mesh.normals[i * 3..i * 3 + 3];
                        Vec3::new(n[0], n[1], n[2])
                    } else {
                        Vec3::ZERO
                    };

                    // Texcoords have 2 components (u, v), so vertex i starts at i * 2
                    let texel = if has_texcoords {
                        let t = &tobj_mesh.texcoords[i * 2..i * 2 + 2];
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

            let faces: Vec<Face> = tobj_mesh
                .indices
                .chunks_exact(3)
                .map(|c| Face::new(c[0], c[1], c[2]))
                .collect();

            meshes.push(Self::new(name, vertices, faces));
        }

        if meshes.is_empty() {
            return Err(LoadError::NoVertices);
        }

        Ok(meshes)
    }

    /// Get a reference to the transform.
    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    /// Get a mutable reference to the transform.
    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
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
