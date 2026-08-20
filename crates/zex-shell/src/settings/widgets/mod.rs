//! Shared settings-window building blocks.

mod controls;
mod layout;
mod palette;
mod theme;
mod toggles;
mod wallpaper;

pub use controls::{spin_button, spin_row, switch_row};
pub use layout::{category, separator, settings_row, vertical_separator};
pub use palette::PaletteRegistry;
pub use palette::palette_button;
pub use theme::theme_selector;
pub use toggles::{IndependentItem, ToggleItem, independent_toggle_buttons, toggle_buttons};
pub use wallpaper::wallpaper_overlay;
