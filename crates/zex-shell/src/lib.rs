//! Zex shell library target
//!
//! The shell is a binary crate, but the bar engine and the module widgets are
//! exposed here so integration tests in `tests/` can drive them without a
//! running compositor.

pub mod app;
pub mod services_bridge;
pub mod widgets;
pub mod windows;

#[cfg(test)]
pub mod tests;