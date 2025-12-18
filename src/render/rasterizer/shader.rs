//! Pixel shaders for triangle rasterization.
//!
//! This module provides a trait-based abstraction for per-pixel shading computations,
//! similar to how GPUs separate the fixed-function rasterizer from programmable
//! fragment/pixel shaders.
//!
//! # Architecture
//!
//! The rasterizer handles:
//! - Bounding box computation
//! - Edge function evaluation
//! - Inside/outside testing
//! - Barycentric coordinate calculation
//!
//! The shader handles:
//! - Attribute interpolation (colors, UVs, etc.)
//! - Texture sampling
//! - Final color computation

use crate::colors::{modulate, pack_color, unpack_color};
use crate::prelude::Vec2;
use crate::texture::Texture;

/// Trait for per-pixel shading computations.
///
/// The rasterizer calls `shade()` for each pixel inside the triangle,
/// providing the barycentric coordinates for attribute interpolation.
///
/// # Barycentric Coordinates
///
/// The `lambda` parameter contains three weights [λ₀, λ₁, λ₂] that:
/// - Sum to 1.0 for any point inside the triangle
/// - Represent the "influence" of each vertex on the current pixel
/// - Can be used to interpolate any per-vertex attribute:
///   `attr_at_pixel = λ₀*attr₀ + λ₁*attr₁ + λ₂*attr₂`
pub trait PixelShader {
    /// Compute the color for a pixel given its barycentric coordinates.
    ///
    /// # Arguments
    /// * `lambda` - Barycentric coordinates [λ₀, λ₁, λ₂] that sum to 1.0
    fn shade(&self, lambda: [f32; 3]) -> u32;
}

/// Flat shader - returns a constant color for all pixels.
///
/// Used for flat shading where the entire triangle has a single color
/// computed from the face normal.
pub struct FlatShader {
    color: u32,
}

impl FlatShader {
    pub fn new(color: u32) -> Self {
        Self { color }
    }
}

impl PixelShader for FlatShader {
    #[inline]
    fn shade(&self, _lambda: [f32; 3]) -> u32 {
        self.color
    }
}

/// Gouraud shader - interpolates vertex colors using barycentric coordinates.
///
/// Used for smooth shading where colors are computed per-vertex from
/// vertex normals and then interpolated across the triangle.
pub struct GouraudShader {
    /// Unpacked RGB colors for each vertex, in [0.0, 1.0] range
    colors: [(f32, f32, f32); 3],
}

impl GouraudShader {
    pub fn new(vertex_colors: [u32; 3]) -> Self {
        Self {
            colors: [
                unpack_color(vertex_colors[0]),
                unpack_color(vertex_colors[1]),
                unpack_color(vertex_colors[2]),
            ],
        }
    }
}

impl PixelShader for GouraudShader {
    #[inline]
    fn shade(&self, lambda: [f32; 3]) -> u32 {
        let r = lambda[0] * self.colors[0].0
            + lambda[1] * self.colors[1].0
            + lambda[2] * self.colors[2].0;
        let g = lambda[0] * self.colors[0].1
            + lambda[1] * self.colors[1].1
            + lambda[2] * self.colors[2].1;
        let b = lambda[0] * self.colors[0].2
            + lambda[1] * self.colors[1].2
            + lambda[2] * self.colors[2].2;
        pack_color(r, g, b, 1.0)
    }
}

/// Texture shader - samples texture at interpolated UV coordinates.
///
/// Used for texture mapping where the texture color replaces the
/// vertex colors entirely (no lighting modulation).
pub struct TextureShader<'a> {
    texture: &'a Texture,
    uvs: [Vec2; 3],
}

impl<'a> TextureShader<'a> {
    pub fn new(texture: &'a Texture, uvs: [Vec2; 3]) -> Self {
        Self { texture, uvs }
    }

    /// Interpolate UV coordinates using barycentric weights
    #[inline]
    fn interpolate_uv(&self, lambda: [f32; 3]) -> (f32, f32) {
        let u = lambda[0] * self.uvs[0].x + lambda[1] * self.uvs[1].x + lambda[2] * self.uvs[2].x;
        let v = lambda[0] * self.uvs[0].y + lambda[1] * self.uvs[1].y + lambda[2] * self.uvs[2].y;
        (u, v)
    }
}

impl PixelShader for TextureShader<'_> {
    #[inline]
    fn shade(&self, lambda: [f32; 3]) -> u32 {
        let (u, v) = self.interpolate_uv(lambda);
        self.texture.sample(u, v)
    }
}

/// Modulated texture shader - texture color multiplied by lighting intensity.
///
/// Combines texture mapping with vertex lighting. The texture color is
/// modulated (multiplied) by the interpolated lighting intensity from
/// the vertex colors.
///
/// This allows textures to react to lighting while still showing
/// the texture detail.
pub struct TextureModulateShader<'a> {
    texture: &'a Texture,
    uvs: [Vec2; 3],
    /// Unpacked vertex colors representing lighting intensity
    colors: [(f32, f32, f32); 3],
}

impl<'a> TextureModulateShader<'a> {
    pub fn new(texture: &'a Texture, uvs: [Vec2; 3], vertex_colors: [u32; 3]) -> Self {
        Self {
            texture,
            uvs,
            colors: [
                unpack_color(vertex_colors[0]),
                unpack_color(vertex_colors[1]),
                unpack_color(vertex_colors[2]),
            ],
        }
    }

    /// Interpolate UV coordinates using barycentric weights
    #[inline]
    fn interpolate_uv(&self, lambda: [f32; 3]) -> (f32, f32) {
        let u = lambda[0] * self.uvs[0].x + lambda[1] * self.uvs[1].x + lambda[2] * self.uvs[2].x;
        let v = lambda[0] * self.uvs[0].y + lambda[1] * self.uvs[1].y + lambda[2] * self.uvs[2].y;
        (u, v)
    }

    /// Compute interpolated lighting intensity as average of RGB
    #[inline]
    fn interpolate_intensity(&self, lambda: [f32; 3]) -> f32 {
        let r = lambda[0] * self.colors[0].0
            + lambda[1] * self.colors[1].0
            + lambda[2] * self.colors[2].0;
        let g = lambda[0] * self.colors[0].1
            + lambda[1] * self.colors[1].1
            + lambda[2] * self.colors[2].1;
        let b = lambda[0] * self.colors[0].2
            + lambda[1] * self.colors[1].2
            + lambda[2] * self.colors[2].2;
        (r + g + b) / 3.0
    }
}

impl PixelShader for TextureModulateShader<'_> {
    #[inline]
    fn shade(&self, lambda: [f32; 3]) -> u32 {
        let (u, v) = self.interpolate_uv(lambda);
        let tex_color = self.texture.sample(u, v);
        let intensity = self.interpolate_intensity(lambda);
        modulate(tex_color, intensity)
    }
}
