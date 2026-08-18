//! Plain-text rendering for item previews

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

pub fn plain_text(markdown: &str) -> String {
    let mut out = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Text(text) | Event::Code(text) => out.push_str(&text),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            Event::Start(Tag::Paragraph)
            | Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Item) => out.push('\n'),
            _ => {}
        }
    }
    out.trim().to_string()
}
