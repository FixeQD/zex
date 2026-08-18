use tempfile::TempDir;
use zex_launcher::icons::{current_theme, find_icon_file, read_theme_from};

#[test]
fn current_theme_is_never_empty() {
    assert!(!current_theme().is_empty());
}

#[test]
fn parses_kde_theme_line() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("kdeglobals");
    std::fs::write(&path, "[Icons]\nTheme=Adwaita\n").unwrap();
    assert_eq!(read_theme_from(&path), Some("Adwaita".to_string()));

    std::fs::write(&path, "[General]\nTheme=wrong\n").unwrap();
    assert_eq!(read_theme_from(&path), None);

    std::fs::write(&path, "[Icons]\nWidgetStyle=foo\n").unwrap();
    assert_eq!(read_theme_from(&path), None);
}

#[test]
fn lookup_never_panics() {
    let _ = find_icon_file("definitely-not-a-real-icon-name-xyz");
}