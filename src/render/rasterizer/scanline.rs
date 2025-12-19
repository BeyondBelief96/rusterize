//! Scanline-based triangle rasterization.
//!
//! This module implements triangle rasterization using the classic scanline algorithm
//! with flat-top/flat-bottom triangle decomposition. This approach was widely used
//! in early software renderers and remains an efficient choice for CPU-based rendering.
//!
//! # Algorithm Overview
//!
//! The scanline algorithm processes triangles one horizontal line at a time:
//!
//! 1. **Sort vertices** by Y coordinate (top to bottom in screen space)
//! 2. **Decompose** the triangle into simpler shapes (flat-top and/or flat-bottom)
//! 3. **Rasterize** each scanline by computing left/right edge intersections
//!
//! # Triangle Decomposition
//!
//! Any triangle can be decomposed into at most two simpler triangles:
//!
//! ```text
//!        v0                   v0
//!        /\                   /\
//!       /  \                 /  \
//!      /    \       =>      /----\<- split at v1.y
//!     /      \             v1   split
//!    /________\             \    /
//!   v1        v2             \  /
//!                             \/
//!                             v2
//!
//!   General triangle      Flat-bottom (top) + Flat-top (bottom)
//! ```
//!
//! Special cases (already flat-top or flat-bottom) require no splitting.
//!
//! # Inverse Slope Method
//!
//! For each scanline, we track the X position along the left and right edges.
//! Rather than computing X from Y each time, we use "inverse slopes":
//!
//! ```text
//! inv_slope = dx / dy = (x_end - x_start) / (y_end - y_start)
//! ```
//!
//! For each scanline: `x = x_start + inv_slope * (y - y_start)`
//!
//! # Gouraud Shading
//!
//! For smooth shading, we perform bilinear interpolation:
//! 1. Interpolate colors along the left and right edges (using Y progress)
//! 2. Interpolate between left/right colors across the scanline (using X progress)
//!
//! This is mathematically equivalent to barycentric interpolation but decomposed
//! into two sequential 1D interpolations, which is more natural for scanline traversal.
//!
//! # Comparison with Edge Function Rasterization
//!
//! | Aspect | Scanline | Edge Function |
//! |--------|----------|---------------|
//! | Approach | Process rows sequentially | Test each pixel independently |
//! | Memory access | Cache-friendly (row by row) | Random within bounding box |
//! | Parallelism | Limited (row-level) | Highly parallel (pixel-level) |
//! | Thin triangles | Efficient (only touches covered pixels) | Wasteful (tests empty bounding box) |
//! | Complexity | More code, edge cases | Simpler, uniform |
//!
//! # References
//!
//! - Foley, van Dam et al., "Computer Graphics: Principles and Practice"
//! - Abrash, Michael, "Graphics Programming Black Book"

use super::shader::{FlatShader, GouraudShader, PixelShader, TextureModulateShader, TextureShader};
use super::{Rasterizer, Triangle};
use crate::engine::TextureMode;
use crate::math::utils::{edge_function, triangle_area};
use crate::math::vec2::Vec2;
use crate::math::vec3::Vec3;
use crate::render::framebuffer::FrameBuffer;
use crate::texture::Texture;
use crate::ShadingMode;

/// Compute barycentric coordinates for point p in triangle (v0, v1, v2).
///
/// Uses precomputed inverse area for efficiency when rasterizing many pixels.
/// Returns [λ0, λ1, λ2] where each λ represents the weight of the
/// corresponding vertex. These sum to 1.0 for points inside the triangle.
#[inline]
fn barycentric(v0: Vec2, v1: Vec2, v2: Vec2, p: Vec2, inv_area: f32) -> [f32; 3] {
    let w0 = edge_function(v1, v2, p);
    let w1 = edge_function(v2, v0, p);
    let w2 = edge_function(v0, v1, p);
    [w0 * inv_area, w1 * inv_area, w2 * inv_area]
}

/// Scanline-based triangle rasterizer.
///
/// This rasterizer uses the classic flat-top/flat-bottom decomposition approach,
/// processing triangles one horizontal scanline at a time. It supports both
/// flat shading and Gouraud (smooth) shading with per-vertex color interpolation.
///
/// # Characteristics
///
/// - **Cache-friendly**: Processes pixels in row order, good for memory locality
/// - **Efficient for thin triangles**: Only visits pixels actually covered
/// - **Sequential**: Best suited for single-threaded CPU rendering
///
/// # Implementation Notes
///
/// The rasterizer handles vertex sorting internally, so input triangles can have
/// vertices in any order. When Gouraud shading is enabled, vertex colors are
/// sorted alongside vertices to maintain correct attribute correspondence.
pub struct ScanlineRasterizer;

impl ScanlineRasterizer {
    /// Creates a new scanline rasterizer instance.
    pub fn new() -> Self {
        Self
    }

    /// Sorts three vertices by Y coordinate (ascending: top to bottom in screen space).
    ///
    /// Uses a simple 3-element bubble sort which is optimal for this small size.
    /// After sorting: `v0.y <= v1.y <= v2.y`
    ///
    /// # Arguments
    ///
    /// * `v0`, `v1`, `v2` - Mutable references to vertices to be sorted in-place
    fn sort_vertices(v0: &mut Vec3, v1: &mut Vec3, v2: &mut Vec3) {
        // Three comparisons suffice for 3 elements (bubble sort)
        if v1.y < v0.y {
            std::mem::swap(v0, v1);
        }
        if v2.y < v1.y {
            std::mem::swap(v1, v2);
        }
        if v1.y < v0.y {
            std::mem::swap(v0, v1);
        }
    }

    // =========================================================================
    // Shader-based rasterization methods
    // =========================================================================

    /// Rasterize a triangle using the provided pixel shader.
    ///
    /// This method combines scanline traversal (for efficiency) with barycentric
    /// coordinate computation (for correct attribute interpolation). The key insight
    /// is that we sort vertices for scanline traversal but compute barycentrics
    /// using the original vertex order.
    ///
    /// # Arguments
    /// * `v0, v1, v2` - Original (unsorted) triangle vertices (z stores clip-space W)
    /// * `buffer` - Framebuffer to write to
    /// * `shader` - Pixel shader for color computation
    fn rasterize_with_shader<S: PixelShader>(
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        buffer: &mut FrameBuffer,
        shader: &S,
    ) {
        // Precompute 1/w for each vertex (z component stores clip-space W)
        // These can be linearly interpolated in screen space for depth testing
        let inv_w = [1.0 / v0.z, 1.0 / v1.z, 1.0 / v2.z];

        // Convert to Vec2 for barycentric calculations (only x, y matter)
        let v0_2d = Vec2::new(v0.x, v0.y);
        let v1_2d = Vec2::new(v1.x, v1.y);
        let v2_2d = Vec2::new(v2.x, v2.y);

        // Compute area for barycentric normalization
        let area = triangle_area(v0_2d, v1_2d, v2_2d);
        if area.abs() < f32::EPSILON {
            return; // Degenerate triangle
        }
        let inv_area = 1.0 / area;

        // Sort vertices for scanline traversal
        // IMPORTANT: We sort copies, keeping original v0, v1, v2 for barycentrics
        let mut sv0 = v0;
        let mut sv1 = v1;
        let mut sv2 = v2;
        Self::sort_vertices(&mut sv0, &mut sv1, &mut sv2);

        // Check triangle type and call appropriate fill method
        if (sv1.y - sv2.y).abs() < f32::EPSILON {
            // Flat-bottom triangle
            Self::fill_flat_bottom_with_shader(
                sv0, sv1, sv2, v0_2d, v1_2d, v2_2d, inv_w, inv_area, buffer, shader,
            );
        } else if (sv0.y - sv1.y).abs() < f32::EPSILON {
            // Flat-top triangle
            Self::fill_flat_top_with_shader(
                sv0, sv1, sv2, v0_2d, v1_2d, v2_2d, inv_w, inv_area, buffer, shader,
            );
        } else {
            // General triangle - split into flat-bottom + flat-top

            // t is the ratio of the height of the triangle from sv0 to sv1 to the total height of the triangle
            let t = (sv1.y - sv0.y) / (sv2.y - sv0.y);
            // We calculate the midpoint x coordinate by interpolating the x coordinates of sv0 and sv2 based on the ratio t
            let split_x = sv0.x + (sv2.x - sv0.x) * t;
            let split_point = Vec3::new(split_x, sv1.y, 0.0);

            // Fill top half (flat-bottom)
            Self::fill_flat_bottom_with_shader(
                sv0,
                split_point,
                sv1,
                v0_2d,
                v1_2d,
                v2_2d, // Always use original for barycentrics
                inv_w,
                inv_area,
                buffer,
                shader,
            );

            // Fill bottom half (flat-top)
            Self::fill_flat_top_with_shader(
                sv1,
                split_point,
                sv2,
                v0_2d,
                v1_2d,
                v2_2d,
                inv_w,
                inv_area,
                buffer,
                shader,
            );
        }
    }

    /// Fill a flat-bottom triangle using a pixel shader.
    ///
    /// # Arguments
    /// * `sv0, sv1, sv2` - Sorted vertices for scanline traversal
    /// * `v0, v1, v2` - Original vertices (Vec2) for barycentric computation
    /// * `inv_w` - 1/w values for each original vertex (for depth interpolation)
    /// * `inv_area` - 1/area for barycentric normalization
    fn fill_flat_bottom_with_shader<S: PixelShader>(
        sv0: Vec3, // Top vertex (sorted)
        sv1: Vec3, // Bottom-left (sorted)
        sv2: Vec3, // Bottom-right (sorted)
        v0: Vec2,  // Original vertices for barycentrics
        v1: Vec2,
        v2: Vec2,
        inv_w: [f32; 3], // 1/w for each original vertex
        inv_area: f32,
        buffer: &mut FrameBuffer,
        shader: &S,
    ) {
        let height = sv1.y - sv0.y;
        if height.abs() < f32::EPSILON {
            return;
        }

        let inv_slope_1 = (sv1.x - sv0.x) / height;
        let inv_slope_2 = (sv2.x - sv0.x) / height;

        let y_start = sv0.y.ceil() as i32;
        let y_end = sv1.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - sv0.y;
            let x1 = sv0.x + inv_slope_1 * dy;
            let x2 = sv0.x + inv_slope_2 * dy;

            let (x_left, x_right) = if x1 < x2 { (x1, x2) } else { (x2, x1) };

            let x_start = x_left.ceil() as i32;
            let x_end = x_right.floor() as i32;

            for x in x_start..=x_end {
                // Compute barycentric coords using ORIGINAL vertices
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let lambda = barycentric(v0, v1, v2, p, inv_area);

                // Interpolate 1/w for depth testing (linear in screen space)
                let depth = lambda[0] * inv_w[0] + lambda[1] * inv_w[1] + lambda[2] * inv_w[2];

                let color = shader.shade(lambda);
                buffer.set_pixel_with_depth(x, y, depth, color);
            }
        }
    }

    /// Fill a flat-top triangle using a pixel shader.
    ///
    /// # Arguments
    /// * `sv0, sv1, sv2` - Sorted vertices for scanline traversal
    /// * `v0, v1, v2` - Original vertices (Vec2) for barycentric computation
    /// * `inv_w` - 1/w values for each original vertex (for depth interpolation)
    /// * `inv_area` - 1/area for barycentric normalization
    fn fill_flat_top_with_shader<S: PixelShader>(
        sv0: Vec3, // Top-left (sorted)
        sv1: Vec3, // Top-right (sorted)
        sv2: Vec3, // Bottom vertex (sorted)
        v0: Vec2,  // Original vertices for barycentrics
        v1: Vec2,
        v2: Vec2,
        inv_w: [f32; 3], // 1/w for each original vertex
        inv_area: f32,
        buffer: &mut FrameBuffer,
        shader: &S,
    ) {
        let height = sv2.y - sv0.y;
        if height.abs() < f32::EPSILON {
            return;
        }

        let inv_slope_1 = (sv2.x - sv0.x) / height;
        let inv_slope_2 = (sv2.x - sv1.x) / height;

        let y_start = sv0.y.ceil() as i32;
        let y_end = sv2.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - sv0.y;
            let x1 = sv0.x + inv_slope_1 * dy;
            let x2 = sv1.x + inv_slope_2 * dy;

            let (x_left, x_right) = if x1 < x2 { (x1, x2) } else { (x2, x1) };

            let x_start = x_left.ceil() as i32;
            let x_end = x_right.floor() as i32;

            for x in x_start..=x_end {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let lambda = barycentric(v0, v1, v2, p, inv_area);

                // Interpolate 1/w for depth testing (linear in screen space)
                let depth = lambda[0] * inv_w[0] + lambda[1] * inv_w[1] + lambda[2] * inv_w[2];

                let color = shader.shade(lambda);
                buffer.set_pixel_with_depth(x, y, depth, color);
            }
        }
    }
}

impl Default for ScanlineRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer for ScanlineRasterizer {
    /// Fills a triangle using the scanline algorithm with pixel shaders.
    ///
    /// This implementation uses the PixelShader trait to handle different shading
    /// and texturing modes. The scanline traversal is combined with barycentric
    /// coordinate computation for correct attribute interpolation.
    ///
    /// # Shader Selection
    ///
    /// The shader is selected based on texture mode and shading mode:
    /// - Texture Replace: TextureShader (texture color only)
    /// - Texture Modulate: TextureModulateShader (texture * lighting)
    /// - Gouraud: GouraudShader (interpolated vertex colors)
    /// - Flat/None: FlatShader (single color)
    ///
    /// # Arguments
    ///
    /// * `triangle` - Triangle to rasterize with vertices, colors, UVs, and modes
    /// * `buffer` - Framebuffer to write pixels to
    /// * `color` - Flat color to use (for Flat/None shading modes without texture)
    /// * `texture` - Optional texture for texture mapping modes
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
