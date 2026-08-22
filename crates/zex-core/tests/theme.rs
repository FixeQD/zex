//! Theming pipeline tests: colour parsing, token overlays, template rendering, SCSS compilation

use std::collections::HashMap;

use serde_json::json;
use zex_core::theme::{Palette, Rgba, COLORS_SCSS, LIGHT_THEME_OVERRIDES_SCSS, PREVIEW_COLORS_SCSS};
use zex_core::theme::{compile, ensure_generator_config, palette_env, preview_scss, render, theme_scss};
use zex_core::theme::matugen;

#[test]
fn hex_parsing() {
    assert_eq!(Rgba::from_hex("#ff0000"), Some(Rgba(0xff0000ff)));
    assert_eq!(Rgba::from_hex("ff0000"), Some(Rgba(0xff0000ff)));
    assert_eq!(Rgba::from_hex("#f00"), Some(Rgba(0xff0000ff)));
    assert_eq!(Rgba::from_hex("#ff000080"), Some(Rgba(0xff000080)));
    assert_eq!(Rgba::from_hex("  #00ff00  "), Some(Rgba(0x00ff00ff)));
    assert_eq!(Rgba::from_hex("#12"), None);
    assert_eq!(Rgba::from_hex("zzzzzz"), None);
}

#[test]
fn css_output_drops_opaque_alpha() {
    assert_eq!(Rgba(0x123456ff).to_css(), "#123456");
    assert_eq!(Rgba(0x12345680).to_css(), "#12345680");
}

#[test]
fn serde_round_trip() {
    let colour = Rgba(0xa1b2c3ff);
    let raw = serde_json::to_string(&colour).unwrap();
    assert_eq!(raw, "\"#a1b2c3\"");
    assert_eq!(serde_json::from_str::<Rgba>(&raw).unwrap(), colour);
    assert!(serde_json::from_str::<Rgba>("\"#xyz\"").is_err());
}

#[test]
fn full_token_map_overlays_every_field() {
    let mut palette = Palette::default();
    let mut map = HashMap::new();
    for (name, value) in palette.tokens() {
        map.insert(
            name,
            if value == "#000000" {
                "#010203".to_string()
            } else {
                value
            },
        );
    }
    let before = palette.clone();
    palette.from_token_map(&map);
    assert_ne!(palette, before);
    assert_eq!(palette.shadow, Rgba(0x010203ff));
}

#[test]
fn token_matching_ignores_case_and_separators() {
    let mut palette = Palette::default();
    let mut map = HashMap::new();
    // keys arrive normalized from the JSON pipeline
    map.insert(
        zex_core::theme::palette::normalize("PrimaryContainer"),
        "#112233".to_string(),
    );
    map.insert(
        zex_core::theme::palette::normalize("on-surface-variant"),
        "#445566".to_string(),
    );
    palette.from_token_map(&map);
    assert_eq!(palette.primary_container, Rgba(0x112233ff));
    assert_eq!(palette.on_surface_variant, Rgba(0x445566ff));
}

#[test]
fn core_only_dump_keeps_defaults_for_missing_tokens() {
    let mut palette = Palette::default();
    let mut map = HashMap::new();
    map.insert("primary".to_string(), "#ff0000".to_string());
    map.insert("surface".to_string(), "#ffffff".to_string());
    palette.from_token_map(&map);
    assert_eq!(palette.primary, Rgba(0xff0000ff));
    assert_eq!(palette.surface, Rgba(0xffffffff));
    assert_eq!(palette.on_primary, Rgba(0xffffffff)); // untouched fallback
}

#[test]
fn json_dump_parsing() {
    let dump = json!({
        "primary": "#111111",
        "on_primary": "#eeeeee",
        "Primary-Container": "#222222",
        "unknown_extra_key": "#ffffff"
    });
    let palette = matugen::palette_from_json(&dump, true).unwrap();
    assert_eq!(palette.primary, Rgba(0x111111ff));
    assert_eq!(palette.on_primary, Rgba(0xeeeeeeff));
    assert_eq!(palette.primary_container, Rgba(0x222222ff));
    assert_eq!(palette.tertiary, Palette::default().tertiary);
}

#[test]
fn json_dump_parsing_v4_layout() {
    let dump = json!({
        "colors": {
            "primary": {
                "dark": { "color": "#101010" },
                "light": { "color": "#fff000" },
                "default": { "color": "#abcdef" }
            },
            "surface-container": {
                "dark": { "color": "#202020" },
                "default": { "color": "#21abcd" }
            },
            "tertiary": {
                "dark": null,
                "light": { "color": "#999999" },
                "default": { "color": "#123123" }
            }
        }
    });
    let palette = matugen::palette_from_json(&dump, true).unwrap();
    assert_eq!(palette.primary, Rgba(0x101010ff)); // dark mode selected
    assert_eq!(palette.surface_container, Rgba(0x202020ff));
    assert_eq!(palette.tertiary, Rgba(0x123123ff)); // fallback to default
    let palette = matugen::palette_from_json(&dump, false).unwrap();
    assert_eq!(palette.primary, Rgba(0xfff000ff)); // light mode selected
    assert_eq!(palette.surface_container, Rgba(0x21abcdff)); // no light entry
}

#[test]
fn fallback_palette_is_light_readable() {
    let palette = Palette::default();
    assert!(!palette.is_dark());
    assert_ne!(palette.surface, palette.on_surface);
    for (_, value) in palette.entries() {
        assert!(value.starts_with('#'));
        assert!(value.len() == 7 || value.len() == 9);
    }
}

#[test]
fn token_count_matches_template() {
    let palette = Palette::default();
    let vars = palette_env(&palette);
    // every {{token}} placeholder is known
    render(COLORS_SCSS, &vars).unwrap();
    // one placeholder per token for the SCSS variables, a second set for the @define-color entries, plus $is-dark
    let placeholders = COLORS_SCSS.matches("{{").count();
    assert_eq!(placeholders, palette.tokens().len() * 2 + 1);
    assert_eq!(vars.len(), palette.tokens().len() + 1);
}

#[test]
fn unknown_placeholder_is_an_error() {
    let mut env = HashMap::new();
    env.insert("primary".to_string(), "#ffffff".to_string());
    assert!(render("$x: {{primary}}; $y: {{nope}};", &env).is_err());
    assert!(render("$x: {{primary", &env).is_err());
}

#[test]
fn render_substitutes_all_placeholders() {
    let palette = Palette::default();
    let rendered = render(COLORS_SCSS, &palette_env(&palette)).unwrap();
    assert!(!rendered.contains("{{"));
    assert!(rendered.contains("#5b5cf0"));
    assert!(rendered.contains("$is-dark: false;"));
}

#[test]
fn theme_scss_light_appends_overrides() {
    let palette = Palette::default();
    let dark = theme_scss(&palette, true).unwrap();
    let light = theme_scss(&palette, false).unwrap();
    assert!(!dark.contains("Light-mode adjustments"));
    assert!(light.contains("Light-mode adjustments"));
    assert_ne!(dark, light);
    // the light overrides carry the dimmed accent
    let primary_dark = palette.primary.dim(0.86).to_css();
    let overrides_section = &light[light.find("Light-mode adjustments").unwrap()..];
    let override_line = overrides_section
        .lines()
        .find(|line| line.contains("@define-color zex-primary"))
        .unwrap();
    assert!(override_line.contains(&primary_dark), "{override_line}");
}

#[test]
fn theme_scss_light_keeps_text_tones() {
    let palette = Palette::default();
    let light = theme_scss(&palette, false).unwrap();
    // text colours are untouched by the light overrides
    let overrides_section = &light[light.find("Light-mode adjustments").unwrap()..];
    assert!(overrides_section.contains("@define-color zex-on-surface #1a1b21;"));
    assert!(overrides_section.contains("@define-color zex-on-background #1a1b21;"));
}

#[test]
fn scss_compiles_to_css() {
    let palette = Palette::default();
    let scss = theme_scss(&palette, true).unwrap();
    let css = compile(&scss).unwrap();
    assert!(css.contains("#5b5cf0"));
    assert!(css.contains("@define-color zex-surface-container-lowest #ffffff;"));
}

#[test]
fn preview_template_renders_lines() {
    let lines = matugen::SCHEMES
        .iter()
        .map(|scheme| format!("  \"{scheme}\": ({scheme}, true),"))
        .collect::<Vec<_>>()
        .join("\n");
    let rendered = preview_scss(&lines).unwrap();
    assert!(!rendered.contains("{{"));
    assert_eq!(rendered.matches("\"").count(), matugen::SCHEMES.len() * 2);
}

#[test]
fn preview_count_is_stable() {
    // 7 dark schemes + first 3 in light
    assert_eq!(matugen::PREVIEW_COUNT, 10);
    assert_eq!(matugen::SCHEMES.len(), 7);
}

#[test]
fn scheme_names_are_stable() {
    assert_eq!(
        matugen::SCHEMES,
        [
            "tonal_spot",
            "content",
            "expressive",
            "neutral",
            "monochrome",
            "rainbow",
            "fidelity",
        ]
    );
}

#[test]
fn ensure_generator_config_writes_once() {
    let dir = std::env::temp_dir().join(format!("zex-theme-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = ensure_generator_config(&dir).unwrap();
    assert!(path.exists());
    let first = std::fs::read_to_string(&path).unwrap();
    let path2 = ensure_generator_config(&dir).unwrap();
    let second = std::fs::read_to_string(&path2).unwrap();
    assert_eq!(first, second);
    let _ = std::fs::remove_dir_all(&dir);
}

// --- E2E tests against the matugen binary ---

/// Minimal lossless PNG writer
fn sample_wallpaper() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("zex-theme-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wallpaper.png");
    if path.exists() {
        return path;
    }

    let width = 64u32;
    let height = 64u32;

    // filtered scanlines: filter byte 0 followed by RGBA pixels
    let mut raw = Vec::with_capacity((width * height * 4 + height) as usize);
    for y in 0..height {
        raw.push(0);
        for x in 0..width {
            raw.extend([(x * 255 / width) as u8, (y * 255 / height) as u8, 128, 255]);
        }
    }

    // zlib stream: header, stored deflate blocks, adler32
    let mut zlib = vec![0x78, 0x01];
    for (index, chunk) in raw.chunks(65535).enumerate() {
        let last = index + 1 == raw.len().div_ceil(65535);
        zlib.push(if last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        zlib.extend(len.to_le_bytes());
        zlib.extend((!len).to_le_bytes());
        zlib.extend(chunk);
    }
    zlib.extend(adler32(&raw).to_be_bytes());

    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::new();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]); // 8-bit, RGBA, no interlace
    push_chunk(&mut png, b"IHDR", &ihdr);
    push_chunk(&mut png, b"IDAT", &zlib);
    push_chunk(&mut png, b"IEND", &[]);
    std::fs::write(&path, png).unwrap();
    path
}

fn push_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend((data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend(kind);
    png.extend(data);
    png.extend(crc32(&png[start..]).to_be_bytes());
}

/// IEEE CRC-32, as used by PNG chunk checksums
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffff_ffff
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

#[test]
#[ignore = "needs matugen binary on PATH"]
fn e2e_generate_palette_from_wallpaper() {
    let path = sample_wallpaper();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let palette = runtime
        .block_on(matugen::generate(&path, "tonal_spot", true))
        .expect("matugen run");
    assert!(palette.is_dark());
    assert_ne!(palette.primary, Palette::default().primary);
    assert!(palette.primary.to_css().starts_with('#'));
}

#[test]
#[ignore = "needs matugen binary on PATH"]
fn e2e_previews_match_scheme_count() {
    let path = sample_wallpaper();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let previews = runtime
        .block_on(matugen::previews(&path))
        .expect("preview generation");
    assert_eq!(previews.len(), matugen::PREVIEW_COUNT);
    for p in &previews {
        assert!(p.primary.starts_with('#'));
        assert!(p.surface.starts_with('#'));
    }
}