//! Material 3 theming pipeline

use std::path::Path;

use anyhow::{Context, Result};

use crate::settings::Settings;

pub mod iced_theme;
pub mod matugen;
pub mod palette;

pub use palette::{Palette, Rgba};
pub use iced_theme::{
    block_on, palette_to_iced_theme, preview_scss, theme_from_settings, theme_from_wallpaper,
};

/// Generate preview swatches for the settings panel
pub fn previews(wallpaper: Option<&Path>) -> Vec<matugen::Preview> {
    let Some(path) = wallpaper.filter(|p| p.is_file()) else {
        tracing::warn!("no wallpaper for previews");
        return Vec::new();
    };
    match block_on(matugen::previews(path)) {
        Ok(Ok(previews)) => previews,
        Ok(Err(err)) | Err(err) => {
            tracing::warn!("preview generation failed: {err:#}");
            Vec::new()
        }
    }
}