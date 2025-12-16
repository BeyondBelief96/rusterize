// Public API - exposed to library consumers
pub mod engine;
pub mod math;
pub mod window;

// Internal modules - used within the crate only
pub(crate) mod framebuffer;
pub(crate) mod mesh;
pub(crate) mod rasterizer;
pub(crate) mod renderer;
pub(crate) mod sorting;

// Re-export commonly needed types at crate root for convenience
pub use engine::Engine;
pub use mesh::{LoadError, Mesh};

/// Module exposing internals for benchmarking. Not part of the stable API.
pub mod bench {
    pub use crate::framebuffer::FrameBuffer;
    pub use crate::rasterizer::{
        EdgeFunctionRasterizer, Rasterizer, ScanlineRasterizer, Triangle,
    };
}