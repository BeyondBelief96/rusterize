//! Triangle rasterization algorithms.
//!
//! This module provides multiple rasterizer implementations that can be
//! swapped at runtime for testing and benchmarking purposes.
//!
//! Available algorithms:
//! - [`ScanlineRasterizer`]: Flat-top/flat-bottom triangle decomposition
//! - [`EdgeFunctionRasterizer`]: Bounding box iteration with edge function tests

mod edgefunction;
mod scanline;

pub use edgefunction::EdgeFunctionRasterizer;
pub use scanline::ScanlineRasterizer;

use super::framebuffer::FrameBuffer;
use crate::math::vec3::Vec3;

/// A triangle ready for rasterization in screen space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub points: [Vec3; 3],
    pub color: u32, // Used for wireframe, and when flat shading
    pub vertex_colors: [u32; 3],
    pub avg_depth: f32,
}

impl Triangle {
    pub fn new(points: [Vec3; 3], color: u32, vertex_colors: [u32; 3], avg_depth: f32) -> Self {
        Self {
            points,
            color,
            vertex_colors,
            avg_depth,
        }
    }
}

/// Trait for triangle rasterization algorithms.
///
/// Implementors define how triangles are filled into a pixel buffer.
/// This allows swapping between different rasterization strategies
/// (scanline, edge functions, etc.) for testing and benchmarking.
pub trait Rasterizer {
    /// Fill a triangle into the frame buffer.
    ///
    /// # Arguments
    /// * `triangle` - The triangle to rasterize
    /// * `buffer` - The frame buffer to draw into
    /// * `color` - The color to fill the triangle with
    fn fill_triangle(&self, triangle: &Triangle, buffer: &mut FrameBuffer, color: u32);
}

/// Available rasterization algorithms.
///
/// Use this enum to select which rasterizer the engine should use.
/// Can be changed at runtime via `Engine::set_rasterizer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RasterizerType {
    /// Scanline rasterizer using flat-top/flat-bottom triangle decomposition.
    /// Generally faster for larger triangles due to efficient horizontal span filling.
    #[default]
    Scanline,
    /// Edge function rasterizer that tests each pixel in the bounding box.
    /// Simpler algorithm, forms the basis for GPU rasterization.
    /// Better for small triangles or when barycentric coordinates are needed.
    EdgeFunction,
}

impl std::fmt::Display for RasterizerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RasterizerType::Scanline => write!(f, "Scanline"),
            RasterizerType::EdgeFunction => write!(f, "EdgeFunction"),
        }
    }
}

/// Internal dispatcher that holds both rasterizer implementations.
pub struct RasterizerDispatcher {
    scanline: ScanlineRasterizer,
    edge_function: EdgeFunctionRasterizer,
    active: RasterizerType,
}

impl RasterizerDispatcher {
    pub fn new(rasterizer_type: RasterizerType) -> Self {
        Self {
            scanline: ScanlineRasterizer::new(),
            edge_function: EdgeFunctionRasterizer::new(),
            active: rasterizer_type,
        }
    }

    pub fn set_type(&mut self, rasterizer_type: RasterizerType) {
        self.active = rasterizer_type;
    }

    pub fn active_type(&self) -> RasterizerType {
        self.active
    }
}

impl Rasterizer for RasterizerDispatcher {
    #[inline]
    fn fill_triangle(&self, triangle: &Triangle, buffer: &mut FrameBuffer, color: u32) {
        match self.active {
            RasterizerType::Scanline => self.scanline.fill_triangle(triangle, buffer, color),
            RasterizerType::EdgeFunction => {
                self.edge_function.fill_triangle(triangle, buffer, color)
            }
        }
    }
}
