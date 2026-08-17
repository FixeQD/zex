//! Unit tests for the hyprland backend: JSON normalization, event mapping and env-based detection

use zex_services::compositor::CompositorEvent;
use zex_services::compositor::hyprland::{
    HyprlandCompositor, HyprlandWindow, HyprlandWorkspace, INSTANCE_ENV, handle_event,
    window_to_info, workspace_to_info,
};

// The detection tests mutate process-wide env vars; `cargo test` runs tests
// in parallel within this binary, so they must take turns.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const CLIENT_JSON: &str = r#"{
    "address": "0x55f123456789",
    "class": "firefox",
    "title": "Example - Mozilla Firefox",
    "workspace": {"id": 2, "name": "2"},
    "focusHistoryID": 0,
    "mapped": true,
    "hidden": false
}"#;

#[test]
fn window_normalization() {
    let window: HyprlandWindow = serde_json::from_str(CLIENT_JSON).unwrap();
    let info = window_to_info(&window);
    assert_eq!(info.address, "0x55f123456789");
    assert_eq!(info.title, "Example - Mozilla Firefox");
    assert_eq!(info.class, "firefox");
    assert_eq!(info.workspace, 2);
    assert!(info.focused);
}

#[test]
fn window_title_falls_back_to_class() {
    let window: HyprlandWindow =
        serde_json::from_str(&CLIENT_JSON.replace(r#""title": "Example - Mozilla Firefox","#, ""))
            .unwrap();
    let info = window_to_info(&window);
    assert_eq!(info.title, "firefox");
    assert!(info.focused);
}

#[test]
fn window_missing_optional_flags_defaults() {
    let window: HyprlandWindow =
        serde_json::from_str(&CLIENT_JSON.replace(r#""mapped": true,"#, "")).unwrap();
    assert!(!window.mapped);
    assert!(!window.hidden);
}

#[test]
fn window_deserializes_from_minimal_json() {
    let value = serde_json::json!({ "address": "0xabc", "class": "kitty", "title": "zsh",
        "workspace": {"id": 1, "name": "1"} });
    let window: HyprlandWindow = serde_json::from_value(value).unwrap();
    assert_eq!(window.address, "0xabc");
    assert_eq!(window.workspace.id, 1);
    assert!(window.is_focused());
}

#[test]
fn workspace_normalization() {
    let workspace: HyprlandWorkspace =
        serde_json::from_str(r#"{"id": 5, "name": "dev", "focused": true}"#).unwrap();
    let info = workspace_to_info(&workspace);
    assert_eq!(info.id, 5);
    assert_eq!(info.name, "dev");
    assert!(info.active);
    assert!(info.focused);
}

#[test]
fn workspace_focus_defaults_to_false() {
    let workspace: HyprlandWorkspace = serde_json::from_str(r#"{"id": 5, "name": "dev"}"#).unwrap();
    let info = workspace_to_info(&workspace);
    assert!(!info.active);
    assert!(!info.focused);
}

#[test]
fn event_mapping() {
    assert_eq!(
        handle_event("workspace>>3,3"),
        Some(CompositorEvent::WorkspaceChanged { id: 3 })
    );
    assert_eq!(
        handle_event("activewindow>>0x123,firefox,Example"),
        Some(CompositorEvent::ActiveWindowChanged)
    );
    assert_eq!(
        handle_event("activewindow>>,"),
        Some(CompositorEvent::ActiveWindowChanged)
    );
    assert_eq!(
        handle_event("openwindow>>0xabc,kitty,zsh"),
        Some(CompositorEvent::WindowOpened)
    );
    assert_eq!(
        handle_event("closewindow>>0xabc"),
        Some(CompositorEvent::WindowClosed)
    );
    assert_eq!(
        handle_event("createworkspacev2>>4,4"),
        Some(CompositorEvent::WorkspacesChanged)
    );
    assert_eq!(
        handle_event("destroyworkspacev2>>4"),
        Some(CompositorEvent::WorkspacesChanged)
    );
    assert_eq!(handle_event("focusedmon>>eDP-1,1"), None);
    assert_eq!(handle_event("garbage"), None);
    assert_eq!(handle_event(""), None);
}

#[test]
fn nothing_detected_without_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var(INSTANCE_ENV);
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
    assert!(HyprlandCompositor::new().is_none());
}

#[test]
fn detected_with_runtime_sockets() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join("zex-hyprland-test");
    let instance = dir.join("hypr").join("sig124");
    std::fs::create_dir_all(&instance).unwrap();
    std::fs::write(instance.join(".socket.sock"), b"").unwrap();
    std::fs::write(instance.join(".socket2.sock"), b"").unwrap();
    unsafe {
        std::env::set_var(INSTANCE_ENV, "sig124");
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
    }
    assert!(HyprlandCompositor::new().is_some());
    let _ = std::fs::remove_dir_all(&dir);
    unsafe {
        std::env::remove_var(INSTANCE_ENV);
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}
