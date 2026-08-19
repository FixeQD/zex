//! Wallpaper texture pipeline: decode image → optional blur → GdkTexture.

use std::path::Path;

use anyhow::{Context, Result};
use gtk4::gdk;
use image::{ImageReader, RgbaImage};

use crate::shared;

/// Decode `path` and produce a background texture for the wallpaper layer
pub fn load_texture(path: &Path, blur_sigma: Option<f32>) -> Result<gdk::Texture> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("cannot open wallpaper image {}", path.display()))?;
    let image = reader
        .decode()
        .with_context(|| format!("cannot decode wallpaper image {}", path.display()))?
        .to_rgba8();

    let image = match blur_sigma {
        Some(sigma) if sigma > 0.0 => image::imageops::fast_blur(&image, sigma),
        _ => image,
    };
    shared::texture_from_rgba(image.width(), image.height(), image.as_raw())
}

/// Solid 1x1 texture, used when no wallpaper is configured or the image fails
/// to decode; stretched by the picture's `content-fit: cover`
pub fn fallback_texture(rgb: (u8, u8, u8)) -> gdk::Texture {
    let (r, g, b) = rgb;
    let image = RgbaImage::from_pixel(1, 1, image::Rgba([r, g, b, 255]));
    shared::texture_from_rgba(1, 1, image.as_raw()).expect("1x1 pixbuf conversion cannot fail")
}
