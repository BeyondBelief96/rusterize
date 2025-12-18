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

use super::{Rasterizer, Triangle};
use crate::colors::{lerp_color, pack_color, unpack_color};
use crate::engine::TextureMode;
use crate::math::vec3::Vec3;
use crate::render::framebuffer::FrameBuffer;
use crate::texture::Texture;
use crate::ShadingMode;

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

    /// Sorts vertices by Y coordinate while keeping colors synchronized.
    ///
    /// When performing Gouraud shading, vertex colors must be reordered alongside
    /// their corresponding vertices to maintain correct attribute mapping.
    ///
    /// # Arguments
    ///
    /// * `v0`, `v1`, `v2` - Mutable references to vertices
    /// * `c0`, `c1`, `c2` - Mutable references to corresponding RGB colors
    fn sort_vertices_with_colors(
        v0: &mut Vec3,
        v1: &mut Vec3,
        v2: &mut Vec3,
        c0: &mut (f32, f32, f32),
        c1: &mut (f32, f32, f32),
        c2: &mut (f32, f32, f32),
    ) {
        if v1.y < v0.y {
            std::mem::swap(v0, v1);
            std::mem::swap(c0, c1);
        }
        if v2.y < v1.y {
            std::mem::swap(v1, v2);
            std::mem::swap(c1, c2);
        }
        if v1.y < v0.y {
            std::mem::swap(v0, v1);
            std::mem::swap(c0, c1);
        }
    }

    /// Fills a flat-bottom triangle with Gouraud (smooth) shading.
    ///
    /// A flat-bottom triangle has its top vertex (v0) above two bottom vertices
    /// (v1, v2) that share the same Y coordinate.
    ///
    /// ```text
    ///        v0 (top)
    ///        /\
    ///       /  \
    ///      /    \
    ///     /______\
    ///   v1        v2  (same Y)
    /// ```
    ///
    /// # Algorithm
    ///
    /// 1. Compute inverse slopes for both edges from v0
    /// 2. For each scanline from top to bottom:
    ///    - Calculate X positions on left and right edges
    ///    - Interpolate colors along both edges using Y progress (t = dy/height)
    ///    - Fill pixels between edges, interpolating color using X progress
    ///
    /// # Arguments
    ///
    /// * `v0` - Top vertex
    /// * `v1`, `v2` - Bottom vertices (must have same Y coordinate)
    /// * `c0`, `c1`, `c2` - RGB colors corresponding to each vertex
    /// * `buffer` - Framebuffer to write pixels to
    fn fill_flat_bottom_gouraud(
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        c0: (f32, f32, f32),
        c1: (f32, f32, f32),
        c2: (f32, f32, f32),
        buffer: &mut FrameBuffer,
    ) {
        let height = v1.y - v0.y;
        if height.abs() < f32::EPSILON {
            return; // Degenerate triangle (zero height)
        }

        // Compute inverse slopes (change in X per unit Y)
        // These tell us how much X changes as we move down one scanline
        let inv_slope_1 = (v1.x - v0.x) / height; // Slope of edge v0 -> v1
        let inv_slope_2 = (v2.x - v0.x) / height; // Slope of edge v0 -> v2

        // Determine scanline range (use ceil/floor for proper pixel coverage)
        let y_start = v0.y.ceil() as i32;
        let y_end = v1.y.floor() as i32;

        for y in y_start..=y_end {
            // Calculate vertical progress through the triangle (0 at top, 1 at bottom)
            let dy = y as f32 - v0.y;
            let t = dy / height;

            // Calculate X positions on each edge at this scanline
            let x1 = v0.x + inv_slope_1 * dy;
            let x2 = v0.x + inv_slope_2 * dy;

            // Interpolate colors along edges using vertical progress
            let color1 = lerp_color(c0, c1, t); // Color on edge v0 -> v1
            let color2 = lerp_color(c0, c2, t); // Color on edge v0 -> v2

            // Determine which edge is left vs right (may vary per triangle)
            let (x_left, x_right, c_left, c_right) = if x1 < x2 {
                (x1, x2, color1, color2)
            } else {
                (x2, x1, color2, color1)
            };

            // Fill pixels across the scanline
            let x_start = x_left.ceil() as i32;
            let x_end = x_right.floor() as i32;
            let span = x_right - x_left;

            for x in x_start..=x_end {
                // Calculate horizontal progress across the scanline (0 at left, 1 at right)
                let tx = if span.abs() < f32::EPSILON {
                    0.0
                } else {
                    (x as f32 - x_left) / span
                };

                // Interpolate color horizontally between left and right edge colors
                let color = lerp_color(c_left, c_right, tx);
                buffer.set_pixel(x, y, pack_color(color.0, color.1, color.2, 1.0));
            }
        }
    }

    /// Fills a flat-top triangle with Gouraud (smooth) shading.
    ///
    /// A flat-top triangle has two top vertices (v0, v1) sharing the same Y
    /// coordinate, above a single bottom vertex (v2).
    ///
    /// ```text
    ///   v0________v1  (same Y)
    ///     \      /
    ///      \    /
    ///       \  /
    ///        \/
    ///        v2 (bottom)
    /// ```
    ///
    /// # Algorithm
    ///
    /// 1. Compute inverse slopes for edges from each top vertex to bottom
    /// 2. For each scanline from top to bottom:
    ///    - Calculate X positions on left and right edges
    ///    - Interpolate colors along both edges using Y progress
    ///    - Fill pixels between edges, interpolating color using X progress
    ///
    /// # Arguments
    ///
    /// * `v0`, `v1` - Top vertices (must have same Y coordinate)
    /// * `v2` - Bottom vertex
    /// * `c0`, `c1`, `c2` - RGB colors corresponding to each vertex
    /// * `buffer` - Framebuffer to write pixels to
    fn fill_flat_top_gouraud(
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        c0: (f32, f32, f32),
        c1: (f32, f32, f32),
        c2: (f32, f32, f32),
        buffer: &mut FrameBuffer,
    ) {
        let height = v2.y - v0.y;
        if height.abs() < f32::EPSILON {
            return; // Degenerate triangle
        }

        // Compute inverse slopes from top vertices to bottom vertex
        let inv_slope_1 = (v2.x - v0.x) / height; // Edge v0 -> v2
        let inv_slope_2 = (v2.x - v1.x) / height; // Edge v1 -> v2

        let y_start = v0.y.ceil() as i32;
        let y_end = v2.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - v0.y;
            let t = dy / height;

            // X positions along each edge
            let x1 = v0.x + inv_slope_1 * dy;
            let x2 = v1.x + inv_slope_2 * dy;

            // Interpolate colors along edges (both converging to c2 at bottom)
            let color1 = lerp_color(c0, c2, t);
            let color2 = lerp_color(c1, c2, t);

            let (x_left, x_right, c_left, c_right) = if x1 < x2 {
                (x1, x2, color1, color2)
            } else {
                (x2, x1, color2, color1)
            };

            let x_start = x_left.ceil() as i32;
            let x_end = x_right.floor() as i32;
            let span = x_right - x_left;

            for x in x_start..=x_end {
                let tx = if span.abs() < f32::EPSILON {
                    0.0
                } else {
                    (x as f32 - x_left) / span
                };
                let color = lerp_color(c_left, c_right, tx);
                buffer.set_pixel(x, y, pack_color(color.0, color.1, color.2, 1.0));
            }
        }
    }

    /// Fills a flat-bottom triangle with a solid color (no interpolation).
    ///
    /// This is the optimized path for flat shading where all pixels receive
    /// the same color. Uses `fill_scanline` for efficient horizontal fills.
    ///
    /// # Arguments
    ///
    /// * `v0` - Top vertex
    /// * `v1`, `v2` - Bottom vertices (same Y coordinate)
    /// * `buffer` - Framebuffer to write to
    /// * `color` - Solid color for all pixels (ARGB format)
    fn fill_flat_bottom_solid(v0: Vec3, v1: Vec3, v2: Vec3, buffer: &mut FrameBuffer, color: u32) {
        let height = v1.y - v0.y;
        if height.abs() < f32::EPSILON {
            return;
        }

        let inv_slope_1 = (v1.x - v0.x) / height;
        let inv_slope_2 = (v2.x - v0.x) / height;

        let y_start = v0.y.ceil() as i32;
        let y_end = v1.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - v0.y;
            let x1 = v0.x + inv_slope_1 * dy;
            let x2 = v0.x + inv_slope_2 * dy;

            // Use min/max to handle either edge being left or right
            let x_left = x1.min(x2).ceil() as i32;
            let x_right = x1.max(x2).floor() as i32;

            // fill_scanline is optimized for horizontal runs of solid color
            buffer.fill_scanline(y, x_left, x_right, color);
        }
    }

    /// Fills a flat-top triangle with a solid color (no interpolation).
    ///
    /// # Arguments
    ///
    /// * `v0`, `v1` - Top vertices (same Y coordinate)
    /// * `v2` - Bottom vertex
    /// * `buffer` - Framebuffer to write to
    /// * `color` - Solid color for all pixels (ARGB format)
    fn fill_flat_top_solid(v0: Vec3, v1: Vec3, v2: Vec3, buffer: &mut FrameBuffer, color: u32) {
        let height = v2.y - v0.y;
        if height.abs() < f32::EPSILON {
            return;
        }

        let inv_slope_1 = (v2.x - v0.x) / height;
        let inv_slope_2 = (v2.x - v1.x) / height;

        let y_start = v0.y.ceil() as i32;
        let y_end = v2.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - v0.y;
            let x1 = v0.x + inv_slope_1 * dy;
            let x2 = v1.x + inv_slope_2 * dy;

            let x_left = x1.min(x2).ceil() as i32;
            let x_right = x1.max(x2).floor() as i32;

            buffer.fill_scanline(y, x_left, x_right, color);
        }
    }
}

impl Default for ScanlineRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer for ScanlineRasterizer {
    /// Fills a triangle using the scanline algorithm.
    ///
    /// # Algorithm Steps
    ///
    /// 1. **Sort vertices** by Y coordinate (top to bottom)
    ///    - For Gouraud shading, also sort corresponding vertex colors
    ///
    /// 2. **Classify triangle shape**:
    ///    - If `v1.y == v2.y`: Already flat-bottom, no split needed
    ///    - If `v0.y == v1.y`: Already flat-top, no split needed
    ///    - Otherwise: General triangle, needs splitting
    ///
    /// 3. **Split general triangles** at the middle vertex's Y level:
    ///    ```text
    ///    split_point.x = v0.x + (v2.x - v0.x) * t
    ///    where t = (v1.y - v0.y) / (v2.y - v0.y)
    ///    ```
    ///    The split point lies on edge v0->v2 at the same Y as v1.
    ///
    /// 4. **Rasterize sub-triangles**:
    ///    - Flat-bottom: v0 (top) to v1 and split_point (bottom)
    ///    - Flat-top: v1 and split_point (top) to v2 (bottom)
    ///
    /// # Arguments
    ///
    /// * `triangle` - Triangle to rasterize with vertices, colors, and shading mode
    /// * `buffer` - Framebuffer to write pixels to
    /// * `color` - Flat color to use (for Flat/None shading modes)
    fn fill_triangle(
        &self,
        triangle: &Triangle,
        buffer: &mut FrameBuffer,
        color: u32,
        texture: Option<&Texture>,
    ) {
        let mut v0 = triangle.points[0];
        let mut v1 = triangle.points[1];
        let mut v2 = triangle.points[2];

        match triangle.shading_mode {
            ShadingMode::Gouraud => {
                // ─────────────────────────────────────────────────────────────
                // Gouraud shading: interpolate colors across the triangle
                // ─────────────────────────────────────────────────────────────

                // Unpack vertex colors and sort alongside vertices
                let mut c0 = unpack_color(triangle.vertex_colors[0]);
                let mut c1 = unpack_color(triangle.vertex_colors[1]);
                let mut c2 = unpack_color(triangle.vertex_colors[2]);

                Self::sort_vertices_with_colors(
                    &mut v0, &mut v1, &mut v2, &mut c0, &mut c1, &mut c2,
                );

                // Case 1: Already a flat-bottom triangle
                if (v1.y - v2.y).abs() < f32::EPSILON {
                    Self::fill_flat_bottom_gouraud(v0, v1, v2, c0, c1, c2, buffer);
                    return;
                }

                // Case 2: Already a flat-top triangle
                if (v0.y - v1.y).abs() < f32::EPSILON {
                    Self::fill_flat_top_gouraud(v0, v1, v2, c0, c1, c2, buffer);
                    return;
                }

                // Case 3: General triangle - split into flat-bottom + flat-top
                // Calculate the parameter t for the split point on edge v0->v2
                let t = (v1.y - v0.y) / (v2.y - v0.y);

                // Split point lies on edge v0->v2 at the same Y as v1
                let split_x = v0.x + (v2.x - v0.x) * t;
                let split_point = Vec3::new(split_x, v1.y, 0.0);

                // Interpolate color at the split point along edge v0->v2
                let split_color = lerp_color(c0, c2, t);

                // Fill top half (flat-bottom): v0 at apex, v1 and split at base
                Self::fill_flat_bottom_gouraud(v0, v1, split_point, c0, c1, split_color, buffer);

                // Fill bottom half (flat-top): v1 and split at top, v2 at apex
                Self::fill_flat_top_gouraud(v1, split_point, v2, c1, split_color, c2, buffer);
            }

            ShadingMode::Flat | ShadingMode::None => {
                // ─────────────────────────────────────────────────────────────
                // Flat shading: single color for entire triangle
                // ─────────────────────────────────────────────────────────────

                Self::sort_vertices(&mut v0, &mut v1, &mut v2);

                // Case 1: Already flat-bottom
                if (v1.y - v2.y).abs() < f32::EPSILON {
                    Self::fill_flat_bottom_solid(v0, v1, v2, buffer, color);
                    return;
                }

                // Case 2: Already flat-top
                if (v0.y - v1.y).abs() < f32::EPSILON {
                    Self::fill_flat_top_solid(v0, v1, v2, buffer, color);
                    return;
                }

                // Case 3: General triangle - split at v1's Y level
                let t = (v1.y - v0.y) / (v2.y - v0.y);
                let split_x = v0.x + (v2.x - v0.x) * t;
                let split_point = Vec3::new(split_x, v1.y, 0.0);

                Self::fill_flat_bottom_solid(v0, v1, split_point, buffer, color);
                Self::fill_flat_top_solid(v1, split_point, v2, buffer, color);
            }
        }
    }
}
