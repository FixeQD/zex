//! Hyprland backend: JSON commands over the hyprctl socket, events over the `.socket2.sock` event socket

use super::traits::{Compositor, CompositorEvent, WindowInfo, WorkspaceInfo};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::{BufRead, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use tracing::warn;

pub const INSTANCE_ENV: &str = "HYPRLAND_INSTANCE_SIGNATURE";

pub struct HyprlandCompositor {
    socket_path: PathBuf,
    events: flume::Receiver<CompositorEvent>,
}

impl HyprlandCompositor {
    pub fn new() -> Option<Self> {
        let signature = std::env::var(INSTANCE_ENV).ok()?;
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let base = PathBuf::from(format!("{runtime_dir}/hypr/{signature}"));
        let socket_path = base.join(".socket.sock");
        let event_socket_path = base.join(".socket2.sock");
        if !socket_path.exists() || !event_socket_path.exists() {
            return None;
        }
        let (sender, events) = flume::unbounded();
        spawn_event_listener(event_socket_path, sender);
        Some(Self {
            socket_path,
            events,
        })
    }

    fn send_command(&self, cmd: &str) -> Result<String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("connect to hyprctl socket {:?}", self.socket_path))?;
        stream
            .write_all(cmd.as_bytes())
            .context("write hyprctl command")?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .context("read hyprctl response")?;
        Ok(response)
    }
}

fn spawn_event_listener(path: PathBuf, sender: flume::Sender<CompositorEvent>) {
    if std::thread::Builder::new()
        .name("zex-hyprland-events".to_string())
        .spawn(move || {
            let stream = match UnixStream::connect(&path) {
                Ok(stream) => stream,
                Err(error) => {
                    warn!("hyprland event socket connect failed: {error}");
                    return;
                }
            };
            let mut reader = std::io::BufReader::new(stream);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap_or_default() == 0 {
                    break;
                }
                if let Some(event) = handle_event(line.trim_end())
                    && sender.send(event).is_err()
                {
                    return;
                }
            }
            warn!("hyprland event stream ended");
        })
        .is_err()
    {
        warn!("failed to spawn hyprland event listener thread");
    }
}

/// Map a raw hyprland event line (`type>>value`) onto a normalized [`CompositorEvent`]
pub fn handle_event(line: &str) -> Option<CompositorEvent> {
    let (kind, value) = line.split_once(">>")?;
    let value = value.trim();
    match kind {
        "createworkspacev2" => Some(CompositorEvent::WorkspacesChanged),
        "destroyworkspacev2" => Some(CompositorEvent::WorkspacesChanged),
        "workspace" => {
            // workspace>><id>,<name>
            let id = value.split(',').next()?.trim().parse().ok()?;
            Some(CompositorEvent::WorkspaceChanged { id })
        }
        "activewindow" => Some(CompositorEvent::ActiveWindowChanged),
        "openwindow" => Some(CompositorEvent::WindowOpened),
        "closewindow" => Some(CompositorEvent::WindowClosed),
        _ => None,
    }
}

/// A window as reported by `j/clients` / `j/activewindow`
#[derive(Debug, Deserialize, Default)]
pub struct HyprlandWindow {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub workspace: HyprlandWorkspace,
    /// 0 means currently focused; higher numbers are less recently focused
    #[serde(rename = "focusHistoryID", default)]
    focus_history_id: i32,
    #[serde(default)]
    pub mapped: bool,
    #[serde(default)]
    pub hidden: bool,
}

impl HyprlandWindow {
    pub fn is_focused(&self) -> bool {
        self.focus_history_id == 0
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct HyprlandWorkspace {
    pub id: i32,
    pub name: String,
}

/// Normalize a hyprland window into [`WindowInfo`]
pub fn window_to_info(window: &HyprlandWindow) -> WindowInfo {
    WindowInfo {
        address: window.address.clone(),
        title: if window.title.is_empty() {
            window.class.clone()
        } else {
            window.title.clone()
        },
        class: window.class.clone(),
        workspace: window.workspace.id,
        focused: window.is_focused(),
    }
}

/// Normalize a hyprland workspace into [`WorkspaceInfo`]
pub fn workspace_to_info(workspace: &HyprlandWorkspace) -> WorkspaceInfo {
    WorkspaceInfo {
        id: workspace.id,
        name: workspace.name.clone(),
        active: false,
        focused: false,
    }
}

impl Compositor for HyprlandCompositor {
    fn name(&self) -> &'static str {
        "Hyprland"
    }

    fn active_window(&self) -> Result<Option<WindowInfo>> {
        let json = self.send_command("j/activewindow")?;
        let value: serde_json::Value = serde_json::from_str(&json).context("parse activewindow")?;
        if value.as_object().is_some_and(|object| object.is_empty()) {
            return Ok(None);
        }
        let window: HyprlandWindow =
            serde_json::from_value(value).context("decode activewindow")?;
        Ok(Some(window_to_info(&window)))
    }

    fn windows(&self) -> Result<Vec<WindowInfo>> {
        let json = self.send_command("j/clients")?;
        let clients: Vec<HyprlandWindow> = serde_json::from_str(&json).context("parse clients")?;
        Ok(clients
            .iter()
            .filter(|window| window.mapped && !window.hidden && !window.class.is_empty())
            .map(window_to_info)
            .collect())
    }

    fn workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        let json = self.send_command("j/workspaces")?;
        let workspaces: Vec<HyprlandWorkspace> =
            serde_json::from_str(&json).context("parse workspaces")?;
        Ok(workspaces.iter().map(workspace_to_info).collect())
    }

    fn switch_to_workspace(&self, id: i32) -> Result<()> {
        self.send_command(&format!("dispatch workspace {id}"))?;
        Ok(())
    }

    fn focus_window(&self, address: &str) -> Result<()> {
        self.send_command(&format!("dispatch focuswindow address:{address}"))?;
        Ok(())
    }

    fn quit(&self) -> Result<()> {
        self.send_command("exit")?;
        Ok(())
    }

    fn events(&self) -> flume::Receiver<CompositorEvent> {
        self.events.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        let window: HyprlandWindow = serde_json::from_str(
            &CLIENT_JSON.replace(r#""title": "Example - Mozilla Firefox","#, ""),
        )
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
        let value = json!({ "address": "0xabc", "class": "kitty", "title": "zsh",
            "workspace": {"id": 1, "name": "1"} });
        let window: HyprlandWindow = serde_json::from_value(value).unwrap();
        assert_eq!(window.address, "0xabc");
        assert_eq!(window.workspace.id, 1);
        assert!(window.is_focused());
    }

    #[test]
    fn workspace_normalization() {
        let workspace: HyprlandWorkspace =
            serde_json::from_str(r#"{"id": 5, "name": "dev"}"#).unwrap();
        let info = workspace_to_info(&workspace);
        assert_eq!(info.id, 5);
        assert_eq!(info.name, "dev");
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
        unsafe {
            std::env::remove_var(INSTANCE_ENV);
        }
        assert!(HyprlandCompositor::new().is_none());
    }

    #[test]
    fn detected_with_runtime_sockets() {
        unsafe {
            std::env::remove_var(INSTANCE_ENV);
        }
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
}
