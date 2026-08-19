//! Zex shell library target
//!
//! The shell is a binary crate, but the bar engine and the module widgets are
//! exposed here so integration tests in `tests/` can drive them without a
//! running compositor.

pub mod bar;
pub mod corners;
pub mod lockscreen;
pub mod m3;
pub mod overlays;
pub mod shared;
pub mod wallpaper;
pub mod widgets;
