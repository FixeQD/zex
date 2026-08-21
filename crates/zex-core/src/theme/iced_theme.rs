use iced_core::{Color, Theme, theme::Palette as IcedPalette};

use crate::theme::palette::{Palette, Rgba};

/// Theming entry template (`$primary`, `$surface`, ...)
pub const COLORS_SCSS: &str = include_str!("../../assets/matugen/templates/colors.scss");

/// Overrides applied on top of [`COLORS_SCSS`] in light mode
pub const LIGHT_THEME_OVERRIDES_SCSS: &str =
    include_str!("../../assets/matugen/templates/lightthemeoverrides.scss");

/// Template for the settings preview grid
pub const PREVIEW_COLORS_SCSS: &str =
    include_str!("../../assets/matugen/templates/preview-colors.scss");

/// Convert a Material 3 Palette to an iced Theme
pub fn palette_to_iced_theme(palette: &Palette, dark: bool) -> Theme {
    let iced_palette = palette_to_iced_palette(palette, dark);
    Theme::custom(if dark { "ZexDark" } else { "ZexLight" }, iced_palette)
}

fn palette_to_iced_palette(palette: &Palette, dark: bool) -> IcedPalette {
    let base = if dark {
        IcedPalette::DARK
    } else {
        IcedPalette::LIGHT
    };

    IcedPalette {
        background: palette.background.into(),
        text: palette.on_background.into(),
        primary: palette.primary.into(),
        success: palette.tertiary.into(),
        warning: palette.secondary.into(),
        danger: palette.error.into(),
        // Other fields use defaults from base
        ..base
    }
}

/// Fill `{{name}}` placeholders from `env`
pub fn render(
    source: &str,
    env: &std::collections::HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        rest = &rest[open + 2..];
        let Some(close) = rest.find("}}") else {
            anyhow::bail!("unterminated placeholder in template");
        };
        let key = &rest[..close];
        rest = &rest[close + 2..];
        match env.get(key) {
            Some(value) => out.push_str(value),
            None => anyhow::bail!("unknown template placeholder {{{{{key}}}}}"),
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Placeholder environment for a palette: every token as a css colour + `is_dark`
pub fn palette_env(palette: &Palette) -> std::collections::HashMap<String, String> {
    let mut env = palette.tokens();
    env.insert(
        "is_dark".to_string(),
        if palette.is_dark() { "true" } else { "false" }.to_string(),
    );
    env
}

/// Assemble the full theme stylesheet for a palette: base template + light-mode overrides when applicable
pub fn theme_scss(palette: &Palette, dark: bool) -> anyhow::Result<String> {
    let base = render(COLORS_SCSS, &palette_env(palette))?;
    if dark {
        Ok(base)
    } else {
        let overrides = render(LIGHT_THEME_OVERRIDES_SCSS, &light_env(palette))?;
        Ok(format!("{base}\n\n{overrides}"))
    }
}

fn light_env(palette: &Palette) -> std::collections::HashMap<String, String> {
    let mut env = palette.tokens();
    for token in ["primary", "secondary", "tertiary", "error"] {
        if let Some(colour) = env.get(token).and_then(|hex| Rgba::from_hex(hex)) {
            env.insert(token.to_string(), colour.dim(0.86).to_css());
        }
    }
    env
}

/// Assemble the preview-grid stylesheet from a list of swatch lines
pub fn preview_scss(preview_lines: &str) -> anyhow::Result<String> {
    let mut env = std::collections::HashMap::new();
    env.insert("preview_lines".to_string(), preview_lines.to_string());
    render(PREVIEW_COLORS_SCSS, &env)
}

/// Compile SCSS to CSS with grass
pub fn compile(scss: &str) -> anyhow::Result<String> {
    grass::from_string(scss, &grass::Options::default()).context("scss compilation failed")
}

/// Sync the desktop-wide colour scheme so other toolkits follow the shell
pub fn apply_system_color_scheme(dark: bool) {
    let scheme = if dark { "prefer-dark" } else { "prefer-light" };
    let status = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "color-scheme", scheme])
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => tracing::warn!("gsettings rejected color-scheme {scheme} ({status})"),
        Err(err) => tracing::warn!("could not run gsettings: {err}"),
    }
}

/// Compile-time guard: the template must contain every palette token
pub fn validate_colors_template() -> anyhow::Result<()> {
    let palette = Palette::default();
    render(COLORS_SCSS, &palette_env(&palette)).map(|_| ())
}

use anyhow::Context;

pub fn ensure_generator_config(config_dir: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let dir = config_dir.join("matugen");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join("config.toml");
    if !path.exists() {
        std::fs::write(&path, include_str!("../../assets/matugen/config.toml"))
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(path)
}

/// Create iced Theme directly from wallpaper path, scheme, and dark mode
pub fn theme_from_wallpaper(
    wallpaper: Option<&std::path::Path>,
    scheme: &str,
    dark: bool,
) -> Theme {
    use crate::theme::matugen;

    let scheme = matugen::SCHEMES
        .iter()
        .find(|known| **known == scheme)
        .copied()
        .unwrap_or("tonal_spot");

    let palette = match wallpaper {
        Some(path) if path.is_file() => match block_on(matugen::generate(path, scheme, dark)) {
            Ok(Ok(palette)) => palette,
            Ok(Err(err)) | Err(err) => {
                tracing::warn!("palette generation failed, using fallback: {err:#}");
                Palette::default_for(dark)
            }
        },
        _ => {
            tracing::warn!("no wallpaper configured, using fallback palette");
            Palette::default_for(dark)
        }
    };

    palette_to_iced_theme(&palette, dark)
}

/// Create iced Theme from Settings
pub fn theme_from_settings(settings: &crate::settings::Settings) -> Theme {
    let wc = &settings.appearance.wallcolors;
    let wallpaper = (!wc.wallpaper_path.is_empty()).then_some(wc.wallpaper_path.as_str());
    theme_from_wallpaper(
        wallpaper.map(std::path::Path::new),
        &wc.color_scheme,
        wc.dark_mode,
    )
}

pub fn block_on<F: std::future::Future>(future: F) -> anyhow::Result<F::Output> {
    let runtime = tokio::runtime::Runtime::new()?;
    Ok(runtime.block_on(future))
}

impl From<Rgba> for Color {
    fn from(rgba: Rgba) -> Self {
        let r = ((rgba.0 >> 24) & 0xff) as f32 / 255.0;
        let g = ((rgba.0 >> 16) & 0xff) as f32 / 255.0;
        let b = ((rgba.0 >> 8) & 0xff) as f32 / 255.0;
        let a = (rgba.0 & 0xff) as f32 / 255.0;
        Color { r, g, b, a }
    }
}
