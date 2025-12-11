mod display;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use display::Engine;

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
        .window("Russsty", display::WINDOW_WIDTH, display::WINDOW_HEIGHT)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut window_width = display::WINDOW_WIDTH;
    let mut window_height = display::WINDOW_HEIGHT;

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
        engine.clear_color_buffer(display::COLOR_BACKGROUND);
        engine.draw_grid(50, display::COLOR_GRID);
        engine.draw_rect(300, 200, 300, 150, display::COLOR_MAGENTA);

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
