//! A CPU-based software-rendered 3D graphics engine.
//!
//! This crate provides a simple 3D rendering pipeline using SDL2 only for
//! window management and display. All rendering is done on the CPU.
//!
//! # Quick Start
//!
//! ```ignore
//! use russsty::prelude::*;
//!
//! let mut window = Window::new("My App", 800, 600)?;
//! let mut engine = Engine::new(800, 600);
//! engine.load_cube_mesh();
//! ```

// Public API - exposed to library consumers
pub mod camera;
pub mod colors;
pub mod engine;
pub mod light;
pub mod math;
pub mod model;
pub mod projection;
pub mod texture;
pub mod transform;
pub mod window;

// Internal modules - used within the crate only
pub(crate) mod clipper;
pub(crate) mod mesh;
pub(crate) mod render;

// Re-export commonly needed types at crate root for convenience
pub use engine::{Engine, RasterizerType, RenderMode, ShadingMode};
pub use mesh::{LoadError, Mesh};
pub use model::Model;
pub use projection::Projection;
pub use transform::Transform;

/// Prelude module for convenient imports.
///
/// # Example
/// ```ignore
/// use russsty::prelude::*;
/// ```
pub mod prelude {
    // Camera
    pub use crate::camera::{FpsCamera, FpsCameraController};

    // Engine
    pub use crate::engine::{Engine, RenderMode, ShadingMode, TextureMode};

    // Model
    pub use crate::model::Model;

    // Projection
    pub use crate::projection::Projection;

    // Transform
    pub use crate::transform::Transform;

    // Math
    pub use crate::math::mat4::Mat4;
    pub use crate::math::vec2::Vec2;
    pub use crate::math::vec3::Vec3;
    pub use crate::math::vec4::Vec4;

    // Rendering
    pub use crate::render::RasterizerType;

    // Window & Input
    pub use crate::window::{FpsCounter, FrameLimiter, InputState, Key, Window, WindowEvent};
}

/// Module exposing internals for benchmarking. Not part of the stable API.
pub mod bench {
    pub use crate::render::{
        EdgeFunctionRasterizer, FrameBuffer, Rasterizer, ScanlineRasterizer, Triangle,
    };
}
