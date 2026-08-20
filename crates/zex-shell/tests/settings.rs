//! Settings panel pure logic: preview swatch stylesheet and tab titles

use zex_core::Settings;
use zex_core::theme::matugen::Preview;
use zex_shell::settings::{preview_css, tab_label};

fn preview(scheme: &str, dark: bool) -> Preview {
    Preview {
        scheme: scheme.to_string(),
        dark,
        primary: "#aa0000".to_string(),
        secondary: "#00aa00".to_string(),
        tertiary: "#0000aa".to_string(),
        surface: "#eeeeee".to_string(),
    }
}

fn previews() -> Vec<Preview> {
    vec![
        preview("tonal_spot", true),
        preview("tonal_spot", false),
        preview("expressive", false),
    ]
}

#[test]
fn preview_css_emits_a_rule_per_swatch() {
    let css = preview_css(&previews(), &Settings::default());
    // 3 swatches × 4 color rules each
    assert_eq!(css.matches(".pv-").count(), 12);
    assert!(css.contains(".pv-0 { background-color: #eeeeee; }"));
    assert!(css.contains(".pv-0 .primary { background-color: #aa0000; }"));
    assert!(css.contains(".pv-2 .tertiary { background-color: #0000aa; }"));
}

#[test]
fn preview_css_picks_matching_scheme_then_darkness() {
    let mut settings = Settings::default();
    settings.appearance.wallcolors.color_scheme = "expressive".to_string();

    let css = preview_css(&previews(), &settings);

    // The "expressive" light preview is selected, not the tonal_spot one
    assert!(css.contains(".light-preview .container { background-color: #eeeeee; }"));
    // dark-preview falls back to any dark preview (tonal_spot)
    assert!(css.contains(".dark-preview .container { background-color: #eeeeee; }"));
}

#[test]
fn preview_css_with_unknown_scheme_falls_back_to_any_dark_or_light() {
    let mut settings = Settings::default();
    settings.appearance.wallcolors.color_scheme = "not-a-scheme".to_string();

    let css = preview_css(&previews(), &settings);

    assert!(css.contains(".dark-preview .container"));
    assert!(css.contains(".light-preview .container"));
    // All previews share the same surface color, so both classes end up identical
    assert!(css.contains(".dark-preview .btn-3 { background-color: #0000aa; }"));
}

#[test]
fn preview_css_handles_empty_previews() {
    let css = preview_css(&[], &Settings::default());
    assert!(css.is_empty());
}

#[test]
fn tab_label_returns_display_names_for_known_keys() {
    assert_eq!(tab_label("quick"), "Quick");
    assert_eq!(tab_label("appearance"), "Appearance");
    assert_eq!(tab_label("interface"), "Interface");
    assert_eq!(tab_label("services"), "Services");
    assert_eq!(tab_label("about"), "About");
}

#[test]
fn tab_label_falls_back_to_the_key_itself() {
    assert_eq!(tab_label("unknown"), "unknown");
    assert_eq!(tab_label(""), "");
}
