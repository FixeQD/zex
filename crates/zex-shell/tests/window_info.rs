//! Window info widget helpers: title truncation

use zex_shell::bar::widgets::window_info::truncate;

#[test]
fn truncate_keeps_short_titles() {
    assert_eq!(truncate("hello"), "hello");
    assert_eq!(truncate(&"x".repeat(52)), "x".repeat(52));
}

#[test]
fn truncate_marks_long_titles() {
    let long = "a".repeat(53);
    let cut = truncate(&long);
    assert_eq!(cut.chars().count(), 53);
    assert!(cut.ends_with('…'));
    assert!(!cut.ends_with('a'));
}

#[test]
fn truncate_counts_characters_not_bytes() {
    // Multi-byte accents must not split inside a code point
    let long = "ż".repeat(52);
    assert_eq!(truncate(&long).chars().count(), 52);

    let cut = truncate(&"ł".repeat(53));
    assert_eq!(cut.chars().count(), 53);
    assert!(cut.ends_with('…'));
    // Even though the ellipsis adds an extra char, the visible title body stays uncut
    assert!(cut.starts_with(&"ł".repeat(52)));
}

#[test]
fn truncate_marks_only_strictly_long_titles() {
    // Boundary: exactly at the limit stays as-is, one over gets the ellipsis
    assert_eq!(truncate(&"z".repeat(52)), "z".repeat(52));
    let cut = truncate(&"z".repeat(53));
    assert_eq!(cut.chars().count(), 53);
    assert!(cut.ends_with('…'));
}

#[test]
fn truncate_handles_empty_input() {
    assert_eq!(truncate(""), "");
}
