//! Stylesheet for preview swatches and theme buttons

use zex_core::Settings as SettingsSnapshot;
use zex_core::theme::matugen::Preview;

/// Build the stylesheet that colors the preview swatches and theme buttons
pub fn preview_css(previews: &[Preview], settings: &SettingsSnapshot) -> String {
    let mut css = String::new();
    for (index, preview) in previews.iter().enumerate() {
        css.push_str(&format!(
            ".pv-{index} {{ background-color: {}; }}\n",
            preview.surface
        ));
        css.push_str(&format!(
            ".pv-{index} .primary {{ background-color: {}; }}\n",
            preview.primary
        ));
        css.push_str(&format!(
            ".pv-{index} .secondary {{ background-color: {}; }}\n",
            preview.secondary
        ));
        css.push_str(&format!(
            ".pv-{index} .tertiary {{ background-color: {}; }}\n",
            preview.tertiary
        ));
    }

    let scheme = settings.appearance.wallcolors.color_scheme.as_str();
    for (preview_class, want_dark) in [("dark-preview", true), ("light-preview", false)] {
        let matched = previews
            .iter()
            .find(|preview| preview.scheme == scheme && preview.dark == want_dark)
            .or_else(|| previews.iter().find(|preview| preview.dark == want_dark));
        if let Some(preview) = matched {
            css.push_str(&format!(
                ".{preview_class} .container {{ background-color: {}; }}\n",
                preview.surface
            ));
            css.push_str(&format!(
                ".{preview_class} .surface {{ background-color: {}; }}\n",
                preview.surface
            ));
            css.push_str(&format!(
                ".{preview_class} .btn-1 {{ background-color: {}; }}\n",
                preview.primary
            ));
            css.push_str(&format!(
                ".{preview_class} .btn-2 {{ background-color: {}; }}\n",
                preview.secondary
            ));
            css.push_str(&format!(
                ".{preview_class} .btn-3 {{ background-color: {}; }}\n",
                preview.tertiary
            ));
        }
    }
    css
}
