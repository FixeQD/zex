//! Material 3 theming pipeline

use std::path::Path;

use anyhow::{Context, Result};

use crate::settings::Settings;

pub mod iced_theme;
pub mod matugen;
pub mod palette;

pub use iced_theme::{
    COLORS_SCSS, LIGHT_THEME_OVERRIDES_SCSS, PREVIEW_COLORS_SCSS, compile, ensure_generator_config,
    palette_env, palette_to_iced_theme, preview_scss, render, theme_from_settings,
    theme_from_wallpaper, theme_scss,
};
pub use palette::{Palette, Rgba};

/// Theme manager using iced Themes
pub struct ThemeManager {
    theme: iced_core::Theme,
    palette: Option<Palette>,
    dark: bool,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        let dark = true;
        let palette = Palette::default_for(dark);
        let theme = iced_theme::palette_to_iced_theme(&palette, dark);
        Self {
            theme,
            palette: Some(palette),
            dark,
        }
    }

    pub fn theme(&self) -> &iced_core::Theme {
        &self.theme
    }

    pub fn palette(&self) -> Option<&Palette> {
        self.palette.as_ref()
    }

    pub fn dark(&self) -> bool {
        self.dark
    }

    /// Retheme from explicit inputs
    pub fn refresh(
        &mut self,
        wallpaper: Option<&Path>,
        scheme: &str,
        dark: bool,
        config_dir: Option<&Path>,
    ) -> Result<()> {
        let scheme = matugen::SCHEMES
            .iter()
            .find(|known| **known == scheme)
            .copied()
            .unwrap_or("tonal_spot");

        if let Some(config_dir) = config_dir {
            match iced_theme::ensure_generator_config(config_dir) {
                Ok(_) => {}
                Err(err) => tracing::warn!("generator config not written: {err:#}"),
            }
        }

        let palette = match wallpaper {
            Some(path) if path.is_file() => match block_on(matugen::generate(path, scheme, dark)) {
                Ok(Ok(palette)) => palette,
                Ok(Err(err)) | Err(err) => {
                    tracing::warn!("palette generation failed, using fallback: {err:#}");
                    Palette::default_for(dark)
                }
            },
            other => {
                if other.is_none() {
                    tracing::warn!("no wallpaper configured, using fallback palette");
                } else {
                    tracing::warn!("wallpaper does not exist, using fallback palette");
                }
                Palette::default_for(dark)
            }
        };

        self.theme = iced_theme::palette_to_iced_theme(&palette, dark);
        iced_theme::apply_system_color_scheme(dark);
        self.palette = Some(palette);
        self.dark = dark;
        Ok(())
    }

    /// Retheme from the appearance settings group
    pub fn reload(&mut self, settings: &Settings) -> Result<()> {
        let wc = &settings.appearance.wallcolors;
        let wallpaper = (!wc.wallpaper_path.is_empty()).then_some(wc.wallpaper_path.as_str());
        self.refresh(
            wallpaper.map(Path::new),
            &wc.color_scheme,
            wc.dark_mode,
            None,
        )
    }

    /// The preview swatches for the settings panel
    pub fn previews(&self, wallpaper: Option<&Path>) -> Vec<matugen::Preview> {
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
}

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

fn block_on<F: std::future::Future>(future: F) -> Result<F::Output> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start block_on runtime")?;
    Ok(runtime.block_on(future))
}
