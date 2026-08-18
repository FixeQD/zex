//! Zex launcher engine: application discovery, indexing, monitoring, item model and search pipeline

pub mod apps;
pub mod calc;
pub mod engine;
pub mod icons;
pub mod items;
pub mod preview;
pub mod search;
pub mod testkit;

pub use apps::{Change, Watchdog, load_apps};
pub use items::{Item, Menu, dispatch};
