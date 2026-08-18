//! Executing launcher items

use crate::apps::session_env;
use crate::apps::{DEFAULT_TERMINAL_TEMPLATE, spawn_entry};
use crate::items::Item;
use crate::search::providers::{build_url, default_url, find};
use std::process::Command;
use tracing::debug;

const THEME_KEYS: [(&str, &str); 3] = [
    ("org.gnome.desktop.interface", "gtk-theme"),
    ("org.gnome.desktop.interface", "icon-theme"),
    ("org.gnome.desktop.interface", "cursor-theme"),
];

/// Run the action described by an item
pub fn dispatch(item: &Item) -> anyhow::Result<()> {
    match item {
        Item::App(app) => spawn_entry(app, DEFAULT_TERMINAL_TEMPLATE),
        Item::Action { command, .. } | Item::Command(command) => run_shell(command),
        Item::File(path) => open_external(&path.to_string_lossy()),
        Item::Web { provider, query } => {
            let url = find(provider)
                .map(|p| build_url(p, query))
                .unwrap_or_else(|| default_url(query));
            open_external(&url)
        }
        Item::Theme(name) => apply_theme(name),
        Item::Ai(prompt) => {
            let encoded = urlencoding::encode(prompt);
            open_external(&format!("https://chatgpt.com/?q={encoded}"))
        }
        Item::Calc { .. } | Item::Menu(_) => Ok(()),
        Item::Window { title, .. } => {
            debug!("window switching arrives with compositor IPC: {title}");
            Ok(())
        }
    }
}

fn run_shell(line: &str) -> anyhow::Result<()> {
    if line.trim().is_empty() {
        anyhow::bail!("empty command");
    }
    Command::new("sh")
        .arg("-c")
        .arg(line)
        .envs(session_env())
        .spawn()
        .map(|_| ())
        .map_err(Into::into)
}

fn open_external(target: &str) -> anyhow::Result<()> {
    Command::new("xdg-open")
        .arg(target)
        .envs(session_env())
        .spawn()
        .map(|_| ())
        .map_err(Into::into)
}

fn apply_theme(name: &str) -> anyhow::Result<()> {
    for (schema, key) in THEME_KEYS {
        let _ = Command::new("gsettings")
            .args(["set", schema, key, name])
            .envs(session_env())
            .spawn();
    }
    Ok(())
}
