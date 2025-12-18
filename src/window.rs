//! SDL2 window management and event handling.
//!
//! Provides the [`Window`] struct for creating and managing the display window,
//! handling input events, and presenting rendered frames.

use std::time::Instant;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;
pub const FPS: u64 = 60;
pub const FRAME_TARGET_TIME: f64 = 1000.0 / FPS as f64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    None,
    Quit,
    Resize(u32, u32),
    KeyPress(Key),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    C,
    G,
    R,
    F,
}

pub struct FrameLimiter {
    previous_frame_time: u64,
}

impl FrameLimiter {
    pub fn new(window: &Window) -> Self {
        Self {
            previous_frame_time: window.timer().ticks64(),
        }
    }

    /// Waits if necessary to maintain frame rate and returns the delta time in milliseconds.
    /// Delta time represents the time elapsed since the last call to this method.
    pub fn wait_and_get_delta(&mut self, window: &Window) -> u64 {
        let mut current_time = window.timer().ticks64();
        let mut delta_time = current_time - self.previous_frame_time;

        if delta_time < FRAME_TARGET_TIME as u64 {
            let time_to_wait = (FRAME_TARGET_TIME as u64) - delta_time;
            std::thread::sleep(std::time::Duration::from_millis(time_to_wait as u64));
            current_time = window.timer().ticks64();
            delta_time = current_time - self.previous_frame_time;
        }

        self.previous_frame_time = current_time;
        delta_time
    }
}

/// Tracks frames per second with once-per-second updates.
pub struct FpsCounter {
    frame_count: u32,
    last_update: Instant,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_update: Instant::now(),
        }
    }

    /// Call each frame. Returns `Some(fps)` once per second, `None` otherwise.
    pub fn tick(&mut self) -> Option<f64> {
        self.frame_count += 1;
        let elapsed = self.last_update.elapsed();
        if elapsed.as_secs() >= 1 {
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.last_update = Instant::now();
            Some(fps)
        } else {
            None
        }
    }
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Window {
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: Box<sdl2::render::TextureCreator<sdl2::video::WindowContext>>,
    texture: sdl2::render::Texture<'static>,
    event_pump: sdl2::EventPump,
    timer_subsystem: sdl2::TimerSubsystem,
    width: u32,
    height: u32,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let timer_subsystem = sdl_context.timer()?;

        let window = video_subsystem
            .window(title, width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let texture_creator = Box::new(canvas.texture_creator());
        let event_pump = sdl_context.event_pump()?;

        // SAFETY: texture_creator is heap-allocated and lives as long as Window.
        // We ensure texture is dropped before texture_creator by struct field order.
        let texture_creator_ref: &'static sdl2::render::TextureCreator<sdl2::video::WindowContext> =
            unsafe { &*(texture_creator.as_ref() as *const _) };
        let texture = texture_creator_ref
            .create_texture_streaming(PixelFormatEnum::ARGB8888, width, height)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            canvas,
            texture_creator,
            texture,
            event_pump,
            timer_subsystem,
            width,
            height,
        })
    }

    pub fn poll_events(&mut self) -> WindowEvent {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return WindowEvent::Quit,
                Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => return WindowEvent::Resize(w as u32, h as u32),
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    let key = match keycode {
                        Keycode::Num1 => Some(Key::Num1),
                        Keycode::Num2 => Some(Key::Num2),
                        Keycode::Num3 => Some(Key::Num3),
                        Keycode::Num4 => Some(Key::Num4),
                        Keycode::Num5 => Some(Key::Num5),
                        Keycode::C => Some(Key::C),
                        Keycode::G => Some(Key::G),
                        Keycode::R => Some(Key::R),
                        _ => None,
                    };
                    if let Some(k) = key {
                        return WindowEvent::KeyPress(k);
                    }
                }
                _ => {}
            }
        }
        WindowEvent::None
    }

    pub fn present(&mut self, buffer: &[u8]) -> Result<(), String> {
        self.texture
            .update(None, buffer, (self.width * 4) as usize)
            .map_err(|e| e.to_string())?;

        self.canvas.clear();
        self.canvas.copy(
            &self.texture,
            None,
            Some(Rect::new(0, 0, self.width, self.height)),
        )?;
        self.canvas.present();
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.width = width;
        self.height = height;
        // SAFETY: Same as in new() - texture_creator outlives texture
        let texture_creator_ref: &'static sdl2::render::TextureCreator<sdl2::video::WindowContext> =
            unsafe { &*(self.texture_creator.as_ref() as *const _) };
        self.texture = texture_creator_ref
            .create_texture_streaming(PixelFormatEnum::ARGB8888, width, height)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn timer(&self) -> &sdl2::TimerSubsystem {
        &self.timer_subsystem
    }

    pub fn set_title(&mut self, title: &str) {
        let _ = self.canvas.window_mut().set_title(title);
    }
}
