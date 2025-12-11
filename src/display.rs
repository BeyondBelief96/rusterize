pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

// Colors in ARGB8888 format
pub const COLOR_BACKGROUND: u32 = 0xFF1E1E1E;
pub const COLOR_GRID: u32 = 0xFF333333;
pub const COLOR_MAGENTA: u32 = 0xFFFF00FF;

pub struct Engine {
    color_buffer: Vec<u32>,
    width: u32,
    height: u32,
}

impl Engine {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            color_buffer: vec![COLOR_BACKGROUND; size],
            width,
            height,
        }
    }

    pub fn clear_color_buffer(&mut self, color: u32) {
        self.color_buffer.fill(color);
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = (y as u32 * self.width + x as u32) as usize;
            self.color_buffer[index] = color;
        }
    }

    pub fn draw_grid(&mut self, spacing: i32, color: u32) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
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

    pub fn get_buffer_as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.color_buffer.as_ptr() as *const u8,
                self.color_buffer.len() * 4,
            )
        }
    }
}
