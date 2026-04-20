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
pub mod shader;

pub use edgefunction::EdgeFunctionRasterizer;
pub use scanline::ScanlineRasterizer;

use super::framebuffer::FrameBuffer;
use crate::{engine::TextureMode, prelude::Vec2, texture::Texture, ShadingMode};

/// A projected vertex in screen space, paired with its clip-space `w`.
///
/// Produced by `Engine::update` once a vertex has cleared clipping and been
/// pushed through the perspective divide and viewport transform. The
/// rasterizer consumes these through [`Triangle::points`] and by that point
/// no other coordinate space is relevant — a `ScreenVertex` is already
/// pixel-addressable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenVertex {
    /// Pixel-space position after the perspective divide and viewport
    /// transform. `(0, 0)` is the top-left of the framebuffer; `+x` is
    /// right, `+y` is down.
    pub position: Vec2,

    /// Clip-space `w`, preserved from before the perspective divide. Used
    /// to derive `1/w` for depth testing and perspective-correct
    /// interpolation of per-vertex attributes.
    pub w: f32,
}

impl ScreenVertex {
    /// Create a `ScreenVertex` from a pixel-space position and clip-space `w`.
    ///
    /// Normally called once per vertex inside `Engine::update` right after
    /// the viewport transform. Downstream code consumes these through
    /// [`Triangle::points`] and should not need to build them by hand.
    #[inline]
    pub fn new(position: Vec2, w: f32) -> Self {
        Self { position, w }
    }
}

/// A triangle ready for rasterization in screen space.
///
/// After the engine has transformed, lit, clipped, and projected a face, it
/// packs the result into this struct and hands it to a [`Rasterizer`]. Every
/// field the rasterizer or line drawer might need is present — which ones
/// actually get read depends on the selected `RenderMode`, `ShadingMode`,
/// and `TextureMode`.
///
/// # Field usage by mode
///
/// | Field | Wireframe line drawing | Filled (`None` shading) | Filled (`Flat`/`Gouraud` shading) | Textured (`Replace`) | Textured (`Modulate`) |
/// |-------|------------------------|-------------------------|-----------------------------------|----------------------|-----------------------|
/// | `points` | yes | yes | yes | yes | yes |
/// | `color` | yes (line color) | yes (fill color) | no¹ | no | no |
/// | `vertex_colors` | no | no¹ | yes (lit color per vertex) | no | yes (tints texel) |
/// | `texture_coords` | no | no | no | yes | yes |
/// | `shading_mode` | no | — | yes (selects shader) | no² | yes (selects shader) |
/// | `texture_mode` | no | yes (selects path) | yes (selects path) | yes | yes |
///
/// ¹ For `ShadingMode::None`, `Engine::update` fills `vertex_colors` with
/// `color` at every vertex, so the two are interchangeable in that path.
///
/// ² `Replace` ignores lighting entirely, so `shading_mode` has no visible
/// effect when this texture mode is active.
///
/// # Field notes
///
/// * **`points`** — three [`ScreenVertex`] values, one per triangle
///   corner. Each carries a pixel-space `position` and the clip-space `w`
///   preserved from before the perspective divide, so the rasterizer can
///   run coverage against `position` and use `1/w` for depth testing and
///   perspective-correct interpolation. See [`ScreenVertex`] for field
///   semantics and invariants. Not model-space, not world-space — already
///   projected and viewport-transformed.
/// * **`color`** — a single packed ARGB color. Used for wireframe lines and
///   as the fill color when no lighting is applied.
/// * **`vertex_colors`** — three packed ARGB colors, one per vertex. The
///   engine bakes the directional light into these during `update()` — for
///   `Flat` shading all three entries are identical; for `Gouraud` each is
///   lit independently at its vertex. The rasterizer interpolates them via
///   barycentric coordinates.
/// * **`texture_coords`** — three `(u, v)` pairs, one per vertex. Only read
///   when `texture_mode` is `Replace` or `Modulate`. Interpolated
///   perspective-correctly inside the shader.
/// * **`shading_mode`** — how `vertex_colors` was computed. The rasterizer
///   uses it to pick between `FlatShader` and `GouraudShader` on the
///   untextured path.
/// * **`texture_mode`** — whether a texture is sampled, and how its sample
///   combines with `vertex_colors`. Drives the main shader selection in
///   `fill_triangle`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    /// Per-vertex screen-space positions plus clip-space `w`.
    /// See [`ScreenVertex`] for field semantics and invariants.
    pub points: [ScreenVertex; 3],
    /// Packed ARGB. Used for wireframe lines and unlit filled triangles.
    pub color: u32,
    /// Per-vertex lit colors. Populated by `Engine::update` from the
    /// directional light according to `shading_mode`.
    pub vertex_colors: [u32; 3],
    /// Per-vertex UVs. Only read when `texture_mode` samples a texture.
    pub texture_coords: [Vec2; 3],
    /// How `vertex_colors` was lit. Selects the untextured shader.
    pub shading_mode: ShadingMode,
    /// How a texture sample (if any) combines with `vertex_colors`.
    /// Drives top-level shader dispatch.
    pub texture_mode: TextureMode,
}

impl Triangle {
    pub fn new(
        points: [ScreenVertex; 3],
        color: u32,
        vertex_colors: [u32; 3],
        texture_coords: [Vec2; 3],
        shading_mode: ShadingMode,
        texture_mode: TextureMode,
    ) -> Self {
        Self {
            points,
            color,
            vertex_colors,
            texture_coords,
            shading_mode,
            texture_mode,
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
    fn fill_triangle(
        &self,
        triangle: &Triangle,
        buffer: &mut FrameBuffer,
        color: u32,
        texture: Option<&Texture>,
    );
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
    fn fill_triangle(
        &self,
        triangle: &Triangle,
        buffer: &mut FrameBuffer,
        color: u32,
        texture: Option<&Texture>,
    ) {
        match self.active {
            RasterizerType::Scanline => self
                .scanline
                .fill_triangle(triangle, buffer, color, texture),
            RasterizerType::EdgeFunction => self
                .edge_function
                .fill_triangle(triangle, buffer, color, texture),
        }
    }
}
