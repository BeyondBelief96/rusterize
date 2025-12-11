use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

// Colors in ARGB8888 format
const COLOR_BACKGROUND: u32 = 0xFF1E1E1E;
const COLOR_GRID: u32 = 0xFF333333;
const COLOR_MAGENTA: u32 = 0xFFFF00FF;

struct Engine {
    color_buffer: Vec<u32>,
    width: u32,
    height: u32,
}

impl Engine {
    fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            color_buffer: vec![COLOR_BACKGROUND; size],
            width,
            height,
        }
    }

    fn clear_color_buffer(&mut self, color: u32) {
        self.color_buffer.fill(color);
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = (y as u32 * self.width + x as u32) as usize;
            self.color_buffer[index] = color;
        }
    }

    fn draw_grid(&mut self, spacing: i32, color: u32) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                if x % spacing == 0 || y % spacing == 0 {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    fn get_buffer_as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.color_buffer.as_ptr() as *const u8,
                self.color_buffer.len() * 4,
            )
        }
    }
}

fn process_input(event_pump: &mut sdl2::EventPump) -> (bool, Option<(u32, u32)>) {
    let mut new_size = None;
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => return (false, None),
            Event::Window { win_event: sdl2::event::WindowEvent::Resized(w, h), .. } => {
                new_size = Some((w as u32, h as u32));
            }
            _ => {}
        }
    }
    (true, new_size)
}

fn update() {
    // Update game logic here
}

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Russsty", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut window_width = WINDOW_WIDTH;
    let mut window_height = WINDOW_HEIGHT;

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::ARGB8888, window_width, window_height)
        .map_err(|e| e.to_string())?;

    let mut engine = Engine::new(window_width, window_height);
    let mut event_pump = sdl_context.event_pump()?;

    let mut is_running = true;

    while is_running {
        // Process input
        let (running, new_size) = process_input(&mut event_pump);
        is_running = running;

        // Handle resize
        if let Some((w, h)) = new_size {
            window_width = w;
            window_height = h;
            engine = Engine::new(window_width, window_height);
            texture = texture_creator
                .create_texture_streaming(PixelFormatEnum::ARGB8888, window_width, window_height)
                .map_err(|e| e.to_string())?;
        }

        // Update
        update();

        // Render
        engine.clear_color_buffer(COLOR_BACKGROUND);
        engine.draw_grid(50, COLOR_GRID);
        engine.draw_rect(300, 200, 300, 150, COLOR_MAGENTA);

        // Update texture with color buffer
        texture
            .update(
                None,
                engine.get_buffer_as_bytes(),
                (window_width * 4) as usize,
            )
            .map_err(|e| e.to_string())?;

        // Clear and render
        canvas.set_draw_color(sdl2::pixels::Color::RGB(64, 64, 64));
        canvas.clear();
        canvas.copy(&texture, None, Some(Rect::new(0, 0, window_width, window_height)))?;
        canvas.present();
    }

    Ok(())
}
