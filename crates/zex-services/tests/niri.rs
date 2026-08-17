//! Unit tests for the niri backend: JSON normalization and event mapping.

use niri_ipc::{Event, Window, Workspace};
use zex_services::compositor::CompositorEvent;
use zex_services::compositor::niri::{handle_event, window_to_info, workspace_to_info};

const WINDOW_JSON: &str = r#"{
    "id": 42,
    "title": "Some Browser",
    "app_id": "firefox",
    "pid": 1234,
    "workspace_id": 3,
    "is_focused": true,
    "is_floating": false,
    "is_urgent": false,
    "layout": {
        "tile_size": [1280.0, 720.0],
        "window_size": [1278, 718],
        "window_offset_in_tile": [1.0, 1.0]
    }
}"#;

const WORKSPACE_JSON: &str = r#"{
    "id": 7,
    "idx": 2,
    "name": "dev",
    "output": "eDP-1",
    "is_urgent": false,
    "is_active": true,
    "is_focused": true,
    "active_window_id": 42
}"#;

#[test]
fn window_normalization() {
    let window: Window = serde_json::from_str(WINDOW_JSON).unwrap();
    let info = window_to_info(&window);
    assert_eq!(info.address, "42");
    assert_eq!(info.title, "Some Browser");
    assert_eq!(info.class, "firefox");
    assert_eq!(info.workspace, 3);
    assert!(info.focused);
}

#[test]
fn window_title_falls_back_to_class() {
    let window: Window =
        serde_json::from_str(&WINDOW_JSON.replace(r#""title": "Some Browser","#, "")).unwrap();
    let info = window_to_info(&window);
    assert_eq!(info.title, "firefox");
}

#[test]
fn window_without_app_id_or_title() {
    let window: Window = serde_json::from_str(
        &WINDOW_JSON
            .replace(r#""app_id": "firefox","#, "")
            .replace(r#""title": "Some Browser","#, ""),
    )
    .unwrap();
    let info = window_to_info(&window);
    assert_eq!(info.class, "");
    assert_eq!(info.title, "");
}

#[test]
fn workspace_normalization() {
    let workspace: Workspace = serde_json::from_str(WORKSPACE_JSON).unwrap();
    let info = workspace_to_info(&workspace);
    assert_eq!(info.id, 7);
    assert_eq!(info.name, "dev");
    assert!(info.active);
    assert!(info.focused);
}

#[test]
fn workspace_without_name() {
    let workspace: Workspace =
        serde_json::from_str(&WORKSPACE_JSON.replace(r#""name": "dev","#, "")).unwrap();
    let info = workspace_to_info(&workspace);
    assert_eq!(info.name, "");
}

#[test]
fn event_mapping() {
    let event: Event =
        serde_json::from_str(r#"{"WorkspaceActivated": {"id": 5, "focused": true}}"#).unwrap();
    assert_eq!(
        handle_event(event),
        Some(CompositorEvent::WorkspaceChanged { id: 5 })
    );

    let event: Event = serde_json::from_str(r#"{"WindowFocusChanged": {"id": 12}}"#).unwrap();
    assert_eq!(
        handle_event(event),
        Some(CompositorEvent::ActiveWindowChanged)
    );

    let event: Event = serde_json::from_str(r#"{"WindowClosed": {"id": 9}}"#).unwrap();
    assert_eq!(handle_event(event), Some(CompositorEvent::WindowClosed));

    let event: Event =
        serde_json::from_str(r#"{"WorkspacesChanged": {"workspaces": []}}"#).unwrap();
    assert_eq!(
        handle_event(event),
        Some(CompositorEvent::WorkspacesChanged)
    );

    let event: Event =
        serde_json::from_str(r#"{"OverviewOpenedOrClosed": {"is_open": true}}"#).unwrap();
    assert_eq!(handle_event(event), None);
}
