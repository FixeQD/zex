//! Fuzzy filtering and ranking of launcher items

use crate::items::Item;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct Matcher {
    inner: SkimMatcherV2,
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            inner: SkimMatcherV2::default().smart_case(),
        }
    }

    /// Fuzzy score of `text` against `query`
    pub fn score(&self, text: &str, query: &str) -> Option<i64> {
        self.inner
            .fuzzy_match(text, query)
            .filter(|score| *score > 0)
    }
}

pub struct Scored {
    pub item: Item,
    pub score: i64,
}

pub fn rank<'a>(
    items: impl IntoIterator<Item = &'a Item>,
    query: &str,
    matcher: &Matcher,
) -> Vec<Scored> {
    if query.is_empty() {
        return items
            .into_iter()
            .map(|item| Scored {
                item: item.clone(),
                score: 0,
            })
            .collect();
    }
    let mut scored: Vec<Scored> = items
        .into_iter()
        .filter_map(|item| {
            matcher.score(&item.title(), query).map(|score| Scored {
                item: item.clone(),
                score,
            })
        })
        .collect();
    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored
}

pub fn best_match<'a>(items: &'a [Item], query: &str) -> Option<&'a Item> {
    items
        .iter()
        .find(|item| item.title().eq_ignore_ascii_case(query.trim()))
}
