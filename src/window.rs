//! SDL2 window management, event handling, and input state.
//!
//! Provides the [`Window`] struct for creating and managing the display window,
//! handling input events, and presenting rendered frames.
//!
//! # Input System
//!
//! The window tracks both discrete events ([`WindowEvent`]) and continuous input
//! state ([`InputState`]). Use `poll_events()` for one-shot events like quit or
//! resize, and `input_state()` for held keys and mouse movement.
//!
//! # Mouse Capture
//!
//! Call `capture_mouse()` to enable FPS-style mouse look. When captured:
//! - The cursor is hidden
//! - Mouse movement is reported as relative deltas
//! - The mouse is constrained to the window
//!
//! Call `release_mouse()` to restore normal mouse behavior.

use std::time::Instant;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;
pub const FPS: u64 = 60;
pub const FRAME_TARGET_TIME: f64 = 1000.0 / FPS as f64;

// =============================================================================
// Window Events (Discrete)
// =============================================================================

/// Discrete window events returned by `poll_events()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    None,
    Quit,
    Resize(u32, u32),
    KeyPress(Key),
    RightMouseDown,
}

/// Keys that trigger discrete events.
///
/// These are for one-shot actions (toggle modes, etc).
/// For continuous input (movement), use [`InputState`].
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
    T,
    Escape,
}

// =============================================================================
// Input State (Continuous)
// =============================================================================

/// Continuous input state for movement and camera control.
///
/// Updated each frame by `poll_events()`. Use this for:
/// - WASD movement (held keys)
/// - Mouse look (relative movement when captured)
/// - Roll control (Q/E keys)
///
/// # Example
///
/// ```ignore
/// let input = window.input_state();
/// if input.forward {
///     camera.move_forward(speed * delta_time);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct InputState {
    // Movement keys
    /// W key held - move forward
    pub forward: bool,
    /// S key held - move backward
    pub back: bool,
    /// A key held - strafe left
    pub left: bool,
    /// D key held - strafe right
    pub right: bool,
    /// Space key held - move up
    pub up: bool,
    /// Left Shift or Left Ctrl held - move down
    pub down: bool,

    // Roll keys
    /// Q key held - roll left
    pub roll_left: bool,
    /// E key held - roll right
    pub roll_right: bool,

    // Mouse
    /// Relative mouse movement this frame (dx, dy).
    /// Only populated when mouse is captured.
    pub mouse_delta: (i32, i32),
}

impl InputState {
    /// Resets per-frame state (mouse delta).
    ///
    /// Called at the start of each frame before processing events.
    fn reset_per_frame(&mut self) {
        self.mouse_delta = (0, 0);
    }
}

// =============================================================================
// Frame Timing
// =============================================================================

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

// =============================================================================
// Window
// =============================================================================

pub struct Window {
    // SDL2 resources
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: Box<sdl2::render::TextureCreator<sdl2::video::WindowContext>>,
    texture: sdl2::render::Texture<'static>,
    event_pump: sdl2::EventPump,
    timer_subsystem: sdl2::TimerSubsystem,
    sdl_context: sdl2::Sdl,

    // Window state
    width: u32,
    height: u32,

    // Input state
    input_state: InputState,
    mouse_captured: bool,
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
            sdl_context,
            canvas,
            texture_creator,
            texture,
            event_pump,
            timer_subsystem,
            width,
            height,
            input_state: InputState::default(),
            mouse_captured: false,
        })
    }

    // =========================================================================
    // Event Polling
    // =========================================================================

    /// Polls for events and updates input state.
    ///
    /// Returns discrete events (quit, resize, key press).
    /// Continuous input (WASD, mouse) is available via `input_state()`.
    ///
    /// Call this once per frame at the start of your game loop.
    pub fn poll_events(&mut self) -> WindowEvent {
        // Reset per-frame state
        self.input_state.reset_per_frame();

        // Collect events first to avoid borrow issues
        let events: Vec<Event> = self.event_pump.poll_iter().collect();

        let mut result = WindowEvent::None;

        for event in events {
            match event {
                Event::Quit { .. } => {
                    result = WindowEvent::Quit;
                }

                Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => {
                    if result == WindowEvent::None {
                        result = WindowEvent::Resize(w as u32, h as u32);
                    }
                }

                // Key down - update continuous state and check for discrete events
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    self.update_key_state(keycode, true);

                    // Check for discrete key events (only if we haven't already got one)
                    if result == WindowEvent::None {
                        if let Some(key) = self.keycode_to_discrete_key(keycode) {
                            result = WindowEvent::KeyPress(key);
                        }
                    }
                }

                // Key up - update continuous state only
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    self.update_key_state(keycode, false);
                }

                // Mouse motion - only track when captured
                Event::MouseMotion { xrel, yrel, .. } => {
                    if self.mouse_captured {
                        self.input_state.mouse_delta.0 += xrel;
                        self.input_state.mouse_delta.1 += yrel;
                    }
                }

                // Right mouse button - toggle mouse capture
                Event::MouseButtonDown {
                    mouse_btn: sdl2::mouse::MouseButton::Right,
                    ..
                } => {
                    if result == WindowEvent::None {
                        result = WindowEvent::RightMouseDown;
                    }
                }

                _ => {}
            }
        }

        result
    }

    /// Updates continuous key state based on key press/release.
    fn update_key_state(&mut self, keycode: Keycode, pressed: bool) {
        match keycode {
            // Movement
            Keycode::W => self.input_state.forward = pressed,
            Keycode::S => self.input_state.back = pressed,
            Keycode::A => self.input_state.left = pressed,
            Keycode::D => self.input_state.right = pressed,
            Keycode::Space => self.input_state.up = pressed,
            Keycode::LShift | Keycode::LCtrl => self.input_state.down = pressed,

            // Roll
            Keycode::Q => self.input_state.roll_left = pressed,
            Keycode::E => self.input_state.roll_right = pressed,

            _ => {}
        }
    }

    /// Maps SDL keycode to discrete key event (if applicable).
    fn keycode_to_discrete_key(&self, keycode: Keycode) -> Option<Key> {
        match keycode {
            Keycode::Num1 => Some(Key::Num1),
            Keycode::Num2 => Some(Key::Num2),
            Keycode::Num3 => Some(Key::Num3),
            Keycode::Num4 => Some(Key::Num4),
            Keycode::Num5 => Some(Key::Num5),
            Keycode::C => Some(Key::C),
            Keycode::G => Some(Key::G),
            Keycode::R => Some(Key::R),
            Keycode::F => Some(Key::F),
            Keycode::T => Some(Key::T),
            Keycode::Escape => Some(Key::Escape),
            _ => None,
        }
    }

    // =========================================================================
    // Input State Access
    // =========================================================================

    /// Returns the current continuous input state.
    ///
    /// Use this for movement controls (WASD), roll (Q/E), and mouse look.
    pub fn input_state(&self) -> &InputState {
        &self.input_state
    }

    // =========================================================================
    // Mouse Capture
    // =========================================================================

    /// Captures the mouse for FPS-style camera control.
    ///
    /// When captured:
    /// - Cursor is hidden
    /// - Mouse is constrained to window
    /// - `input_state().mouse_delta` reports relative movement
    ///
    /// Call `release_mouse()` to restore normal behavior.
    pub fn capture_mouse(&mut self) {
        if self.mouse_captured {
            return;
        }

        // Enable relative mouse mode (hides cursor, reports relative motion)
        self.sdl_context.mouse().set_relative_mouse_mode(true);
        self.mouse_captured = true;

        // Clear any accumulated delta
        self.input_state.mouse_delta = (0, 0);
    }

    /// Releases the mouse from capture.
    ///
    /// Restores normal cursor behavior.
    pub fn release_mouse(&mut self) {
        if !self.mouse_captured {
            return;
        }

        self.sdl_context.mouse().set_relative_mouse_mode(false);
        self.mouse_captured = false;
    }

    /// Toggles mouse capture state.
    ///
    /// Convenience method for toggling with a single key.
    pub fn toggle_mouse_capture(&mut self) {
        if self.mouse_captured {
            self.release_mouse();
        } else {
            self.capture_mouse();
        }
    }

    /// Returns whether the mouse is currently captured.
    pub fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    // =========================================================================
    // Rendering
    // =========================================================================

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

    // =========================================================================
    // Accessors
    // =========================================================================

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
