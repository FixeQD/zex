//! Wallpaper texture pipeline: decode image → optional blur → GdkTexture.

use std::path::Path;

use anyhow::{Context, Result};
use gtk4::gdk;
use gtk4::gdk_pixbuf::{Colorspace, Pixbuf};
use image::{ImageReader, RgbaImage};

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
    texture_from_rgba(&image)
}

/// Solid 1x1 texture, used when no wallpaper is configured or the image fails to decode
/// Stretched by the picture's `content-fit: cover`
pub fn fallback_texture(rgb: (u8, u8, u8)) -> gdk::Texture {
    let (r, g, b) = rgb;
    let image = RgbaImage::from_pixel(1, 1, image::Rgba([r, g, b, 255]));
    texture_from_rgba(&image).expect("1x1 pixbuf conversion cannot fail")
}

fn texture_from_rgba(image: &RgbaImage) -> Result<gdk::Texture> {
    let width = image.width();
    let height = image.height();
    let bytes = gtk4::glib::Bytes::from_owned(image.as_raw().clone());
    let pixbuf = Pixbuf::from_bytes(
        &bytes,
        Colorspace::Rgb,
        true,
        8,
        width as i32,
        height as i32,
        width as i32 * 4,
    );
    Ok(gdk::Texture::for_pixbuf(&pixbuf))
}
