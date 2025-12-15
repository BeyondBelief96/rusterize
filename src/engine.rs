use crate::triangle::Triangle;
use crate::mesh::{Mesh, CUBE_VERTICES, CUBE_FACES};
use crate::math::vec3::Vec3;

pub const COLOR_BACKGROUND: u32 = 0xFF1E1E1E;
pub const COLOR_GRID: u32 = 0xFF333333;
pub const COLOR_MAGENTA: u32 = 0xFFFF00FF;

pub struct Engine {
    color_buffer: Vec<u32>,
    buffer_width: u32,
    buffer_height: u32,
    triangles_to_render: Vec<Triangle>,
    mesh: Mesh
}

impl Engine {
    pub fn new(buffer_width: u32, buffer_height: u32) -> Self {
        let size = (buffer_width * buffer_height) as usize;
        Self {
            color_buffer: vec![COLOR_BACKGROUND; size],
            buffer_width,
            buffer_height,
            triangles_to_render: Vec::new(),
            mesh: Mesh::new(vec![], vec![], Vec3::ZERO),
        }
    }

    pub fn load_cube_mesh(&mut self) {
        self.mesh = Mesh::new(CUBE_VERTICES.to_vec(), CUBE_FACES.to_vec(), Vec3::ZERO);
    }

    /// Resize the buffer, maintaining the invariant that buffer_width * buffer_height == color_buffer.len()
    /// This is the only safe way to change buffer dimensions
    pub fn resize(&mut self, buffer_width: u32, buffer_height: u32) {
        let size = (buffer_width * buffer_height) as usize;
        self.color_buffer = vec![COLOR_BACKGROUND; size];
        self.buffer_width = buffer_width;
        self.buffer_height = buffer_height;
    }

    pub fn clear_color_buffer(&mut self, color: u32) {
        self.color_buffer.fill(color);
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.buffer_width as i32 && y >= 0 && y < self.buffer_height as i32 {
            let index = (y as u32 * self.buffer_width + x as u32) as usize;
            self.color_buffer[index] = color;
        }
    }

    pub fn buffer_width(&self) -> u32 {
        self.buffer_width
    }

    pub fn buffer_height(&self) -> u32 {
        self.buffer_height
    }

    pub fn draw_grid(&mut self, spacing: i32, color: u32) {
        for y in 0..self.buffer_height as i32 {
            for x in 0..self.buffer_width as i32 {
                if x % spacing == 0 || y % spacing == 0 {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn set_triangles_to_render(&mut self, triangles: Vec<Triangle>) {
        self.triangles_to_render = triangles;
    }

    pub fn get_triangles_to_render(&self) -> &[Triangle] {
        &self.triangles_to_render
    }

    pub fn get_triangles_to_render_mut(&mut self) -> &mut Vec<Triangle> {
        &mut self.triangles_to_render
    }

    pub fn clear_triangles_to_render(&mut self) {
        self.triangles_to_render.clear();
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn mesh_mut(&mut self) -> &mut Mesh {
        &mut self.mesh
    }

    pub fn draw_triangle(&mut self, triangle: &Triangle) {
        // Draw triangle wireframe by drawing lines between the three points
        if triangle.points.len() != 3 {
            return;
        }

        let p0 = &triangle.points[0];
        let p1 = &triangle.points[1];
        let p2 = &triangle.points[2];

        // Draw lines between points (simple line drawing)
        self.draw_line_bresenham(p0.x as i32, p0.y as i32, p1.x as i32, p1.y as i32, triangle.color);
        self.draw_line_bresenham(p1.x as i32, p1.y as i32, p2.x as i32, p2.y as i32, triangle.color);
        self.draw_line_bresenham(p2.x as i32, p2.y as i32, p0.x as i32, p0.y as i32, triangle.color);
    } 

    #[allow(dead_code)]
    fn draw_line_dda(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        // Digital Differential Analyzer (DDA) line algorithm

        // Calculate the length of each side of the triangle
        let dx = x1 - x0;
        let dy = y1 - y0;

        // Determine the longest side of the triangle
        let mut side_length = dx.abs();
        if dy.abs() > side_length {
            side_length = dy.abs();
        }

        // Calculate the increment for each side of the triangle
        // One of these will be 1, and the other will be less than 1, some fractional amount.
        let x_increment = dx as f32 / side_length as f32;
        let y_increment = dy as f32 / side_length as f32;
        let mut current_x = x0 as f32;
        let mut current_y = y0 as f32;

        // Loop through the longest side of the triangle and set the pixels along the way.
        // The rounding is necessary to ensure the pixels are set at the correct integer coordinates.
        // The increment is added to the current x and y coordinates to move the pixel along the line.
        for _ in 0..side_length {
            self.set_pixel(current_x.round() as i32, current_y.round() as i32, color);
            current_x = current_x + x_increment;
            current_y = current_y + y_increment;
        }
    }

    fn draw_line_bresenham(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        // Bresenham's line algorithm
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            self.set_pixel(x, y, color);

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn get_buffer_as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.color_buffer.as_ptr() as *const u8,
                self.color_buffer.len() * 4,
            )
        }
    }

    #[cfg(test)]
    fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x >= 0 && x < self.buffer_width as i32 && y >= 0 && y < self.buffer_height as i32 {
            let index = (y as u32 * self.buffer_width + x as u32) as usize;
            Some(self.color_buffer[index])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::vec2::Vec2;

    #[test]
    fn test_new() {
        let engine = Engine::new(100, 200);
        assert_eq!(engine.buffer_width(), 100);
        assert_eq!(engine.buffer_height(), 200);
        assert_eq!(engine.triangles_to_render.len(), 0);
        // Check that buffer is initialized with background color
        assert_eq!(engine.get_pixel(0, 0), Some(COLOR_BACKGROUND));
        assert_eq!(engine.get_pixel(99, 199), Some(COLOR_BACKGROUND));
    }

    #[test]
    fn test_set_pixel_valid() {
        let mut engine = Engine::new(10, 10);
        let test_color = 0xFF00FF00;
        
        engine.set_pixel(5, 5, test_color);
        assert_eq!(engine.get_pixel(5, 5), Some(test_color));
    }

    #[test]
    fn test_set_pixel_boundary() {
        let mut engine = Engine::new(10, 10);
        let test_color = 0xFF00FF00;
        
        // Test boundaries
        engine.set_pixel(0, 0, test_color);
        assert_eq!(engine.get_pixel(0, 0), Some(test_color));
        
        engine.set_pixel(9, 9, test_color);
        assert_eq!(engine.get_pixel(9, 9), Some(test_color));
    }

    #[test]
    fn test_set_pixel_out_of_bounds() {
        let mut engine = Engine::new(10, 10);
        let test_color = 0xFF00FF00;
        
        // Test negative coordinates
        engine.set_pixel(-1, 5, test_color);
        assert_eq!(engine.get_pixel(-1, 5), None);
        
        engine.set_pixel(5, -1, test_color);
        assert_eq!(engine.get_pixel(5, -1), None);
        
        // Test coordinates beyond bounds
        engine.set_pixel(10, 5, test_color);
        assert_eq!(engine.get_pixel(10, 5), None);
        
        engine.set_pixel(5, 10, test_color);
        assert_eq!(engine.get_pixel(5, 10), None);
    }

    #[test]
    fn test_clear_color_buffer() {
        let mut engine = Engine::new(10, 10);
        let clear_color = 0xFF123456;
        
        // Set some pixels first
        engine.set_pixel(0, 0, 0xFF000000);
        engine.set_pixel(5, 5, 0xFF000000);
        engine.set_pixel(9, 9, 0xFF000000);
        
        // Clear the buffer
        engine.clear_color_buffer(clear_color);
        
        // Check that all pixels are cleared
        assert_eq!(engine.get_pixel(0, 0), Some(clear_color));
        assert_eq!(engine.get_pixel(5, 5), Some(clear_color));
        assert_eq!(engine.get_pixel(9, 9), Some(clear_color));
        assert_eq!(engine.get_pixel(3, 7), Some(clear_color));
    }

    #[test]
    fn test_draw_rect() {
        let mut engine = Engine::new(10, 10);
        let rect_color = 0xFF00FF00;
        
        engine.draw_rect(2, 2, 3, 3, rect_color);
        
        // Check that the rectangle was drawn
        assert_eq!(engine.get_pixel(2, 2), Some(rect_color));
        assert_eq!(engine.get_pixel(4, 2), Some(rect_color));
        assert_eq!(engine.get_pixel(2, 4), Some(rect_color));
        assert_eq!(engine.get_pixel(4, 4), Some(rect_color));
        
        // Check that pixels outside the rectangle are not drawn
        assert_eq!(engine.get_pixel(1, 2), Some(COLOR_BACKGROUND));
        assert_eq!(engine.get_pixel(5, 2), Some(COLOR_BACKGROUND));
        assert_eq!(engine.get_pixel(2, 1), Some(COLOR_BACKGROUND));
        assert_eq!(engine.get_pixel(2, 5), Some(COLOR_BACKGROUND));
    }

    #[test]
    fn test_draw_rect_partial_out_of_bounds() {
        let mut engine = Engine::new(10, 10);
        let rect_color = 0xFF00FF00;
        
        // Draw a rectangle that extends beyond bounds
        engine.draw_rect(8, 8, 5, 5, rect_color);
        
        // Only the in-bounds pixels should be drawn
        assert_eq!(engine.get_pixel(8, 8), Some(rect_color));
        assert_eq!(engine.get_pixel(9, 9), Some(rect_color));
        // Out of bounds pixels should not be set
        assert_eq!(engine.get_pixel(10, 10), None);
    }

    #[test]
    fn test_draw_grid() {
        let mut engine = Engine::new(10, 10);
        let grid_color = 0xFF00FF00;
        
        engine.clear_color_buffer(0xFF000000);
        engine.draw_grid(3, grid_color);
        
        // Check that grid lines are drawn at spacing intervals
        // x % 3 == 0 or y % 3 == 0 should be grid_color
        assert_eq!(engine.get_pixel(0, 0), Some(grid_color)); // x % 3 == 0
        assert_eq!(engine.get_pixel(3, 0), Some(grid_color)); // x % 3 == 0
        assert_eq!(engine.get_pixel(6, 0), Some(grid_color)); // x % 3 == 0
        assert_eq!(engine.get_pixel(9, 0), Some(grid_color)); // x % 3 == 0
        assert_eq!(engine.get_pixel(0, 3), Some(grid_color)); // y % 3 == 0
        assert_eq!(engine.get_pixel(0, 6), Some(grid_color)); // y % 3 == 0
        assert_eq!(engine.get_pixel(0, 9), Some(grid_color)); // y % 3 == 0
        
        // Check that non-grid pixels are not drawn
        assert_eq!(engine.get_pixel(1, 1), Some(0xFF000000));
        assert_eq!(engine.get_pixel(2, 2), Some(0xFF000000));
    }

    #[test]
    fn test_set_and_get_triangles_to_render() {
        let mut engine = Engine::new(10, 10);
        
        let triangle1 = Triangle {
            points: vec![
                Vec2::new(1.0, 1.0),
                Vec2::new(2.0, 2.0),
                Vec2::new(3.0, 1.0),
            ],
            color: 0xFF00FF00,
        };
        
        let triangle2 = Triangle {
            points: vec![
                Vec2::new(4.0, 4.0),
                Vec2::new(5.0, 5.0),
                Vec2::new(6.0, 4.0),
            ],
            color: 0xFFFF0000,
        };
        
        let triangles = vec![triangle1.clone(), triangle2.clone()];
        engine.set_triangles_to_render(triangles);
        
        let rendered = engine.get_triangles_to_render();
        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0], triangle1);
        assert_eq!(rendered[1], triangle2);
    }

    #[test]
    fn test_draw_triangle_valid() {
        let mut engine = Engine::new(20, 20);
        engine.clear_color_buffer(0xFF000000);
        
        let triangle = Triangle {
            points: vec![
                Vec2::new(5.0, 5.0),
                Vec2::new(15.0, 5.0),
                Vec2::new(10.0, 15.0),
            ],
            color: 0xFF00FF00,
        };
        
        engine.draw_triangle(&triangle);
        
        // Check that at least some pixels along the triangle edges are drawn
        // The exact pixels depend on the DDA algorithm, but we should see
        // pixels near the vertices
        let mut pixels_drawn = 0;
        for y in 0..20 {
            for x in 0..20 {
                if engine.get_pixel(x, y) == Some(0xFF00FF00) {
                    pixels_drawn += 1;
                }
            }
        }
        
        // Should have drawn at least some pixels (the three edges)
        assert!(pixels_drawn > 0);
    }

    #[test]
    fn test_draw_triangle_invalid_points() {
        let mut engine = Engine::new(10, 10);
        engine.clear_color_buffer(0xFF000000);
        
        // Triangle with wrong number of points
        let triangle = Triangle {
            points: vec![
                Vec2::new(1.0, 1.0),
                Vec2::new(2.0, 2.0),
            ],
            color: 0xFF00FF00,
        };
        
        engine.draw_triangle(&triangle);
        
        // Should not draw anything
        assert_eq!(engine.get_pixel(1, 1), Some(0xFF000000));
    }

    #[test]
    fn test_get_buffer_as_bytes() {
        let mut engine = Engine::new(2, 2);
        let test_color = 0xFF123456;
        
        engine.set_pixel(0, 0, test_color);
        
        let bytes = engine.get_buffer_as_bytes();
        // Buffer should be 2 * 2 * 4 = 16 bytes
        assert_eq!(bytes.len(), 16);
        
        // Check that the first pixel (little-endian) matches
        // 0xFF123456 in little-endian bytes: 56 34 12 FF
        assert_eq!(bytes[0], 0x56);
        assert_eq!(bytes[1], 0x34);
        assert_eq!(bytes[2], 0x12);
        assert_eq!(bytes[3], 0xFF);
    }

    #[test]
    fn test_draw_line_horizontal() {
        let mut engine = Engine::new(10, 10);
        engine.clear_color_buffer(0xFF000000);
        
        // Draw a horizontal line using DDA (through draw_triangle)
        // We'll test by creating a triangle with a horizontal edge
        let triangle = Triangle {
            points: vec![
                Vec2::new(1.0, 5.0),
                Vec2::new(8.0, 5.0),
                Vec2::new(4.0, 7.0),
            ],
            color: 0xFF00FF00,
        };
        
        engine.draw_triangle(&triangle);
        
        // Should have drawn pixels along the horizontal line from (1,5) to (8,5)
        let mut found_horizontal = false;
        for x in 1..=8 {
            if engine.get_pixel(x, 5) == Some(0xFF00FF00) {
                found_horizontal = true;
                break;
            }
        }
        assert!(found_horizontal);
    }

    #[test]
    fn test_draw_line_vertical() {
        let mut engine = Engine::new(10, 10);
        engine.clear_color_buffer(0xFF000000);
        
        // Draw a vertical line
        let triangle = Triangle {
            points: vec![
                Vec2::new(5.0, 1.0),
                Vec2::new(5.0, 8.0),
                Vec2::new(7.0, 4.0),
            ],
            color: 0xFF00FF00,
        };
        
        engine.draw_triangle(&triangle);
        
        // Should have drawn pixels along the vertical line from (5,1) to (5,8)
        let mut found_vertical = false;
        for y in 1..=8 {
            if engine.get_pixel(5, y) == Some(0xFF00FF00) {
                found_vertical = true;
                break;
            }
        }
        assert!(found_vertical);
    }

    #[test]
    fn test_draw_line_diagonal() {
        let mut engine = Engine::new(10, 10);
        engine.clear_color_buffer(0xFF000000);
        
        // Draw a diagonal line
        let triangle = Triangle {
            points: vec![
                Vec2::new(1.0, 1.0),
                Vec2::new(8.0, 8.0),
                Vec2::new(1.0, 8.0),
            ],
            color: 0xFF00FF00,
        };
        
        engine.draw_triangle(&triangle);
        
        // Should have drawn pixels along the diagonal
        let mut found_diagonal = false;
        for i in 1..=8 {
            if engine.get_pixel(i, i) == Some(0xFF00FF00) {
                found_diagonal = true;
                break;
            }
        }
        assert!(found_diagonal);
    }
}
