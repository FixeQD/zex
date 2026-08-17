//! Niri backend over the `niri-ipc` protocol crate.

use super::traits::{Compositor, CompositorEvent, WindowInfo, WorkspaceInfo};
use anyhow::{Context, Result, bail};
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Reply, Request, Response};
use std::path::PathBuf;
use tracing::{trace, warn};

pub const SOCKET_ENV: &str = "NIRI_SOCKET";

pub struct NiriCompositor {
    socket_path: PathBuf,
    events: flume::Receiver<CompositorEvent>,
}

impl NiriCompositor {
    pub fn new() -> Option<Self> {
        let path = PathBuf::from(std::env::var(SOCKET_ENV).ok()?);
        if !path.exists() {
            return None;
        }
        let (sender, events) = flume::unbounded();
        spawn_event_listener(path.clone(), sender);
        Some(Self {
            socket_path: path,
            events,
        })
    }

    fn send(&self, request: Request) -> Result<Response> {
        let mut socket = Socket::connect_to(&self.socket_path)
            .with_context(|| format!("connect to niri socket {:?}", self.socket_path))?;
        let reply = socket.send(request).context("niri request failed")?;
        parse_reply(reply)
    }
}

fn parse_reply(reply: Reply) -> Result<Response> {
    match reply {
        Ok(response) => Ok(response),
        Err(message) => bail!("niri replied with an error: {message}"),
    }
}

fn spawn_event_listener(path: PathBuf, sender: flume::Sender<CompositorEvent>) {
    if std::thread::Builder::new()
        .name("zex-niri-events".to_string())
        .spawn(move || {
            let mut socket = match Socket::connect_to(path) {
                Ok(socket) => socket,
                Err(error) => {
                    warn!("niri event stream connect failed: {error}");
                    return;
                }
            };
            let reply = socket.send(Request::EventStream);
            if !matches!(reply, Ok(Ok(Response::Handled))) {
                warn!("niri event stream request failed: {reply:?}");
                return;
            }
            trace!("niri event stream started");
            let mut read_event = socket.read_events();
            while let Ok(event) = read_event() {
                if let Some(event) = handle_event(event)
                    && sender.send(event).is_err()
                {
                    return;
                }
            }
            warn!("niri event stream ended");
        })
        .is_err()
    {
        warn!("failed to spawn niri event listener thread");
    }
}

/// Map a niri protocol [`Event`] onto normalized [`CompositorEvent`]
pub fn handle_event(event: Event) -> Option<CompositorEvent> {
    match event {
        Event::WorkspacesChanged { .. } => Some(CompositorEvent::WorkspacesChanged),
        Event::WorkspaceActivated { id, .. } => {
            Some(CompositorEvent::WorkspaceChanged { id: id as i32 })
        }
        Event::WindowFocusChanged { .. } => Some(CompositorEvent::ActiveWindowChanged),
        Event::WindowOpenedOrChanged { .. } => Some(CompositorEvent::WindowOpened),
        Event::WindowClosed { .. } => Some(CompositorEvent::WindowClosed),
        _ => None,
    }
}

impl Compositor for NiriCompositor {
    fn name(&self) -> &'static str {
        "Niri"
    }

    fn active_window(&self) -> Result<Option<WindowInfo>> {
        let response = self.send(Request::FocusedWindow)?;
        match response {
            Response::FocusedWindow(window) => Ok(window.as_ref().map(window_to_info)),
            other => bail!("unexpected reply to FocusedWindow: {other:?}"),
        }
    }

    fn windows(&self) -> Result<Vec<WindowInfo>> {
        let response = self.send(Request::Windows)?;
        match response {
            Response::Windows(windows) => Ok(windows.iter().map(window_to_info).collect()),
            other => bail!("unexpected reply to Windows: {other:?}"),
        }
    }

    fn workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        let response = self.send(Request::Workspaces)?;
        match response {
            Response::Workspaces(workspaces) => {
                Ok(workspaces.iter().map(workspace_to_info).collect())
            }
            other => bail!("unexpected reply to Workspaces: {other:?}"),
        }
    }

    fn switch_to_workspace(&self, id: i32) -> Result<()> {
        let index = u8::try_from(id)
            .with_context(|| format!("workspace index {id} is out of niri range"))?;
        let response = self.send(Request::Action(Action::FocusWorkspace {
            reference: niri_ipc::WorkspaceReferenceArg::Index(index),
        }))?;
        match response {
            Response::Handled => Ok(()),
            other => bail!("unexpected reply to FocusWorkspace: {other:?}"),
        }
    }

    fn focus_window(&self, address: &str) -> Result<()> {
        let id = address
            .parse::<u64>()
            .with_context(|| format!("invalid niri window id {address:?}"))?;
        let response = self.send(Request::Action(Action::FocusWindow { id }))?;
        match response {
            Response::Handled => Ok(()),
            other => bail!("unexpected reply to FocusWindow: {other:?}"),
        }
    }

    fn quit(&self) -> Result<()> {
        let response = self.send(Request::Action(Action::Quit {
            skip_confirmation: true,
        }))?;
        match response {
            Response::Handled => Ok(()),
            other => bail!("unexpected reply to Quit: {other:?}"),
        }
    }

    fn events(&self) -> flume::Receiver<CompositorEvent> {
        self.events.clone()
    }
}

pub fn window_to_info(window: &niri_ipc::Window) -> WindowInfo {
    let class = window.app_id.clone().unwrap_or_default();
    let title = window.title.clone().unwrap_or_default();
    WindowInfo {
        address: window.id.to_string(),
        title: if title.is_empty() {
            class.clone()
        } else {
            title
        },
        class,
        workspace: window.workspace_id.map_or(-1, |id| id as i32),
        focused: window.is_focused,
    }
}

pub fn workspace_to_info(workspace: &niri_ipc::Workspace) -> WorkspaceInfo {
    WorkspaceInfo {
        id: workspace.id as i32,
        name: workspace.name.clone().unwrap_or_default(),
        active: workspace.is_active,
        focused: workspace.is_focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niri_ipc::{Event, Window, Workspace};

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
}
