//! Compositor abstraction for window and workspace management

pub mod detect;
pub mod hyprland;
pub mod niri;
pub mod traits;

pub use detect::detect_compositor;
pub use traits::{Compositor, CompositorEvent, WindowInfo, WorkspaceInfo};
