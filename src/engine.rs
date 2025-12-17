//! Core rendering engine.
//!
//! The [`Engine`] struct is the main entry point for the renderer. It manages
//! the rendering pipeline including mesh transformation, projection, and
//! rasterization.

use crate::colors;
use crate::math::mat4::Mat4;
use crate::math::vec3::Vec3;
use crate::mesh::{LoadError, Mesh, CUBE_FACES, CUBE_VERTICES};
use crate::prelude::Vec4;
use crate::render::{Rasterizer, RasterizerDispatcher, Renderer, Triangle};

pub use crate::render::RasterizerType;

const DEFAULT_FOV_FACTOR: f32 = 640.0;

/// Rendering mode presets
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

pub struct Engine {
    renderer: Renderer,
    rasterizer: RasterizerDispatcher,
    triangles_to_render: Vec<Triangle>,
    mesh: Mesh,
    camera_position: Vec3,
    projection_matrix: Mat4,
    render_mode: RenderMode,
    pub backface_culling: bool,
    pub draw_grid: bool,
}

impl Engine {
    pub fn new(width: u32, height: u32) -> Self {
        let fov: f32 = 45.0;
        let aspect_ratio = width as f32 / height as f32;
        let projection_matrix = Mat4::perspective_lh(fov.to_radians(), aspect_ratio, 0.1, 100.0);

        Self {
            renderer: Renderer::new(width, height),
            rasterizer: RasterizerDispatcher::new(RasterizerType::default()),
            triangles_to_render: Vec::new(),
            mesh: Mesh::new(vec![], vec![], Vec3::ZERO, Vec3::ONE, Vec3::ZERO),
            camera_position: Vec3::new(0.0, 0.0, -5.0),
            projection_matrix,
            render_mode: RenderMode::default(),
            backface_culling: true,
            draw_grid: true,
        }
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

    pub fn load_cube_mesh(&mut self) {
        self.mesh = Mesh::new(
            CUBE_VERTICES.to_vec(),
            CUBE_FACES.to_vec(),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::ZERO,
        );
    }

    pub fn load_mesh(&mut self, file_path: &str) -> Result<(), LoadError> {
        self.mesh = Mesh::from_obj(file_path)?;
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.projection_matrix =
            Mat4::perspective_lh(45.0, width as f32 / height as f32, 0.1, 100.0);
    }

    pub fn set_camera_position(&mut self, position: Vec3) {
        self.camera_position = position;
    }

    pub fn camera_position(&self) -> Vec3 {
        self.camera_position
    }

    pub fn mesh_mut(&mut self) -> &mut Mesh {
        &mut self.mesh
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Returns the rendered frame as bytes (ARGB8888 format)
    pub fn frame_buffer(&self) -> &[u8] {
        self.renderer.as_bytes()
    }

    /// Update the engine state - transforms vertices and builds triangles to render
    pub fn update(&mut self) {
        let faces = self.mesh.faces().to_vec();
        let vertices = self.mesh.vertices().to_vec();
        let rotation = self.mesh.rotation();
        let translation = self.mesh.translation();
        let scale = self.mesh().scale();
        let buffer_width = self.renderer.width();
        let buffer_height = self.renderer.height();
        let camera_position = self.camera_position;
        let backface_culling = self.backface_culling;

        let mut triangles = Vec::new();
        let world_matrix = Mat4::translation(translation.x, translation.y, translation.z)
            * Mat4::rotation_x(rotation.x)
            * Mat4::rotation_y(rotation.y)
            * Mat4::rotation_z(rotation.z)
            * Mat4::scaling(scale.x, scale.y, scale.z);

        self.mesh_mut().translation().z = camera_position.z;
        for face in faces.iter() {
            let face_vertices = [
                vertices[face.a as usize - 1],
                vertices[face.b as usize - 1],
                vertices[face.c as usize - 1],
            ];

            // Model Space --> World Space
            let mut transformed_vertices = Vec::new();
            for vertex in face_vertices.iter() {
                let transformed_vertex = world_matrix * *vertex;

                transformed_vertices.push(transformed_vertex);
            }

            // No camera/view space transformation yet, however, we can consider ourselves in camera space at this point.
            // Apply backface culling
            if backface_culling {
                let vec_ab = transformed_vertices[1] - transformed_vertices[0];
                let vec_ac = transformed_vertices[2] - transformed_vertices[0];

                // This normal is not normalized, but we only care about its direction.
                let normal = vec_ab.cross(vec_ac);

                // In view space, camera is at origin. Vector from vertex to camera is just -vertex.
                let camera_ray = -transformed_vertices[0];
                if normal.dot(camera_ray) < 0.0 {
                    continue;
                }
            }

            let mut projected_vertices = Vec::new();
            for vertex in &transformed_vertices {
                let clip_space_vertex =
                    self.projection_matrix * Vec4::new(vertex.x, vertex.y, vertex.z, 1.0);

                // w <= 0 means vertex is behind or on the near plane.
                if clip_space_vertex.w <= 0.0 {
                    continue;
                }

                let ndc_vertex = Vec3::new(
                    clip_space_vertex.x / clip_space_vertex.w,
                    clip_space_vertex.y / clip_space_vertex.w,
                    clip_space_vertex.z / clip_space_vertex.w,
                );

                let screen_x = (ndc_vertex.x + 1.0) * 0.5 * buffer_width as f32;
                let screen_y = (1.0 - ndc_vertex.y) * 0.5 * buffer_height as f32;
                projected_vertices.push(Vec3::new(screen_x, screen_y, clip_space_vertex.w));
            }

            if projected_vertices.len() == 3 {
                let avg_depth = (transformed_vertices[0].z
                    + transformed_vertices[1].z
                    + transformed_vertices[2].z)
                    / 3.0;

                triangles.push(Triangle::new(
                    [
                        projected_vertices[0],
                        projected_vertices[1],
                        projected_vertices[2],
                    ],
                    colors::FILL,
                    avg_depth,
                ));
            }
        }

        // Sort triangles by depth using the painter's algorithm (descending order)
        // Further away triangles are drawn first
        triangles.sort_by(|a, b| b.avg_depth.partial_cmp(&a.avg_depth).unwrap());

        self.triangles_to_render = triangles;
    }

    /// Render the current frame
    pub fn render(&mut self) {
        self.renderer.clear(colors::BACKGROUND);

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
            for triangle in &self.triangles_to_render {
                self.rasterizer
                    .fill_triangle(triangle, &mut fb, triangle.color);
            }
        }

        // Wireframe and vertices (uses renderer methods)
        for triangle in &self.triangles_to_render {
            if draw_wireframe {
                self.renderer
                    .draw_triangle_wireframe(triangle, colors::WIREFRAME);
            }
            if draw_vertices {
                for vertex in &triangle.points {
                    self.renderer
                        .draw_rect(vertex.x as i32, vertex.y as i32, 4, 4, colors::VERTEX);
                }
            }
        }
    }
}
