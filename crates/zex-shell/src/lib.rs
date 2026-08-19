//! Zex shell library target
//!
//! The shell is a binary crate, but the bar engine and the module widgets are
//! exposed here so integration tests in `tests/` can drive them without a
//! running compositor.

pub mod bar;
pub mod lockscreen;
pub mod shared;
pub mod wallpaper;
pub mod widgets;
