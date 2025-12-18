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
//! This is equivalent to the 2D cross product (B - A) × (P - A), which gives:
//! - Positive value: P is to the left of edge AB (counter-clockwise)
//! - Negative value: P is to the right of edge AB (clockwise)
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
//! # References
//!
//! - Juan Pineda, "A Parallel Algorithm for Polygon Rasterization" (1988)
//! - Scratchapixel: <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation>

use super::shader::{
    FlatShader, GouraudShader, PixelShader, TextureModulateShader, TextureShader,
};
use super::{Rasterizer, Triangle};
use crate::engine::TextureMode;
use crate::math::vec3::Vec3;
use crate::render::framebuffer::FrameBuffer;
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
    /// E(P) = (P.x - A.x) * (B.y - A.y) - (P.y - A.y) * (B.x - A.x)
    /// ```
    ///
    /// # Returns
    ///
    /// - Positive: P is to the left of edge AB (counter-clockwise winding)
    /// - Negative: P is to the right of edge AB (clockwise winding)
    /// - Zero: P lies exactly on the edge AB
    ///
    /// # Arguments
    ///
    /// * `a` - Start point of the edge
    /// * `b` - End point of the edge
    /// * `p` - Point to test against the edge
    #[inline]
    fn edge_function(a: Vec3, b: Vec3, p: Vec3) -> f32 {
        (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x)
    }

    /// Rasterize a triangle using the provided pixel shader.
    ///
    /// This method handles all the common rasterization logic:
    /// - Bounding box computation and clipping
    /// - Edge function evaluation
    /// - Inside/outside testing
    /// - Barycentric coordinate calculation
    ///
    /// The shader is called for each pixel inside the triangle to compute
    /// the final color.
    fn rasterize_with_shader<S: PixelShader>(
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        buffer: &mut FrameBuffer,
        shader: &S,
    ) {
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
        // Step 3: Iterate over all pixels in bounding box
        // ─────────────────────────────────────────────────────────────────────
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Sample at pixel center
                let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);

                // Compute edge functions
                let w0 = Self::edge_function(v1, v2, p);
                let w1 = Self::edge_function(v2, v0, p);
                let w2 = Self::edge_function(v0, v1, p);

                // Inside test (handles both CW and CCW winding)
                let inside = if area > 0.0 {
                    w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
                } else {
                    w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
                };

                if inside {
                    // Compute barycentric coordinates
                    let lambda = [w0 * inv_area, w1 * inv_area, w2 * inv_area];

                    // Delegate to shader for color computation
                    let color = shader.shade(lambda);
                    buffer.set_pixel(x, y, color);
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
                let shader = TextureShader::new(tex, triangle.texture_coords);
                Self::rasterize_with_shader(v0, v1, v2, buffer, &shader);
            }
            (TextureMode::Modulate, Some(tex)) => {
                let shader = TextureModulateShader::new(
                    tex,
                    triangle.texture_coords,
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
