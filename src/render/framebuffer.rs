//! Frame buffer abstraction for 2D pixel access.
//!
//! Provides a safe view into color and depth buffers with bounds-checked access.
//! The depth buffer enables proper hidden surface removal via z-buffer algorithm.

/// A view into color and depth buffers.
///
/// Wraps 1D slices with width/height metadata to enable safe 2D pixel access.
/// This is a borrowed view, not an owning type - it's meant to be created
/// temporarily when you need to pass buffers + dimensions together.
///
/// # Depth Buffer
///
/// The depth buffer stores 1/w values (reciprocal of clip-space W) for each pixel.
/// Using 1/w instead of z because it can be linearly interpolated in screen space.
/// Larger values are closer to the camera (since w increases with distance in
/// left-handed coordinates, 1/w decreases).
pub struct FrameBuffer<'a> {
    color_buffer: &'a mut [u32],
    depth_buffer: &'a mut [f32],
    width: u32,
    height: u32,
}

impl<'a> FrameBuffer<'a> {
    /// Create a new FrameBuffer view from buffer slices and dimensions.
    ///
    /// # Panics
    /// Panics if buffer lengths don't match width * height
    pub fn new(
        color_buffer: &'a mut [u32],
        depth_buffer: &'a mut [f32],
        width: u32,
        height: u32,
    ) -> Self {
        debug_assert_eq!(
            color_buffer.len(),
            (width * height) as usize,
            "Color buffer size doesn't match dimensions"
        );
        debug_assert_eq!(
            depth_buffer.len(),
            (width * height) as usize,
            "Depth buffer size doesn't match dimensions"
        );
        Self {
            color_buffer,
            depth_buffer,
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Set a pixel at (x, y) with depth testing.
    ///
    /// The pixel is only written if the depth value is greater than the existing
    /// depth at that location (closer to camera, since we store 1/w).
    /// Silently ignores out-of-bounds coordinates.
    ///
    /// # Arguments
    /// * `x`, `y` - Pixel coordinates
    /// * `depth` - The 1/w value for this pixel (larger = closer)
    /// * `color` - The color to write if depth test passes
    #[inline]
    pub fn set_pixel_with_depth(&mut self, x: i32, y: i32, depth: f32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            // Depth test: larger 1/w means closer to camera
            if depth > self.depth_buffer[idx] {
                self.depth_buffer[idx] = depth;
                self.color_buffer[idx] = color;
            }
        }
    }

    /// Set a pixel without depth testing (for overlays, UI, etc.)
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.color_buffer[(y as u32 * self.width + x as u32) as usize] = color;
        }
    }

    /// Get the color at (x, y), or None if out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            Some(self.color_buffer[(y as u32 * self.width + x as u32) as usize])
        } else {
            None
        }
    }
}
