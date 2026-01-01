//! 3D model representation containing multiple meshes.
//!
//! A [`Model`] is a collection of [`Mesh`] instances loaded from a single file.
//! Each mesh can have its own local transform relative to the model's world transform.

use std::collections::HashMap;

use crate::mesh::{LoadError, Mesh};
use crate::texture::Texture;
use crate::transform::Transform;

/// A 3D model containing one or more meshes.
///
/// Models are loaded from OBJ files and can contain multiple named meshes.
/// The model has a world transform (position, rotation, scale) and each
/// mesh within it can have an additional local transform.
pub struct Model {
    name: String,
    meshes: Vec<Mesh>,
    mesh_names: HashMap<String, usize>,
    transform: Transform,
    texture: Option<Texture>,
}

impl Model {
    /// Create a new empty model with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            meshes: Vec::new(),
            mesh_names: HashMap::new(),
            transform: Transform::default(),
            texture: None,
        }
    }

    /// Load a model from an OBJ file.
    ///
    /// All objects/groups in the OBJ file become separate meshes within this model.
    pub fn from_obj(name: impl Into<String>, file_path: &str) -> Result<Self, LoadError> {
        let meshes = Mesh::load_all_from_obj(file_path)?;
        let mesh_names: HashMap<String, usize> = meshes
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name().to_string(), i))
            .collect();

        Ok(Self {
            name: name.into(),
            meshes,
            mesh_names,
            transform: Transform::default(),
            texture: None,
        })
    }

    /// Get the model name.
    pub fn name(&self) -> &str {
        &self.name
    }

    // ============ Transform Accessors ============

    /// Get a reference to the model's world transform.
    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    /// Get a mutable reference to the model's world transform.
    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    // ============ Mesh Access ============

    /// Get a mesh by name.
    pub fn mesh(&self, name: &str) -> Option<&Mesh> {
        self.mesh_names.get(name).map(|&i| &self.meshes[i])
    }

    /// Get a mutable reference to a mesh by name.
    pub fn mesh_mut(&mut self, name: &str) -> Option<&mut Mesh> {
        self.mesh_names
            .get(name)
            .copied()
            .map(move |i| &mut self.meshes[i])
    }

    /// Get a mesh by index.
    pub fn mesh_by_index(&self, index: usize) -> Option<&Mesh> {
        self.meshes.get(index)
    }

    /// Get a mutable reference to a mesh by index.
    pub fn mesh_by_index_mut(&mut self, index: usize) -> Option<&mut Mesh> {
        self.meshes.get_mut(index)
    }

    /// Get all meshes as a slice.
    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    /// Get all meshes as a mutable slice.
    pub fn meshes_mut(&mut self) -> &mut [Mesh] {
        &mut self.meshes
    }

    /// Get the number of meshes in this model.
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// Iterate over mesh names.
    pub fn mesh_names(&self) -> impl Iterator<Item = &str> {
        self.mesh_names.keys().map(|s| s.as_str())
    }

    /// Add a mesh to this model.
    pub fn add_mesh(&mut self, mesh: Mesh) {
        let name = mesh.name().to_string();
        let index = self.meshes.len();
        self.meshes.push(mesh);
        self.mesh_names.insert(name, index);
    }

    // ============ Texture ============

    /// Set the texture for this model.
    pub fn set_texture(&mut self, texture: Texture) {
        self.texture = Some(texture);
    }

    /// Clear the texture for this model.
    pub fn clear_texture(&mut self) {
        self.texture = None;
    }

    /// Get the texture for this model.
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }
}
