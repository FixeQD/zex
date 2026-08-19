//! Material 3 widget components: buttons, sliders and the navigation rail.

pub mod button;
pub mod navigation_rail;
pub mod showcase;
pub mod slider;

pub use button::{ConnectedButtonGroup, M3Button, M3Shape, M3Size, M3Type};
pub use navigation_rail::NavigationRail;
pub use slider::M3Slider;

/// Stylesheet for every component in this module
pub const M3_CSS_SCSS: &str = include_str!("../../assets/css/m3.scss");

/// Compile and install the M3 stylesheet for the default display
pub fn install_css() -> gtk4::CssProvider {
    crate::shared::install_css_provider(M3_CSS_SCSS)
}
