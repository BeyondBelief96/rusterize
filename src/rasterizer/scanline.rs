use super::{Rasterizer, Triangle};
use crate::framebuffer::FrameBuffer;
use crate::math::{vec2::Vec2, vec3::Vec3};

/// Scanline-based triangle rasterizer.
///
/// Uses the flat-top/flat-bottom triangle decomposition approach:
/// 1. Sort vertices by Y coordinate
/// 2. Split triangle into flat-top and/or flat-bottom triangles
/// 3. Rasterize each scanline from left to right
pub struct ScanlineRasterizer;

impl ScanlineRasterizer {
    pub fn new() -> Self {
        Self
    }

    /// Find the midpoint where the triangle is split into two smaller triangles.
    /// This assumes that the input points are sorted by Y coordinate already.
    fn find_triangle_split_point(&self, p0: Vec3, p1: Vec3, p2: Vec3) -> Vec3 {
        let x_slope = (p2.x - p0.x) / (p2.y - p0.y);
        let my = p1.y;
        let mx = p0.x + x_slope * (my - p0.y);
        Vec3::new(mx, my, p0.z + x_slope * (my - p0.z))
    }

    fn sort_vertices(&self, v0: &mut Vec3, v1: &mut Vec3, v2: &mut Vec3) {
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

    fn fill_flat_bottom_triangle(
        &self,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        buffer: &mut FrameBuffer,
        color: u32,
    ) {
        let inv_slope_1 = (v1.x - v0.x) / (v1.y - v0.y);
        let inv_slope_2 = (v2.x - v0.x) / (v2.y - v0.y);

        let y_start = v0.y.ceil() as i32;
        let y_end = v1.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - v0.y;
            let x1 = v0.x + inv_slope_1 * dy;
            let x2 = v0.x + inv_slope_2 * dy;
            // Don't assume which is left/right - use min/max
            let x_left = x1.min(x2).ceil() as i32;
            let x_right = x1.max(x2).floor() as i32;
            buffer.fill_scanline(y, x_left, x_right, color);
        }
    }

    fn fill_flat_top_triangle(
        &self,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        buffer: &mut FrameBuffer,
        color: u32,
    ) {
        let inv_slope_1 = (v2.x - v0.x) / (v2.y - v0.y);
        let inv_slope_2 = (v2.x - v1.x) / (v2.y - v1.y);

        let y_start = v0.y.ceil() as i32;
        let y_end = v2.y.floor() as i32;

        for y in y_start..=y_end {
            let dy = y as f32 - v0.y;
            let x1 = v0.x + inv_slope_1 * dy;
            let x2 = v1.x + inv_slope_2 * dy;
            // Don't assume which is left/right - use min/max
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
    fn fill_triangle(&self, triangle: &Triangle, buffer: &mut FrameBuffer, color: u32) {
        let mut v0 = triangle.points[0];
        let mut v1 = triangle.points[1];
        let mut v2 = triangle.points[2];

        self.sort_vertices(&mut v0, &mut v1, &mut v2);

        // Check if triangle is flat-bottom (bottom two vertices have same y)
        if (v1.y - v2.y).abs() < f32::EPSILON {
            self.fill_flat_bottom_triangle(v0, v1, v2, buffer, color);
            return;
        }

        // Check if triangle is flat-top (top two vertices have same y)
        if (v0.y - v1.y).abs() < f32::EPSILON {
            self.fill_flat_top_triangle(v0, v1, v2, buffer, color);
            return;
        }

        // General case: split into flat-bottom and flat-top triangles
        let split_point = self.find_triangle_split_point(v0, v1, v2);

        // Fill flat-bottom triangle (top half)
        self.fill_flat_bottom_triangle(v0, v1, split_point, buffer, color);
        // Fill flat-top triangle (bottom half)
        self.fill_flat_top_triangle(v1, split_point, v2, buffer, color);
    }
}
