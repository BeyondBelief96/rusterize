//! Edge function-based triangle rasterization.
//!
//! This module implements triangle rasterization using the edge function algorithm,
//! which is the foundation of modern GPU rasterization. The algorithm tests each
//! pixel against three edge equations to determine triangle coverage.
//!
//! # Algorithm Overview
//!
//! The edge function algorithm works by:
//! 1. Computing a bounding box around the triangle
//! 2. For each pixel in the bounding box, evaluating three edge functions
//! 3. A pixel is inside the triangle if all edge functions have the same sign
//!
//! # Edge Function
//!
//! For an edge from point A to point B, the edge function at point P is:
//!
//! ```text
//! E(P) = (P.x - A.x) * (B.y - A.y) - (P.y - A.y) * (B.x - A.x)
//! ```
//!
//! This is equivalent to the 2D cross product (P - A) × (B - A), which gives:
//! - Positive value: P is to the left of edge AB
//! - Negative value: P is to the right of edge AB
//! - Zero: P is exactly on the edge
//!
//! # Barycentric Coordinates
//!
//! The edge function values are proportional to barycentric coordinates:
//!
//! ```text
//! lambda_i = E_i(P) / (E_0 + E_1 + E_2)
//! ```
//!
//! Where E_i is the edge function for the edge opposite to vertex i.
//! These coordinates are used for attribute interpolation (colors, UVs, etc.).
//!
//! # Winding Order
//!
//! The algorithm handles both clockwise and counter-clockwise triangles by
//! checking the sign of the total signed area. For CW triangles, all edge
//! functions will be negative for interior points; for CCW, all positive.
//!
//! # Top-Left Fill Rule
//!
//! To avoid double-drawing pixels on shared edges between adjacent triangles,
//! we implement the top-left fill convention (same as GPUs):
//!
//! - A pixel exactly on an edge is only drawn if that edge is a "top" or "left" edge
//! - **Top edge**: Horizontal edge where the triangle extends below (edge points left)
//! - **Left edge**: Non-horizontal edge that goes upward (in screen space, dy < 0)
//!
//! This ensures each pixel on a shared edge is owned by exactly one triangle.
//!
//! # References
//!
//! - Juan Pineda, "A Parallel Algorithm for Polygon Rasterization" (1988)
//! - Scratchapixel: <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation>

use super::shader::{FlatShader, GouraudShader, PixelShader};
use super::{Rasterizer, Triangle};
use crate::engine::TextureMode;
use crate::math::vec3::Vec3;
use crate::render::framebuffer::FrameBuffer;
use crate::render::rasterizer::shader::{
    PerspectiveCorrectTextureModulateShader, PerspectiveCorrectTextureShader,
};
use crate::texture::Texture;
use crate::ShadingMode;

/// Triangle rasterizer using the edge function algorithm.
///
/// This rasterizer iterates over all pixels in the triangle's bounding box
/// and uses edge functions to determine which pixels are inside the triangle.
/// It supports both flat shading (single color) and Gouraud shading (per-vertex
/// color interpolation using barycentric coordinates).
///
/// # Characteristics
///
/// - **Simplicity**: Easy to understand and implement
/// - **Parallelizable**: Each pixel can be evaluated independently (GPU-friendly)
/// - **Accurate**: Handles all triangle orientations and edge cases
/// - **Flexible**: Natural support for attribute interpolation via barycentric coords
///
/// # Performance Considerations
///
/// The bounding box approach means we test many pixels outside the triangle,
/// especially for thin/elongated triangles. More sophisticated implementations
/// use hierarchical testing or tile-based approaches to reduce wasted work.
pub struct EdgeFunctionRasterizer;

impl EdgeFunctionRasterizer {
    /// Creates a new edge function rasterizer instance.
    pub fn new() -> Self {
        EdgeFunctionRasterizer {}
    }

    /// Computes the edge function value for point P relative to edge (A -> B).
    ///
    /// The edge function is the signed area of the parallelogram formed by
    /// vectors (B - A) and (P - A), computed as their 2D cross product:
    ///
    /// ```text
    /// E(P) = (B.x - A.x) * (P.y - A.y) - (B.y - A.y) * (P.x - A.x)
    /// ```
    ///
    /// # Returns
    ///
    /// - Positive: P is to the right of edge AB
    /// - Negative: P is to the left of edge AB
    /// - Zero: P lies exactly on the edge AB
    ///
    /// # Arguments
    ///
    /// * `a` - Start point of the edge
    /// * `b` - End point of the edge
    /// * `p` - Point to test against the edge
    #[inline]
    fn edge_function(a: Vec3, b: Vec3, p: Vec3) -> f32 {
        (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
    }

    /// Determines if an edge from A to B is a "top" or "left" edge.
    ///
    /// Used for the top-left fill rule to ensure pixels on shared edges
    /// are drawn by exactly one triangle.
    ///
    /// In screen space (Y increases downward):
    /// - **Top edge**: Horizontal (dy == 0) and pointing left (dx < 0)
    /// - **Left edge**: Going upward (dy < 0)
    ///
    /// # Arguments
    ///
    /// * `a` - Start point of the edge
    /// * `b` - End point of the edge
    #[inline]
    fn is_top_left(a: Vec3, b: Vec3) -> bool {
        let dx = b.x - a.x;
        let dy = b.y - a.y;

        // Top edge: horizontal and pointing left
        // Left edge: going upward (dy < 0 in screen space)
        (dy == 0.0 && dx < 0.0) || dy < 0.0
    }

    /// Rasterize a triangle using the provided pixel shader.
    ///
    /// This method handles all the common rasterization logic:
    /// - Bounding box computation and clipping
    /// - Edge function evaluation
    /// - Inside/outside testing
    /// - Barycentric coordinate calculation
    /// - Depth interpolation and testing
    ///
    /// The shader is called for each pixel inside the triangle to compute
    /// the final color. Depth testing uses interpolated 1/w values.
    ///
    /// # Arguments
    /// * `v0, v1, v2` - Triangle vertices where x,y are screen coords and z stores clip-space W
    /// * `buffer` - Framebuffer with color and depth buffers
    /// * `shader` - Pixel shader for color computation
    fn rasterize_with_shader<S: PixelShader>(
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        buffer: &mut FrameBuffer,
        shader: &S,
    ) {
        // Precompute 1/w for each vertex (z component stores clip-space W)
        // These can be linearly interpolated in screen space (1/ z)
        let inv_w0 = 1.0 / v0.z;
        let inv_w1 = 1.0 / v1.z;
        let inv_w2 = 1.0 / v2.z;
        // ─────────────────────────────────────────────────────────────────────
        // Step 1: Compute bounding box
        // ─────────────────────────────────────────────────────────────────────
        let min_x = v0.x.min(v1.x).min(v2.x).floor() as i32;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil() as i32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor() as i32;
        let max_y = v0.y.max(v1.y).max(v2.y).ceil() as i32;

        // Clip to framebuffer bounds
        let min_x = min_x.max(0);
        let max_x = max_x.min(buffer.width() as i32 - 1);
        let min_y = min_y.max(0);
        let max_y = max_y.min(buffer.height() as i32 - 1);

        // ─────────────────────────────────────────────────────────────────────
        // Step 2: Compute signed area (2x triangle area)
        // ─────────────────────────────────────────────────────────────────────
        let area = Self::edge_function(v0, v1, v2);
        if area.abs() < f32::EPSILON {
            return; // Degenerate triangle
        }
        let inv_area = 1.0 / area;

        // ─────────────────────────────────────────────────────────────────────
        // Step 3: Compute top-left edge biases
        // ─────────────────────────────────────────────────────────────────────
        // For the top-left fill rule, we add a small bias to non-top-left edges
        // so that pixels exactly on those edges are excluded (w + bias < 0 or > 0).
        // Top-left edges get no bias, so pixels on them are included.
        //
        // Edge 0: v1 -> v2 (opposite to v0)
        // Edge 1: v2 -> v0 (opposite to v1)
        // Edge 2: v0 -> v1 (opposite to v2)
        let bias0: f32 = if Self::is_top_left(v1, v2) { 0.0 } else { -1.0 };
        let bias1: f32 = if Self::is_top_left(v2, v0) { 0.0 } else { -1.0 };
        let bias2: f32 = if Self::is_top_left(v0, v1) { 0.0 } else { -1.0 };

        // ─────────────────────────────────────────────────────────────────────
        // Step 4: Iterate over all pixels in bounding box
        // ─────────────────────────────────────────────────────────────────────
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Sample at pixel center
                let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);

                // Compute edge functions
                let w0 = Self::edge_function(v1, v2, p);
                let w1 = Self::edge_function(v2, v0, p);
                let w2 = Self::edge_function(v0, v1, p);

                // Inside test with top-left rule (handles both CW and CCW winding)
                // The bias shifts the decision boundary for non-top-left edges
                let inside = if area > 0.0 {
                    // CCW winding: positive edge functions for interior
                    (w0 + bias0) >= 0.0 && (w1 + bias1) >= 0.0 && (w2 + bias2) >= 0.0
                } else {
                    // CW winding: negative edge functions for interior
                    // Flip bias sign for CW triangles
                    (w0 - bias0) <= 0.0 && (w1 - bias1) <= 0.0 && (w2 - bias2) <= 0.0
                };

                if inside {
                    // Compute barycentric coordinates (use original w values, not biased)
                    let lambda = [w0 * inv_area, w1 * inv_area, w2 * inv_area];

                    // Interpolate 1/w for depth testing (linear in screen space)
                    let depth = lambda[0] * inv_w0 + lambda[1] * inv_w1 + lambda[2] * inv_w2;

                    // Delegate to shader for color computation
                    let color = shader.shade(lambda);
                    buffer.set_pixel_with_depth(x, y, depth, color);
                }
            }
        }
    }
}

impl Default for EdgeFunctionRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer for EdgeFunctionRasterizer {
    /// Fills a triangle using the edge function algorithm with shader-based coloring.
    ///
    /// This method selects the appropriate pixel shader based on texture_mode and
    /// shading_mode, then delegates to `rasterize_with_shader` for the actual
    /// rasterization work.
    ///
    /// # Shader Selection
    ///
    /// | texture_mode | shading_mode | Shader Used |
    /// |--------------|--------------|-------------|
    /// | Replace | * | TextureShader |
    /// | Modulate | * | TextureModulateShader |
    /// | None | Gouraud | GouraudShader |
    /// | None | Flat/None | FlatShader |
    fn fill_triangle(
        &self,
        triangle: &Triangle,
        buffer: &mut FrameBuffer,
        color: u32,
        texture: Option<&Texture>,
    ) {
        let [v0, v1, v2] = triangle.points;

        // Select shader based on texture_mode and shading_mode
        match (triangle.texture_mode, texture) {
            // Textured paths (when texture is available)
            (TextureMode::Replace, Some(tex)) => {
                let shader = PerspectiveCorrectTextureShader::new(
                    tex,
                    triangle.texture_coords,
                    triangle.points,
                );
                Self::rasterize_with_shader(v0, v1, v2, buffer, &shader);
            }
            (TextureMode::Modulate, Some(tex)) => {
                let shader = PerspectiveCorrectTextureModulateShader::new(
                    tex,
                    triangle.texture_coords,
                    triangle.points,
                    triangle.vertex_colors,
                );
                Self::rasterize_with_shader(v0, v1, v2, buffer, &shader);
            }

            // Non-textured paths (texture_mode is None, or no texture loaded)
            _ => match triangle.shading_mode {
                ShadingMode::Gouraud => {
                    let shader = GouraudShader::new(triangle.vertex_colors);
                    Self::rasterize_with_shader(v0, v1, v2, buffer, &shader);
                }
                ShadingMode::Flat | ShadingMode::None => {
                    let shader = FlatShader::new(color);
                    Self::rasterize_with_shader(v0, v1, v2, buffer, &shader);
                }
            },
        }
    }
}
