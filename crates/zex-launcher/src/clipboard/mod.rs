//! Clipboard memory: captured entries, bounded history, persistence and restore
mod content;
mod history;
mod restore;
mod watch;

pub use content::{Content, Entry};
pub use history::{History, Settings};
pub use restore::{place_image, place_text, restore};
pub use watch::Watcher;
