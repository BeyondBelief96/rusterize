//! Rendering subsystem.
//!
//! This module contains all rendering-related components:
//! - [`FrameBuffer`]: A view into a 2D pixel buffer for safe pixel access
//! - [`Renderer`]: Owns the color buffer and provides primitive drawing operations
//! - [`rasterizer`]: Triangle rasterization algorithms

pub mod framebuffer;
pub mod rasterizer;
pub mod renderer;

pub use framebuffer::FrameBuffer;
pub use rasterizer::{
    EdgeFunctionRasterizer, Rasterizer, RasterizerDispatcher, RasterizerType, ScanlineRasterizer,
    ScreenVertex, Triangle,
};
pub use renderer::Renderer;
