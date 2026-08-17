//! Session detection tests: pure env logic, no live compositor needed.

use zex_services::compositor::detect_compositor;
use zex_services::compositor::hyprland::{HyprlandCompositor, INSTANCE_ENV};
use zex_services::compositor::niri::{NiriCompositor, SOCKET_ENV};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn nothing_detected_without_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var(SOCKET_ENV);
        std::env::remove_var(INSTANCE_ENV);
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
    assert!(NiriCompositor::new().is_none());
    assert!(HyprlandCompositor::new().is_none());
    assert!(detect_compositor().is_none());
}
