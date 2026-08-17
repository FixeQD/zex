//! Runtime session detection for compositor backends.

use super::hyprland::HyprlandCompositor;
use super::niri::NiriCompositor;
use super::traits::Compositor;
use tracing::{info, warn};

/// Detect the running compositor from the session environment
/// Returns its client or `None` when no supported compositor is present
pub fn detect_compositor() -> Option<Box<dyn Compositor>> {
    if let Some(compositor) = NiriCompositor::new() {
        info!("detected niri session");
        return Some(Box::new(compositor));
    }

    if let Some(compositor) = HyprlandCompositor::new() {
        info!("detected hyprland session");
        return Some(Box::new(compositor));
    }

    warn!("no supported compositor detected");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_detected_without_env() {
        unsafe {
            std::env::remove_var(crate::compositor::niri::SOCKET_ENV);
            std::env::remove_var(crate::compositor::hyprland::INSTANCE_ENV);
        }
        assert!(NiriCompositor::new().is_none());
        assert!(HyprlandCompositor::new().is_none());
        assert!(detect_compositor().is_none());
    }
}
