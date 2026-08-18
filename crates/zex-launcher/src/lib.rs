//! Zex launcher engine: application discovery, indexing, monitoring and launching

pub mod apps;
pub mod icons;
pub mod testkit;

pub use apps::{Change, Watchdog, load_apps};
