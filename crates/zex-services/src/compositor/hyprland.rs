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
    /// Whether this workspace is currently focused
    #[serde(default)]
    pub focused: bool,
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
        active: workspace.focused,
        focused: workspace.focused,
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
