//! Emoji catalog: one-time load from the Unicode data and fuzzy browsing

use crate::engine::Matcher;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub mark: String,
    pub label: String,
}

/// Every emoji the Unicode tables know about, loaded once
pub fn catalog() -> &'static [Glyph] {
    static ALL: OnceLock<Vec<Glyph>> = OnceLock::new();
    ALL.get_or_init(|| {
        emojis::iter()
            .map(|emoji| Glyph {
                mark: emoji.as_str().to_string(),
                label: emoji.name().to_string(),
            })
            .collect()
    })
}

/// Entries matching `query` by fuzzy label match, best first
/// An empty query yields everything, truncated to `limit`
pub fn browse(matcher: &Matcher, query: &str, limit: usize) -> Vec<Glyph> {
    let all = catalog();
    let query = query.trim();
    if query.is_empty() {
        return all.iter().take(limit).cloned().collect();
    }
    let mut scored: Vec<(Glyph, i64)> = all
        .iter()
        .filter_map(|glyph| {
            matcher
                .score(&glyph.label, query)
                .map(|score| (glyph.clone(), score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
        .into_iter()
        .take(limit)
        .map(|(glyph, _)| glyph)
        .collect()
}
