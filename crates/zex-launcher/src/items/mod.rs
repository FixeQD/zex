//! Launcher item model

use crate::apps::AppInfo;
use std::path::PathBuf;

mod dispatch;
mod traits;

pub use dispatch::dispatch;
pub use traits::{Identify, Launchable};

#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub title: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    App(AppInfo),
    Action {
        owner: String,
        label: String,
        command: String,
    },
    Command(String),
    File(PathBuf),
    Window {
        title: String,
        app: String,
    },
    Web {
        provider: String,
        query: String,
    },
    Theme(String),
    Ai(String),
    Calc {
        expression: String,
        answer: String,
    },
    Clipboard(crate::clipboard::Entry),
    Emoji(crate::emoji::Glyph),
    Menu(Menu),
}

impl Item {
    pub fn title(&self) -> String {
        match self {
            Item::App(app) => app.title.clone(),
            Item::Action { label, .. } => label.clone(),
            Item::Command(line) => line.clone(),
            Item::File(path) => path.to_string_lossy().to_string(),
            Item::Window { title, .. } => title.clone(),
            Item::Web { provider, query } => format!("{provider}: {query}"),
            Item::Theme(name) => format!("Theme: {name}"),
            Item::Ai(prompt) => format!("Ask AI: {prompt}"),
            Item::Calc { expression, .. } => expression.clone(),
            Item::Clipboard(entry) => entry.snippet(),
            Item::Emoji(glyph) => format!("{} {}", glyph.mark, glyph.label),
            Item::Menu(menu) => menu.title.clone(),
        }
    }

    pub fn subtitle(&self) -> Option<String> {
        match self {
            Item::App(app) => app.summary.clone(),
            Item::Action { owner, .. } => Some(format!("in {owner}")),
            Item::File(path) => path
                .parent()
                .map(|parent| parent.to_string_lossy().to_string()),
            Item::Calc { answer, .. } => Some(answer.clone()),
            Item::Window { app, .. } => Some(app.clone()),
            Item::Clipboard(entry) => Some(entry.content.kind_label().to_string()),
            Item::Emoji(glyph) => Some(glyph.label.clone()),
            _ => None,
        }
    }

    pub fn icon_path(&self) -> Option<PathBuf> {
        match self {
            Item::App(app) => app.icon_file.clone(),
            _ => None,
        }
    }
}
