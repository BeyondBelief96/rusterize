use crate::framebuffer::FrameBuffer;
use crate::rasterizer::Triangle;

pub(crate) const COLOR_BACKGROUND: u32 = 0xFF1E1E1E;
pub(crate) const COLOR_GRID: u32 = 0xFF333333;

pub(crate) struct Renderer {
    color_buffer: Vec<u32>,
    width: u32,
    height: u32,
}

impl Renderer {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            color_buffer: vec![COLOR_BACKGROUND; size],
            width,
            height,
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let size = (width * height) as usize;
        self.color_buffer = vec![COLOR_BACKGROUND; size];
        self.width = width;
        self.height = height;
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn clear(&mut self, color: u32) {
        self.color_buffer.fill(color);
    }

    pub(crate) fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = (y as u32 * self.width + x as u32) as usize;
            self.color_buffer[index] = color;
        }
    }

    pub(crate) fn draw_grid(&mut self, spacing: i32, color: u32) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                if x % spacing == 0 || y % spacing == 0 {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    pub(crate) fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub(crate) fn draw_triangle_wireframe(&mut self, triangle: &Triangle, color: u32) {
        let [p0, p1, p2] = triangle.points;

        self.draw_line(p0.x as i32, p0.y as i32, p1.x as i32, p1.y as i32, color);
        self.draw_line(p1.x as i32, p1.y as i32, p2.x as i32, p2.y as i32, color);
        self.draw_line(p2.x as i32, p2.y as i32, p0.x as i32, p0.y as i32, color);
    }

    /// Draws a line between two points using Bresenham's line algorithm.
    ///
    /// Bresenham's algorithm efficiently determines which pixels to illuminate
    /// by using only integer arithmetic. It works by tracking an "error" term
    /// that represents the distance between the ideal line and the current pixel.
    ///
    /// The key insight: for each step along the major axis (the axis with greater
    /// distance), we decide whether to also step along the minor axis based on
    /// accumulated error. When the error exceeds a threshold, we step diagonally
    /// instead of straight.
    pub(crate) fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        // Calculate the absolute distances in each axis.
        // These represent how far we need to travel horizontally and vertically.
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();

        // Determine the step direction for each axis.
        // +1 if we're moving in the positive direction, -1 if negative.
        // This allows the algorithm to work for lines in any direction.
        let x_incr_direction = if x0 < x1 { 1 } else { -1 };
        let y_incr_direction = if y0 < y1 { 1 } else { -1 };

        // The error term tracks how far off we are from the ideal line.
        // Initialized to dx - dy, which balances the algorithm for lines
        // of any slope. A positive error favors x movement, negative favors y.
        let mut err = dx - dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            self.set_pixel(x, y, color);

            // Check if we've reached the destination
            if x == x1 && y == y1 {
                break;
            }

            // Double the error for comparison (avoids floating point).
            // We compare against -dy and dx to decide movement direction.
            let e2 = 2 * err;

            // If e2 > -dy, the error has accumulated enough that we should
            // step in x. We then subtract dy from err to "pay back" the error
            // we've accumulated by not stepping in y.
            if e2 > -dy {
                err -= dy;
                x += x_incr_direction;
            }

            // If e2 < dx, we should also step in y. We add dx to err because
            // stepping in y reduces our deviation from the ideal line.
            // Note: both conditions can be true, resulting in a diagonal step.
            if e2 < dx {
                err += dx;
                y += y_incr_direction;
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn draw_line_dda(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = x1 - x0;
        let dy = y1 - y0;

        let mut side_length = dx.abs();
        if dy.abs() > side_length {
            side_length = dy.abs();
        }

        let x_increment = dx as f32 / side_length as f32;
        let y_increment = dy as f32 / side_length as f32;
        let mut current_x = x0 as f32;
        let mut current_y = y0 as f32;

        for _ in 0..side_length {
            self.set_pixel(current_x.round() as i32, current_y.round() as i32, color);
            current_x += x_increment;
            current_y += y_increment;
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.color_buffer.as_ptr() as *const u8,
                self.color_buffer.len() * 4,
            )
        }
    }

    /// Get a mutable FrameBuffer view into the color buffer.
    pub(crate) fn as_framebuffer(&mut self) -> FrameBuffer<'_> {
        FrameBuffer::new(&mut self.color_buffer, self.width, self.height)
    }
}
