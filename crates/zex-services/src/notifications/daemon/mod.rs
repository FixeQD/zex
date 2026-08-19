//! Daemon subsystem, layered:
//! `model` (types) → `store` (state) → `engine` (logic) → `bus` (D-Bus),
//! with `client` (external facade) and `runtime` (lifecycle) on top.

pub mod client;
pub mod engine;
pub mod history;
pub mod runtime;
pub mod server;
pub mod types;

pub const BUS_NAME: &str = "org.freedesktop.Notifications";
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
