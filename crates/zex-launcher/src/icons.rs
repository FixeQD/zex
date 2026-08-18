//! Icon file lookup for application icons

use freedesktop_icons::lookup;
use std::path::PathBuf;

/// Preferred icon pixel size
const ICON_SIZE: u16 = 64;

pub fn find_icon_file(name: &str) -> Option<PathBuf> {
    let theme = current_theme();
    lookup(name)
        .with_size(ICON_SIZE)
        .with_theme(&theme)
        .find()
        .or_else(|| {
            lookup(name)
                .with_size(ICON_SIZE)
                .with_theme("hicolor")
                .find()
        })
        .or_else(|| search_all_themes(name))
}

fn search_all_themes(name: &str) -> Option<PathBuf> {
    for theme in freedesktop_icons::list_themes() {
        if let Some(path) = lookup(name).with_size(ICON_SIZE).with_theme(&theme).find() {
            return Some(path);
        }
    }
    None
}

/// The active icon theme for this session
pub fn current_theme() -> String {
    let config_dir = dirs::config_dir();
    let kde_theme = config_dir.and_then(|dir| read_theme_from(&dir.join("kdeglobals")));
    kde_theme
        .or_else(|| freedesktop_icons::default_theme_gtk())
        .unwrap_or_else(|| "hicolor".to_string())
}

/// Read the `Theme=` key out of the `[Icons]` group of a KDE globals file
pub fn read_theme_from(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_icons = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_icons = line == "[Icons]";
            continue;
        }
        if in_icons && let Some(theme) = line.strip_prefix("Theme=") {
            return Some(theme.to_string());
        }
    }
    None
}
