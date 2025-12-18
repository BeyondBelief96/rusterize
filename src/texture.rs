use std::path::Path;

/// Represents a 2D texture for texture mapping.
pub struct Texture {
    data: Vec<u32>, // The pixel data of the texture in ARGB format.
    width: u32,     // The width of the texture in pixels.
    height: u32,    // The height of the texture in pixels.
}

impl Texture {
    // Load a texture from an image file (PNG, JPG, etc.)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, image::ImageError> {
        let img = image::open(path)?.to_rgba8();
        let (width, height) = img.dimensions();

        // Convert RGBA bytes to ARGB u32
        let data: Vec<u32> = img
            .pixels()
            .map(|p| {
                let [r, g, b, a] = p.0;
                ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
            })
            .collect();

        Ok(Self {
            data,
            width,
            height,
        })
    }

    /// Sample the texture at UV coordinates using nearest-neighbor filtering.
    ///
    /// # UV Coordinate Convention
    /// - UV coordinates are in [0,1] range
    /// - (0,0) = bottom-left in OBJ convention, but textures are stored top-left origin
    /// - We flip V to correct for this: v_corrected = 1.0 - v
    ///
    /// # Wrapping
    /// Uses repeat/wrap mode via rem_euclid for UVs outside [0,1]
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> u32 {
        // Wrap UV coordinates to [0, 1) range using rem_euclid
        // (handles negative values correctly, unlike % operator)
        let u = u.rem_euclid(1.0);

        // Flip V: OBJ uses bottom-left origin, textures use top-left
        let v = (1.0 - v).rem_euclid(1.0);

        // Convert normalized [0,1) UV to pixel coordinates [0, width-1]
        let x = ((u * self.width as f32) as u32).min(self.width - 1);
        let y = ((v * self.height as f32) as u32).min(self.height - 1);

        // Sample from flat array: index = y * width + x
        self.data[(y * self.width + x) as usize]
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
}
