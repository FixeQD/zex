//! Material 3 theming pipeline

use std::path::Path;

use anyhow::{Context, Result};

use crate::settings::Settings;

pub mod css;
pub mod matugen;
pub mod palette;

pub use palette::{Palette, Rgba};

pub struct ThemeManager {
    provider: gtk4::CssProvider,
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
        Self {
            provider: gtk4::CssProvider::new(),
            palette: None,
            dark: true,
        }
    }

    pub fn provider(&self) -> &gtk4::CssProvider {
        &self.provider
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
            match css::ensure_generator_config(config_dir) {
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

        css::apply_theme(&self.provider, &palette, dark)?;
        css::apply_system_color_scheme(dark);
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

fn block_on<F: std::future::Future>(future: F) -> Result<F::Output> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start block_on runtime")?;
    Ok(runtime.block_on(future))
}
