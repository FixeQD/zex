//! Workspace switcher helpers: pagination window and style parsing

use zex_shell::bar::widgets::workspaces::{Style, page_window};

#[test]
fn page_windows_are_size_amount() {
    assert_eq!(page_window(1, 3), (1, 3));
    assert_eq!(page_window(3, 3), (1, 3));
    assert_eq!(page_window(4, 3), (4, 6));
    assert_eq!(page_window(7, 5), (6, 10));
    // A zero amount collapses to a one-item page
    assert_eq!(page_window(5, 0), (5, 5));
}

#[test]
fn page_windows_cover_every_active_workspace() {
    // Every workspace belongs to exactly one page window
    for active in 1..=25 {
        let (start, end) = page_window(active, 8);
        assert!(
            active >= start && active <= end,
            "{active} outside {start}..={end}"
        );
        assert_eq!(end - start + 1, 8);
    }
}

#[test]
fn style_parses() {
    assert_eq!(Style::from_settings("dots"), Style::Dots);
    assert_eq!(Style::from_settings("windows"), Style::Windows);
    assert_eq!(Style::from_settings("fancy"), Style::Numbers);
    assert_eq!(Style::from_settings(""), Style::Numbers);
    assert_eq!(Style::from_settings("DOTS"), Style::Numbers);
}

#[test]
fn style_values_map_to_css_classes() {
    assert_eq!(Style::Numbers.css_class(), "numbers");
    assert_eq!(Style::Dots.css_class(), "dots");
    assert_eq!(Style::Windows.css_class(), "windows");
    assert_eq!(Style::from_settings("windows").css_class(), "windows");
}
