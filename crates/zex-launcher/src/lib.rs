//! Zex launcher engine: application discovery, indexing, monitoring, item model and search pipeline

pub mod apps;
pub mod calc;
pub mod chat;
pub mod clipboard;
pub mod emoji;
pub mod engine;
pub mod icons;
pub mod ipc;
pub mod items;
pub mod preview;
pub mod process;
pub mod search;
pub mod testkit;

pub use apps::{Change, Watchdog, load_apps};
pub use chat::{Profile, Role, Turn, answer, stream};
pub use clipboard::{Content, Entry, History, Settings as ClipboardSettings, Watcher};
pub use emoji::Glyph;
pub use items::{Item, Menu, dispatch};
