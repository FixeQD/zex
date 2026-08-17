//! Template rendering, SCSS compilation and GTK CSS loading

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::Path;

use super::palette::{Palette, Rgba};

/// Theming entry template (`$primary`, `$surface`, ...)
pub const COLORS_SCSS: &str = include_str!("../../assets/matugen/templates/colors.scss");

/// Overrides applied on top of [`COLORS_SCSS`] in light mode
pub const LIGHT_THEME_OVERRIDES_SCSS: &str =
    include_str!("../../assets/matugen/templates/lightthemeoverrides.scss");

/// Template for the settings preview grid
pub const PREVIEW_COLORS_SCSS: &str =
    include_str!("../../assets/matugen/templates/preview-colors.scss");

/// Fill `{{name}}` placeholders from `env`
pub fn render(source: &str, env: &HashMap<String, String>) -> Result<String> {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        rest = &rest[open + 2..];
        let Some(close) = rest.find("}}") else {
            bail!("unterminated placeholder in template");
        };
        let key = &rest[..close];
        rest = &rest[close + 2..];
        match env.get(key) {
            Some(value) => out.push_str(value),
            None => bail!("unknown template placeholder {{{{{key}}}}}"),
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Placeholder environment for a palette: every token as a css colour + `is_dark`
pub fn palette_env(palette: &Palette) -> HashMap<String, String> {
    let mut env = palette.tokens();
    env.insert(
        "is_dark".to_string(),
        if palette.is_dark() { "true" } else { "false" }.to_string(),
    );
    env
}

/// Assemble the full theme stylesheet for a palette: base template + light-mode overrides when applicable
/// In light mode the accent literals are dimmed first so the defines stay legible on bright surfaces
pub fn theme_scss(palette: &Palette, dark: bool) -> Result<String> {
    let base = render(COLORS_SCSS, &palette_env(palette))?;
    if dark {
        Ok(base)
    } else {
        let overrides = render(LIGHT_THEME_OVERRIDES_SCSS, &light_env(palette))?;
        Ok(format!("{base}\n\n{overrides}"))
    }
}

fn light_env(palette: &Palette) -> HashMap<String, String> {
    let mut env = palette.tokens();
    for token in ["primary", "secondary", "tertiary", "error"] {
        if let Some(colour) = env.get(token).and_then(|hex| Rgba::from_hex(hex)) {
            env.insert(token.to_string(), colour.dim(0.86).to_css());
        }
    }
    env
}

/// Assemble the preview-grid stylesheet from a list of swatch lines
pub fn preview_scss(preview_lines: &str) -> Result<String> {
    let mut env = HashMap::new();
    env.insert("preview_lines".to_string(), preview_lines.to_string());
    render(PREVIEW_COLORS_SCSS, &env)
}

/// Compile SCSS to CSS with grass
pub fn compile(scss: &str) -> Result<String> {
    grass::from_string(scss, &grass::Options::default()).context("scss compilation failed")
}

/// Load compiled CSS into a provider replacing any previous theme
pub fn load(provider: &gtk4::CssProvider, css: &str) {
    provider.load_from_string(css);
}

/// Render, compile and load a full theme for `palette`
pub fn apply_theme(provider: &gtk4::CssProvider, palette: &Palette, dark: bool) -> Result<()> {
    let scss = theme_scss(palette, dark)?;
    let css = compile(&scss)?;
    load(provider, &css);
    Ok(())
}

/// Sync the desktop-wide colour scheme so other toolkits follow the shell
/// A session without the settings service is not an error
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
pub fn validate_colors_template() -> Result<()> {
    let palette = Palette::default();
    render(COLORS_SCSS, &palette_env(&palette)).map(|_| ())
}

pub fn ensure_generator_config(config_dir: &Path) -> Result<std::path::PathBuf> {
    let dir = config_dir.join("matugen");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join("config.toml");
    if !path.exists() {
        std::fs::write(&path, include_str!("../../assets/matugen/config.toml"))
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(path)
}
