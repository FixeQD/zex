use zex_launcher::emoji::{browse, catalog};
use zex_launcher::engine::Matcher;

#[test]
fn catalog_contains_the_whole_unicode_set() {
    let all = catalog();
    assert!(all.len() > 1000);
    assert!(!all[0].mark.is_empty());
    assert!(!all[0].label.is_empty());
}

#[test]
fn empty_query_returns_a_limited_preview() {
    let browsed = browse(&Matcher::new(), "", 10);
    assert_eq!(browsed.len(), 10);
}

#[test]
fn fuzzy_browse_finds_known_emoji() {
    let browsed = browse(&Matcher::new(), "grinning", 5);
    assert!(!browsed.is_empty());
    assert!(browsed.iter().any(|glyph| glyph.label == "grinning face"));
}

#[test]
fn browse_is_ranked_best_first() {
    let browsed = browse(&Matcher::new(), "heart", 10);
    let scores: Vec<i64> = browsed
        .iter()
        .map(|glyph| Matcher::new().score(&glyph.label, "heart").unwrap_or(0))
        .collect();
    assert!(scores.windows(2).all(|pair| pair[0] >= pair[1]));
}