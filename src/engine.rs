//! Core rendering engine.
//!
//! The [`Engine`] struct is the main entry point for the renderer. It manages
//! the rendering pipeline including mesh transformation, projection, and
//! rasterization.

use std::collections::HashMap;

use crate::camera::FpsCamera;
use crate::clipper::{ClipSpaceClipper, ClipSpacePolygon, ClipSpaceVertex};
use crate::colors;
use crate::frustum::{Frustum, FrustumTest};
use crate::light::DirectionalLight;
use crate::mesh::{LoadError, Texel, Vertex};
use crate::model::Model;
use crate::prelude::{Mat4, Vec2, Vec3, Vec4};
use crate::projection::Projection;
use crate::render::{Rasterizer, RasterizerDispatcher, Renderer, ScreenVertex, Triangle};

pub use crate::render::RasterizerType;
use crate::texture::Texture;

/// What primitives get drawn for each triangle.
///
/// This controls *drawing style* only — it is independent of `ShadingMode`
/// and `TextureMode`, which control how filled pixels are colored.
///
/// | Variant | Wireframe lines | Filled interior | Vertex dots |
/// |---------|-----------------|-----------------|-------------|
/// | `Wireframe` | yes | no | no |
/// | `WireframeVertices` | yes | no | yes |
/// | `FilledWireframe` | yes | yes | no |
/// | `FilledWireframeVertices` | yes | yes | yes |
/// | `Filled` | no | yes | no |
///
/// When only wireframe lines are drawn, `ShadingMode` and `TextureMode` are
/// irrelevant — line drawing always uses `Triangle::color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Wireframe only (key: 1)
    Wireframe,
    /// Wireframe + vertices (key: 2)
    WireframeVertices,
    /// Filled + wireframe (key: 3)
    #[default]
    FilledWireframe,
    /// Filled + wireframe + vertices (key: 4)
    FilledWireframeVertices,
    /// Filled only (key: 5)
    Filled,
}

/// How per-vertex lighting is computed and stored into `Triangle::vertex_colors`.
///
/// `ShadingMode` controls the *lighting* step only — what color each vertex
/// ends up with after the directional light is applied. How that lit color
/// reaches the final pixel depends on `TextureMode`:
///
/// - With `TextureMode::None`, the lit `vertex_colors` are used directly
///   (interpolated across the triangle for `Gouraud`; uniform for `Flat`).
/// - With `TextureMode::Modulate`, the lit `vertex_colors` tint the texture
///   sample (texel × light).
/// - With `TextureMode::Replace`, the lit `vertex_colors` are ignored — the
///   texture sample is used verbatim, so `ShadingMode` has no visible effect.
///
/// Lighting is precomputed once per frame in `Engine::update` and baked into
/// `Triangle::vertex_colors`; the rasterizer never re-evaluates the light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadingMode {
    /// No lighting applied. All three entries of `vertex_colors` are set to
    /// `Triangle::color` so downstream code can treat `vertex_colors`
    /// uniformly regardless of shading mode.
    None,
    /// One lit color per face, computed from the face normal. All three
    /// entries of `vertex_colors` are identical, so there is no visible
    /// gradient across the triangle.
    #[default]
    Flat,
    /// Lighting is computed at each vertex from the vertex normal and
    /// interpolated across the triangle via barycentric coordinates,
    /// producing a smooth gradient.
    Gouraud,
}

/// How a texture sample (if any) combines with the lit vertex color.
///
/// `TextureMode` is orthogonal to `ShadingMode`:
/// - `ShadingMode` decides *what* `vertex_colors` contains (the lit color).
/// - `TextureMode` decides *how* that lit color is used at each pixel.
///
/// Combination reference (for filled pixels only):
///
/// | `TextureMode` | Pixel color | `ShadingMode` influence |
/// |---------------|-------------|--------------------------|
/// | `None` | interpolated `vertex_colors` | full — this *is* the lit color |
/// | `Replace` | texture sample (texel) | none — lighting is ignored |
/// | `Modulate` | texel × interpolated `vertex_colors` | full — lighting tints the texel |
///
/// Naming note: `Replace` and `Modulate` mirror the classic fixed-function
/// OpenGL `glTexEnv` terminology. Think of them as "texture only" (unlit)
/// and "texture × light" (lit) respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureMode {
    /// No texture sampled; the lit `vertex_colors` (or `color` when
    /// `ShadingMode::None`) are used directly.
    #[default]
    None,
    /// Texture sample replaces the color entirely. Lighting from
    /// `ShadingMode` is discarded — use this for unlit surfaces like
    /// skyboxes, UI, or textures that already have lighting baked in.
    Replace,
    /// Texture sample is multiplied component-wise by the lit
    /// `vertex_colors`. This is the standard "textured and lit" path:
    /// the texture provides surface detail, lighting provides shading.
    Modulate,
}

impl std::fmt::Display for ShadingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadingMode::None => write!(f, "None"),
            ShadingMode::Flat => write!(f, "Flat"),
            ShadingMode::Gouraud => write!(f, "Gouraud"),
        }
    }
}

impl std::fmt::Display for TextureMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureMode::None => write!(f, "None"),
            TextureMode::Replace => write!(f, "Replace"),
            TextureMode::Modulate => write!(f, "Modulate"),
        }
    }
}

pub struct Engine {
    renderer: Renderer,
    rasterizer: RasterizerDispatcher,
    // Triangles grouped by model index for per-model texture support
    triangles_per_model: Vec<Vec<Triangle>>,
    models: Vec<Model>,
    model_names: HashMap<String, usize>,
    // Global texture fallback (used when model doesn't have its own)
    global_texture: Option<Texture>,
    camera: FpsCamera,
    projection: Projection,
    projection_matrix: Mat4,
    clipper: ClipSpaceClipper,
    render_mode: RenderMode,
    texture_mode: TextureMode,
    shading_mode: ShadingMode,
    light: DirectionalLight,
    pub backface_culling: bool,
    pub draw_grid: bool,
}

impl Engine {
    pub fn new(width: u32, height: u32) -> Self {
        let aspect_ratio = width as f32 / height as f32;
        let projection = Projection::from_degrees(45.0, aspect_ratio, 0.1, 100.0);

        Self {
            renderer: Renderer::new(width, height),
            rasterizer: RasterizerDispatcher::new(RasterizerType::default()),
            triangles_per_model: Vec::new(),
            models: Vec::new(),
            model_names: HashMap::new(),
            global_texture: None,
            camera: FpsCamera::new(Vec3::new(0.0, 0.0, -5.0)),
            projection_matrix: projection.matrix(),
            clipper: ClipSpaceClipper::new(),
            projection,
            texture_mode: TextureMode::default(),
            render_mode: RenderMode::default(),
            shading_mode: ShadingMode::default(),
            light: DirectionalLight::new(Vec3::new(0.0, 0.0, 1.0)),
            backface_culling: true,
            draw_grid: true,
        }
    }

    pub fn set_shading_mode(&mut self, mode: ShadingMode) {
        self.shading_mode = mode;
    }

    pub fn shading_mode(&self) -> ShadingMode {
        self.shading_mode
    }

    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }

    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    pub fn set_rasterizer(&mut self, rasterizer_type: RasterizerType) {
        self.rasterizer.set_type(rasterizer_type);
    }

    pub fn rasterizer(&self) -> RasterizerType {
        self.rasterizer.active_type()
    }

    // ============ Model Management ============

    /// Add a model from an OBJ file with the given name.
    /// Returns the model index for efficient access.
    pub fn add_model(&mut self, name: &str, file_path: &str) -> Result<usize, LoadError> {
        let model = Model::from_obj(name, file_path)?;
        let index = self.models.len();
        self.model_names.insert(name.to_string(), index);
        self.models.push(model);
        Ok(index)
    }

    /// Get a model by name.
    pub fn model(&self, name: &str) -> Option<&Model> {
        self.model_names.get(name).map(|&i| &self.models[i])
    }

    /// Get a mutable reference to a model by name.
    pub fn model_mut(&mut self, name: &str) -> Option<&mut Model> {
        self.model_names
            .get(name)
            .copied()
            .map(move |i| &mut self.models[i])
    }

    /// Get a model by index.
    pub fn model_by_index(&self, index: usize) -> Option<&Model> {
        self.models.get(index)
    }

    /// Get a mutable reference to a model by index.
    pub fn model_by_index_mut(&mut self, index: usize) -> Option<&mut Model> {
        self.models.get_mut(index)
    }

    /// Get all models as a slice.
    pub fn models(&self) -> &[Model] {
        &self.models
    }

    /// Get the number of models in the scene.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Remove a model by name. Returns the removed model if found.
    pub fn remove_model(&mut self, name: &str) -> Option<Model> {
        if let Some(&index) = self.model_names.get(name) {
            self.model_names.remove(name);
            let model = self.models.remove(index);
            // Update indices for models after the removed one
            for (_, idx) in self.model_names.iter_mut() {
                if *idx > index {
                    *idx -= 1;
                }
            }
            Some(model)
        } else {
            None
        }
    }

    /// Clear all models from the scene.
    pub fn clear_models(&mut self) {
        self.models.clear();
        self.model_names.clear();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        let aspect_ratio = width as f32 / height as f32;
        self.projection.set_aspect_ratio(aspect_ratio);
        self.projection_matrix = self.projection.matrix();
        // Note: ClipSpaceClipper doesn't need rebuilding - it uses fixed planes
    }

    pub fn camera(&self) -> &FpsCamera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut FpsCamera {
        &mut self.camera
    }

    pub fn set_camera_position(&mut self, position: Vec3) {
        self.camera.set_position(position);
    }

    pub fn camera_position(&self) -> Vec3 {
        self.camera.position()
    }

    pub fn set_light_direction(&mut self, direction: Vec3) {
        self.light = DirectionalLight::new(direction);
    }

    pub fn light_direction(&self) -> Vec3 {
        self.light.direction
    }

    /// Returns the rendered frame as bytes (ARGB8888 format)
    pub fn frame_buffer(&self) -> &[u8] {
        self.renderer.as_bytes()
    }

    /// Set the global texture (used when models don't have their own).
    pub fn set_texture(&mut self, texture: Texture) {
        self.global_texture = Some(texture);
    }

    /// Clear the global texture.
    pub fn clear_texture(&mut self) {
        self.global_texture = None;
    }

    /// Get the global texture.
    pub fn texture(&self) -> Option<&Texture> {
        self.global_texture.as_ref()
    }

    pub fn set_texture_mode(&mut self, mode: TextureMode) {
        self.texture_mode = mode;
    }

    pub fn texture_mode(&self) -> TextureMode {
        self.texture_mode
    }

    /// Update the engine state - transforms vertices and builds triangles to render.
    pub fn update(&mut self) {
        let buffer_width = self.renderer.width();
        let buffer_height = self.renderer.height();
        let camera_position = self.camera.position();
        let view_matrix = self.camera.view_matrix();
        // Extract world-space frustum planes from VP via Gribb-Hartmann.
        // World-space planes let us skip a per-mesh view_matrix multiply in
        // every cull test below.
        let frustum = Frustum::from_matrix(&(self.projection_matrix * view_matrix));
        let backface_culling = self.backface_culling;
        let shading_mode = self.shading_mode;

        let mut triangles_per_model: Vec<Vec<Triangle>> = Vec::with_capacity(self.models.len());

        // Iterate over all models in the scene
        for model in &self.models {
            let mut model_triangles = Vec::new();

            // Model world matrix from transform
            let model_world_matrix = model.transform().to_matrix();

            // --- Model-level hierarchical frustum test ---
            // Classify the model's enclosing sphere first. If the whole model
            // is off-screen we skip every mesh; if it's fully inside we skip
            // the per-mesh frustum tests (they're guaranteed to pass).
            let model_bounds = model.bounds();
            let model_world_center = model_world_matrix * model_bounds.center;
            let m_scl = model.transform().scale();
            let model_scale_max = m_scl.x.abs().max(m_scl.y.abs()).max(m_scl.z.abs());
            let model_world_radius = model_bounds.radius * model_scale_max;

            let skip_mesh_cull =
                match frustum.classify_sphere(model_world_center, model_world_radius) {
                    FrustumTest::Outside => {
                        triangles_per_model.push(model_triangles);
                        continue;
                    }
                    FrustumTest::FullyInside => true,
                    FrustumTest::Intersecting => false,
                };

            // Iterate over all meshes in this model
            for mesh in model.meshes() {
                // Mesh local matrix from transform
                let mesh_local_matrix = mesh.transform().to_matrix();

                // Combined world matrix: model_world * mesh_local
                let world_matrix = model_world_matrix * mesh_local_matrix;

                // Scales are needed both for the cull radius and the normal matrix.
                let model_scl = model.transform().scale();
                let mesh_scl = mesh.transform().scale();

                if !skip_mesh_cull {
                    // --- Layer 1: bounding-sphere test (with coherency cache) ---
                    let bounds_world_center = world_matrix * mesh.bounds().center;
                    let scale_max = (model_scl.x * mesh_scl.x)
                        .abs()
                        .max((model_scl.y * mesh_scl.y).abs())
                        .max((model_scl.z * mesh_scl.z).abs());
                    let world_radius = scale_max * mesh.bounds().radius;

                    if !frustum.contains_sphere_cached(
                        bounds_world_center,
                        world_radius,
                        mesh.cull_cache(),
                    ) {
                        continue;
                    }

                    // --- Layer 2: AABB n/p-vertex test for a tighter answer ---
                    // Transform the 8 local-space AABB corners into world space,
                    // then take their enclosing axis-aligned box.
                    let mut world_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                    let mut world_max =
                        Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
                    for c in mesh.aabb().corners() {
                        let v = world_matrix * c;
                        world_min.x = world_min.x.min(v.x);
                        world_min.y = world_min.y.min(v.y);
                        world_min.z = world_min.z.min(v.z);
                        world_max.x = world_max.x.max(v.x);
                        world_max.y = world_max.y.max(v.y);
                        world_max.z = world_max.z.max(v.z);
                    }
                    if frustum.aabb_outside(world_min, world_max) {
                        continue;
                    }
                }

                let faces = mesh.faces();
                let vertices = mesh.vertices();

                // Normal matrix = inverse transpose of rotation+scale (excludes translation)
                // Combine model and mesh rotation+scale for correct normal transformation
                let model_rot = model.transform().rotation();
                let mesh_rot = mesh.transform().rotation();

                let combined_rotation_scale = Mat4::rotation_x(model_rot.x)
                    * Mat4::rotation_y(model_rot.y)
                    * Mat4::rotation_z(model_rot.z)
                    * Mat4::scaling(model_scl.x, model_scl.y, model_scl.z)
                    * Mat4::rotation_x(mesh_rot.x)
                    * Mat4::rotation_y(mesh_rot.y)
                    * Mat4::rotation_z(mesh_rot.z)
                    * Mat4::scaling(mesh_scl.x, mesh_scl.y, mesh_scl.z);

                let normal_matrix = combined_rotation_scale
                    .inverse()
                    .unwrap_or(Mat4::identity())
                    .transpose();

                for face in faces.iter() {
                    let face_vertices: [Vertex; 3] = [
                        vertices[face.a as usize],
                        vertices[face.b as usize],
                        vertices[face.c as usize],
                    ];

                    let face_texcoords: [Texel; 3] = [
                        face_vertices[0].texel,
                        face_vertices[1].texel,
                        face_vertices[2].texel,
                    ];

                    // Model Space --> World Space (positions)
                    let world_space_positions = [
                        world_matrix * face_vertices[0].position,
                        world_matrix * face_vertices[1].position,
                        world_matrix * face_vertices[2].position,
                    ];

                    // Calculate face normal (needed for backface culling).
                    // Note: this is a left-handed coordinate system, so under
                    // the left-hand rule (B-A) × (C-A) points toward the
                    // camera exactly when the triangle is wound CW from the
                    // viewer's side. CW is therefore "front-facing" here.
                    let vec_ab = world_space_positions[1] - world_space_positions[0];
                    let vec_ac = world_space_positions[2] - world_space_positions[0];
                    let face_normal = vec_ab.cross(vec_ac);

                    // Backface cull: if the face normal points away from the
                    // camera (dot with the camera-ward ray is negative), the
                    // triangle is facing away and we skip it. Flip this sign
                    // if the scene's meshes are CCW-wound.
                    if backface_culling {
                        let camera_ray = camera_position - world_space_positions[0];
                        if face_normal.dot(camera_ray) < 0.0 {
                            continue;
                        }
                    }

                    // Transform to view (camera) space
                    let view_space_positions = [
                        view_matrix * world_space_positions[0],
                        view_matrix * world_space_positions[1],
                        view_matrix * world_space_positions[2],
                    ];

                    // Calculate colors based on shading mode
                    // Use white for textured modulate mode so lighting doesn't darken the texture
                    let base_color = if self.texture_mode == TextureMode::Modulate {
                        0xFFFFFFFF // White - full brightness when lit
                    } else {
                        colors::FILL
                    };
                    let (flat_color, vertex_colors) = match shading_mode {
                        ShadingMode::None => {
                            // No lighting - use base color
                            (base_color, [base_color, base_color, base_color])
                        }
                        ShadingMode::Flat => {
                            // Flat shading - one color per face based on face normal
                            let normal = face_normal.normalize();
                            let diffuse =
                                self.light.intensity(normal) * self.light.diffuse_strength;
                            let intensity = (diffuse + self.light.ambient_intensity).min(1.0);
                            let color = colors::modulate(base_color, intensity);
                            (color, [color, color, color])
                        }
                        ShadingMode::Gouraud => {
                            // Gouraud shading - per-vertex lighting
                            let mut vert_colors = [0u32; 3];
                            for i in 0..3 {
                                let world_normal =
                                    (normal_matrix * face_vertices[i].normal).normalize();
                                let diffuse = self.light.intensity(world_normal)
                                    * self.light.diffuse_strength;
                                let intensity = (diffuse + self.light.ambient_intensity).min(1.0);
                                vert_colors[i] = colors::modulate(base_color, intensity);
                            }
                            let avg_color = vert_colors[0];
                            (avg_color, vert_colors)
                        }
                    };

                    // ==================== PROJECT TO CLIP SPACE ====================
                    // Transform from view space to clip space (homogeneous coordinates)
                    let clip_space_positions = [
                        self.projection_matrix * Vec4::from_vec3(view_space_positions[0], 1.0),
                        self.projection_matrix * Vec4::from_vec3(view_space_positions[1], 1.0),
                        self.projection_matrix * Vec4::from_vec3(view_space_positions[2], 1.0),
                    ];

                    // ==================== CLIP IN CLIP SPACE ====================
                    // Create ClipSpaceVertex instances with homogeneous positions
                    let clip_vertices = [
                        ClipSpaceVertex::new(
                            clip_space_positions[0],
                            face_texcoords[0],
                            vertex_colors[0],
                        ),
                        ClipSpaceVertex::new(
                            clip_space_positions[1],
                            face_texcoords[1],
                            vertex_colors[1],
                        ),
                        ClipSpaceVertex::new(
                            clip_space_positions[2],
                            face_texcoords[2],
                            vertex_colors[2],
                        ),
                    ];

                    // Clip against the canonical clip cube: -w <= x,y,z <= w
                    let polygon = ClipSpacePolygon::from_triangle(
                        clip_vertices[0],
                        clip_vertices[1],
                        clip_vertices[2],
                    );
                    let clipped_polygon = self.clipper.clip_polygon(polygon);

                    // Skip if polygon was completely clipped away
                    if clipped_polygon.is_empty() {
                        continue;
                    }

                    // ==================== PERSPECTIVE DIVIDE & VIEWPORT TRANSFORM ====================
                    // Triangulate the clipped polygon and transform to screen space
                    for (v0, v1, v2) in clipped_polygon.triangulate() {
                        let clipped_positions = [v0.position, v1.position, v2.position];
                        let clipped_texcoords = [v0.texcoord, v1.texcoord, v2.texcoord];
                        let clipped_colors = [v0.color, v1.color, v2.color];

                        let mut screen_vertices = [ScreenVertex::new(Vec2::ZERO, 0.0); 3];
                        let mut all_valid = true;

                        for (i, clip_pos) in clipped_positions.iter().enumerate() {
                            // After clipping, w should always be positive
                            // but check anyway for safety
                            if clip_pos.w <= 0.0 {
                                all_valid = false;
                                break;
                            }

                            // Perspective divide: clip space -> NDC [-1, 1]
                            let ndc_x = clip_pos.x / clip_pos.w;
                            let ndc_y = clip_pos.y / clip_pos.w;

                            // Viewport transform: NDC -> screen coordinates
                            let screen_x = (ndc_x + 1.0) * 0.5 * buffer_width as f32;
                            let screen_y = (1.0 - ndc_y) * 0.5 * buffer_height as f32;

                            // Store w for depth buffer (1/w) and perspective-correct interpolation
                            screen_vertices[i] =
                                ScreenVertex::new(Vec2::new(screen_x, screen_y), clip_pos.w);
                        }

                        if all_valid {
                            // Use flat_color for flat shading, interpolated colors for Gouraud
                            let tri_color = if shading_mode == ShadingMode::Gouraud {
                                clipped_colors[0] // Use first vertex color as representative
                            } else {
                                flat_color
                            };

                            model_triangles.push(Triangle::new(
                                screen_vertices,
                                tri_color,
                                clipped_colors,
                                clipped_texcoords,
                                shading_mode,
                                self.texture_mode,
                            ));
                        }
                    }
                }
            }

            triangles_per_model.push(model_triangles);
        }

        // No sorting needed - depth buffer handles hidden surface removal
        self.triangles_per_model = triangles_per_model;
    }

    /// Render the current frame
    pub fn render(&mut self) {
        self.renderer.clear(colors::BACKGROUND);
        self.renderer.clear_depth();

        if self.draw_grid {
            self.renderer.draw_grid(50, colors::GRID);
        }

        // Determine what to draw based on render mode
        let (draw_filled, draw_wireframe, draw_vertices) = match self.render_mode {
            RenderMode::Wireframe => (false, true, false),
            RenderMode::WireframeVertices => (false, true, true),
            RenderMode::FilledWireframe => (true, true, false),
            RenderMode::FilledWireframeVertices => (true, true, true),
            RenderMode::Filled => (true, false, false),
        };

        // Fill triangles first (requires framebuffer borrow)
        if draw_filled {
            let mut fb = self.renderer.as_framebuffer();
            // Render each model's triangles with its own texture
            for (model_idx, triangles) in self.triangles_per_model.iter().enumerate() {
                // Use model's texture if available, otherwise global texture
                let texture = self
                    .models
                    .get(model_idx)
                    .and_then(|m| m.texture())
                    .or(self.global_texture.as_ref());

                for triangle in triangles {
                    self.rasterizer
                        .fill_triangle(triangle, &mut fb, triangle.color, texture);
                }
            }
        }

        // Wireframe and vertices (uses renderer methods)
        for triangles in &self.triangles_per_model {
            for triangle in triangles {
                if draw_wireframe {
                    self.renderer
                        .draw_triangle_wireframe(triangle, colors::WIREFRAME);
                }
                if draw_vertices {
                    for vertex in &triangle.points {
                        self.renderer.draw_rect(
                            vertex.position.x as i32,
                            vertex.position.y as i32,
                            4,
                            4,
                            colors::VERTEX,
                        );
                    }
                }
            }
        }
    }
}
