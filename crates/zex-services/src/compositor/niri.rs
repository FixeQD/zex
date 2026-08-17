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
