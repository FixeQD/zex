//! Grouping ranked items into named sections

use super::filter::{Matcher, Scored, rank};
use crate::items::Item;
use crate::search::{Intent, detect};

pub struct Section {
    pub title: &'static str,
    pub items: Vec<Item>,
}

fn dynamic_sections(intent: &Intent) -> Vec<Section> {
    match intent {
        Intent::Run(command) => vec![Section {
            title: "Run",
            items: vec![Item::Command(command.clone())],
        }],
        Intent::Web { trigger, query } => vec![Section {
            title: "Web",
            items: vec![Item::Web {
                provider: trigger.clone(),
                query: query.clone(),
            }],
        }],
        Intent::Math { expression, answer } => vec![Section {
            title: "Calculator",
            items: vec![Item::Calc {
                expression: expression.clone(),
                answer: answer.clone().unwrap_or_else(|| "?".to_string()),
            }],
        }],
        Intent::Browse(_) => Vec::new(),
    }
}

pub fn organize(catalog: Vec<Item>, query: &str, matcher: &Matcher) -> Vec<Section> {
    let intent = detect(query);
    let mut sections = dynamic_sections(&intent);

    let static_groups: [(&str, fn(&Item) -> bool); 6] = [
        ("Applications", |item| matches!(item, Item::App(_))),
        ("Actions", |item| matches!(item, Item::Action { .. })),
        ("Files", |item| matches!(item, Item::File(_))),
        ("Windows", |item| matches!(item, Item::Window { .. })),
        ("Themes", |item| matches!(item, Item::Theme(_))),
        ("Menus", |item| matches!(item, Item::Menu(_))),
    ];
    for (title, belongs) in static_groups {
        let scored: Vec<Scored> = rank(catalog.iter().filter(|item| belongs(item)), query, matcher);
        let items: Vec<Item> = scored.into_iter().map(|s| s.item).collect();
        if !items.is_empty() {
            sections.push(Section { title, items });
        }
    }
    sections
}
