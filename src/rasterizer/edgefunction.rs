use super::{Rasterizer, Triangle};
use crate::math::vec3::Vec3;

/// Triangle rasterizer using the edge function algorithm.
///
/// For each pixel in the triangle's bounding box, evaluates three edge functions.
/// A pixel is inside the triangle if all edge functions have the same sign.
/// This approach is simple, parallelizable, and forms the basis for GPU rasterization.
pub struct EdgeFunctionRasterizer;

impl EdgeFunctionRasterizer {
    pub fn new() -> Self {
        EdgeFunctionRasterizer {}
    }

    /// Compute the edge function value for point p relative to edge (a -> b).
    ///
    /// Returns a positive value if p is to the left of the edge (counter-clockwise),
    /// negative if to the right (clockwise), and zero if exactly on the edge.
    /// This is the 2D cross product: (b - a) × (p - a)
    #[inline]
    fn edge_function(a: Vec3, b: Vec3, p: Vec3) -> f32 {
        (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x)
    }
}

impl Rasterizer for EdgeFunctionRasterizer {
    fn fill_triangle(
        &self,
        triangle: &Triangle,
        buffer: &mut crate::framebuffer::FrameBuffer,
        color: u32,
    ) {
        let v0 = triangle.points[0];
        let v1 = triangle.points[1];
        let v2 = triangle.points[2];

        // Compute bounding box, clamped to buffer bounds
        let min_x = v0.x.min(v1.x).min(v2.x).floor() as i32;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil() as i32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor() as i32;
        let max_y = v0.y.max(v1.y).max(v2.y).ceil() as i32;

        let min_x = min_x.max(0);
        let max_x = max_x.min(buffer.width() as i32 - 1);
        let min_y = min_y.max(0);
        let max_y = max_y.min(buffer.height() as i32 - 1);

        // Check triangle winding by computing signed area
        // Positive = counter-clockwise, negative = clockwise
        let area = Self::edge_function(v0, v1, v2);

        // Skip degenerate triangles (zero area)
        if area.abs() < f32::EPSILON {
            return;
        }

        // Iterate over all pixels in the bounding box
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Sample at pixel center
                let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);

                // Compute edge functions for all three edges
                let w0 = Self::edge_function(v1, v2, p);
                let w1 = Self::edge_function(v2, v0, p);
                let w2 = Self::edge_function(v0, v1, p);

                // Point is inside if all edge functions have the same sign as the triangle area
                // This handles both CW and CCW wound triangles
                let inside = if area > 0.0 {
                    w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
                } else {
                    w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
                };

                if inside {
                    buffer.set_pixel(x, y, color);
                }
            }
        }
    }
}
