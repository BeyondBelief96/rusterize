use crate::math::vec3::Vec3;
use crate::mesh::{LoadError, Mesh, CUBE_FACES, CUBE_VERTICES};
use crate::rasterizer::{Rasterizer, RasterizerDispatcher, Triangle};
use crate::renderer::{Renderer, COLOR_BACKGROUND, COLOR_GRID};

pub use crate::rasterizer::RasterizerType;

const DEFAULT_FOV_FACTOR: f32 = 640.0;

// Configurable colors - change these at compile time
pub mod colors {
    pub const FILL: u32 = 0xFF444444; // Gray fill
    pub const WIREFRAME: u32 = 0xFF00FF00; // Green wireframe
    pub const VERTEX: u32 = 0xFFFF0000; // Red vertices
}

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
    fov_factor: f32,
    render_mode: RenderMode,
    pub backface_culling: bool,
    pub draw_grid: bool,
}

impl Engine {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            renderer: Renderer::new(width, height),
            rasterizer: RasterizerDispatcher::new(RasterizerType::default()),
            triangles_to_render: Vec::new(),
            mesh: Mesh::new(vec![], vec![], Vec3::ZERO),
            camera_position: Vec3::new(0.0, 0.0, -5.0),
            fov_factor: DEFAULT_FOV_FACTOR,
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
        self.mesh = Mesh::new(CUBE_VERTICES.to_vec(), CUBE_FACES.to_vec(), Vec3::ZERO);
    }

    pub fn load_mesh(&mut self, file_path: &str) -> Result<(), LoadError> {
        self.mesh = Mesh::from_obj(file_path)?;
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn set_camera_position(&mut self, position: Vec3) {
        self.camera_position = position;
    }

    pub fn camera_position(&self) -> Vec3 {
        self.camera_position
    }

    pub fn set_fov_factor(&mut self, fov: f32) {
        self.fov_factor = fov;
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

    /// Project a 3D point to 2D screen coordinates
    fn project(&self, point: Vec3) -> Option<Vec3> {
        // Clip points that are behind or too close to the camera
        if point.z < 0.1 {
            return None;
        }

        Some(Vec3::new(
            self.fov_factor * point.x / point.z,
            self.fov_factor * point.y / point.z,
            point.z,
        ))
    }

    /// Update the engine state - transforms vertices and builds triangles to render
    pub fn update(&mut self) {
        let faces = self.mesh.faces().to_vec();
        let vertices = self.mesh.vertices().to_vec();
        let rotation = self.mesh.rotation();
        let buffer_width = self.renderer.width();
        let buffer_height = self.renderer.height();
        let camera_position = self.camera_position;
        let backface_culling = self.backface_culling;

        let mut triangles = Vec::new();

        for face in faces.iter() {
            let face_vertices = [
                vertices[face.a as usize - 1],
                vertices[face.b as usize - 1],
                vertices[face.c as usize - 1],
            ];

            // Model Space --> World Space
            let mut transformed_vertices = Vec::new();
            for vertex in face_vertices.iter() {
                let mut transformed_vertex = *vertex;
                transformed_vertex = transformed_vertex.rotate_x(rotation.x);
                transformed_vertex = transformed_vertex.rotate_y(rotation.y);
                transformed_vertex = transformed_vertex.rotate_z(rotation.z);
                transformed_vertex.z -= camera_position.z;
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

            // Project all three vertices; skip triangle if any fail
            let p0 = self.project(transformed_vertices[0]);
            let p1 = self.project(transformed_vertices[1]);
            let p2 = self.project(transformed_vertices[2]);

            if let (Some(mut p0), Some(mut p1), Some(mut p2)) = (p0, p1, p2) {
                // Center on screen
                let half_width = buffer_width as f32 / 2.0;
                let half_height = buffer_height as f32 / 2.0;
                p0.x += half_width;
                p0.y += half_height;
                p1.x += half_width;
                p1.y += half_height;
                p2.x += half_width;
                p2.y += half_height;

                let avg_depth = (transformed_vertices[0].z
                    + transformed_vertices[1].z
                    + transformed_vertices[2].z)
                    / 3.0;

                triangles.push(Triangle::new([p0, p1, p2], colors::FILL, avg_depth));
            }
        }

        // Sort triangles by depth using the painter's algorithm (descending order)
        // Further away triangles are drawn first
        self.triangles_to_render
            .sort_by(|a, b| a.avg_depth.partial_cmp(&b.avg_depth).unwrap());

        self.triangles_to_render = triangles;
    }

    /// Render the current frame
    pub fn render(&mut self) {
        self.renderer.clear(COLOR_BACKGROUND);

        if self.draw_grid {
            self.renderer.draw_grid(50, COLOR_GRID);
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
