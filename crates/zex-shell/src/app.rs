use std::collections::HashMap;

use iced::Theme;
use iced::window::Id as IcedId;
use iced_exwlshell::actions::{
    ExwlShellCustomAction, ExwlShellCustomActionWithId, IcedXdgWindowSettings,
};
use iced_exwlshell::reexport::{Anchor, NewLayerShellSettings};
use zex_core::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WindowKind {
    Bar { monitor: usize, bar_id: u8 },
    Launcher,
    QuickCenter,
    Osd,
    PowerMenu,
    Wallpaper { monitor: usize },
    Corner { name: String },
    Notification { monitor: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseWindowKind {
    Settings,
}

#[derive(Debug, Clone)]
pub struct WindowState {
    pub kind: WindowKind,
}

pub struct ServiceHandles {
    pub recorder_tx: flume::Sender<zex_services::recorder::RecorderCmd>,
}

impl Default for ServiceHandles {
    fn default() -> Self {
        let (recorder_tx, _) = flume::unbounded();
        Self { recorder_tx }
    }
}

#[derive(Debug, Clone)]
pub enum ServiceEvent {
    Recorder(zex_services::recorder::RecorderEvent),
    Compositor(zex_services::compositor::CompositorEvent),
}

#[derive(Debug, Clone)]
pub enum IpcRequest {
    OpenWindow(String),
    ToggleWindow(String),
    CloseWindow(String),
    RunCommand(String),
    Lock,
    Reload,
    Quit,
}

pub struct State {
    pub config: Settings,
    pub theme: Theme,
    pub windows: HashMap<IcedId, WindowState>,
    pub pending_windows: Vec<(WindowKind, IcedId)>,
    pub pending_lock: bool,
    pub pending_base_windows: Vec<(BaseWindowKind, IcedId)>,
    pub services: ServiceHandles,
    pub ipc_rx: Option<flume::Receiver<IpcRequest>>,
}

impl State {
    pub fn new(
        config: Settings,
        theme: Theme,
        services: ServiceHandles,
        ipc_rx: flume::Receiver<IpcRequest>,
    ) -> Self {
        Self {
            config,
            theme,
            windows: HashMap::new(),
            pending_windows: Vec::new(),
            pending_lock: false,
            pending_base_windows: Vec::new(),
            services,
            ipc_rx: Some(ipc_rx),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ThemeChanged(Theme),
    SettingsChanged(Settings),
    WindowCreated(WindowKind, IcedId),
    BaseWindowCreated(BaseWindowKind, IcedId),
    LockCreated(IcedId),
    CloseWindow(IcedId),
    OpenLayerShell(WindowKind),
    OpenBaseWindow(BaseWindowKind),
    LockRequested,
    AuthResult(bool),
    ServiceEvent(ServiceEvent),
    IpcRequest(IpcRequest),
    ShellEvent(()),
    SwitchWorkspace(i32),
    MediaPlayPause(String),
    NewLayerShell(NewLayerShellSettings, IcedId),
    NewBaseWindow(IcedXdgWindowSettings, IcedId),
    DoLock,
    DoUnlock,
    RemoveWindow(IcedId),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        match self {
            Message::NewLayerShell(settings, id) => Ok(ExwlShellCustomActionWithId(
                Some(id),
                ExwlShellCustomAction::NewLayerShell { settings, id },
            )),
            Message::NewBaseWindow(settings, id) => Ok(ExwlShellCustomActionWithId(
                Some(id),
                ExwlShellCustomAction::NewBaseWindow { settings, id },
            )),
            Message::DoLock => Ok(ExwlShellCustomActionWithId(
                None,
                ExwlShellCustomAction::Lock,
            )),
            Message::DoUnlock => Ok(ExwlShellCustomActionWithId(
                None,
                ExwlShellCustomAction::UnLock,
            )),
            Message::RemoveWindow(id) => Ok(ExwlShellCustomActionWithId(
                Some(id),
                ExwlShellCustomAction::RemoveWindow,
            )),
            other => Err(other),
        }
    }
}

// Helper to map a newly materialized shell to a WindowKind
pub fn shell_info_to_message(info: iced_exwlshell::NewShellInfo) -> Option<Message> {
    use iced_exwlshell::reexport::WlShellType;
    match info.shell {
        WlShellType::LayerShell => Some(Message::WindowCreated(
            WindowKind::Corner {
                name: format!("layer-{:?}", info.id),
            },
            info.id,
        )),
        WlShellType::SessionLock => Some(Message::LockCreated(info.id)),
        WlShellType::XdgTopLevel => Some(Message::BaseWindowCreated(
            BaseWindowKind::Settings,
            info.id,
        )),
        WlShellType::PopUp => Some(Message::WindowCreated(
            WindowKind::Notification { monitor: 0 },
            info.id,
        )),
        WlShellType::InputPanel => None,
    }
}

pub struct ZexProgram;

impl iced::Program for ZexProgram {
    type State = State;
    type Message = Message;
    type Theme = Theme;
    type Renderer = iced_wgpu::Renderer;
    type Executor = iced::executor::Default;

    fn name() -> &'static str {
        "zex-shell"
    }

    fn boot(&self) -> (State, iced::Task<Message>) {
        let config = Settings::default();
        let theme = Theme::Dark;
        let services = ServiceHandles::default();
        let (_tx, rx) = flume::unbounded();
        (State::new(config, theme, services, rx), iced::Task::none())
    }

    fn update(&self, state: &mut State, message: Message) -> iced::Task<Message> {
        match message {
            Message::ThemeChanged(theme) => {
                state.theme = theme;
                iced::Task::none()
            }
            Message::SettingsChanged(config) => {
                state.config = config;
                iced::Task::none()
            }
            Message::WindowCreated(kind, id) => {
                state.windows.insert(id, WindowState { kind });
                iced::Task::none()
            }
            Message::BaseWindowCreated(_, id) => {
                state.windows.insert(
                    id,
                    WindowState {
                        kind: WindowKind::Corner {
                            name: "settings".into(),
                        },
                    },
                );
                iced::Task::none()
            }
            Message::LockCreated(id) => {
                state.windows.insert(
                    id,
                    WindowState {
                        kind: WindowKind::Corner {
                            name: "lockscreen".into(),
                        },
                    },
                );
                iced::Task::none()
            }
            Message::CloseWindow(id) => {
                state.windows.remove(&id);
                iced::Task::none()
            }
            Message::OpenLayerShell(kind) => {
                let id = IcedId::unique();
                state.pending_windows.push((kind, id));
                iced::Task::none()
            }
            Message::OpenBaseWindow(kind) => {
                let id = IcedId::unique();
                state.pending_base_windows.push((kind, id));
                iced::Task::none()
            }
            Message::LockRequested => {
                state.pending_lock = true;
                iced::Task::none()
            }
            Message::AuthResult(success) => {
                if !success {
                    tracing::warn!("auth failed");
                }
                // find and remove lockscreen window
                if let Some(id) = state
                    .windows
                    .iter()
                    .find(|(_, ws)| matches!(&ws.kind, WindowKind::Corner { name } if name == "lockscreen"))
                    .map(|(id, _)| *id)
                {
                    state.windows.remove(&id);
                }
                iced::Task::none()
            }
            Message::ServiceEvent(_) => iced::Task::none(),
            Message::IpcRequest(req) => match req {
                IpcRequest::Lock => {
                    state.pending_lock = true;
                    iced::Task::none()
                }
                IpcRequest::Quit => std::process::exit(0),
                _ => iced::Task::none(),
            },
            Message::SwitchWorkspace(_) | Message::MediaPlayPause(_) => iced::Task::none(),
            Message::ShellEvent(_) => iced::Task::none(),
            Message::NewLayerShell(_, _)
            | Message::NewBaseWindow(_, _)
            | Message::DoLock
            | Message::DoUnlock
            | Message::RemoveWindow(_) => iced::Task::none(),
        }
    }

    fn view<'a>(
        &self,
        state: &'a State,
        window: IcedId,
    ) -> iced::Element<'a, Message, Theme, iced_wgpu::Renderer> {
        if let Some(ws) = state.windows.get(&window) {
            match ws.kind {
                WindowKind::Bar { monitor, bar_id } => crate::windows::bar::view(monitor, bar_id, state),
                _ => iced::widget::text(format!("{ws:?}")).into(),
            }
        } else {
            iced::widget::text("zex-shell").into()
        }
    }

    fn theme(&self, state: &State, _window: IcedId) -> Option<Theme> {
        Some(state.theme.clone())
    }

    fn subscription(&self, _state: &State) -> iced::Subscription<Message> {
        // ShellEvent forwarding will be added once iced_wayland_subscriber exposes shell channel.
        //! For now keep subscription empty so `cargo check` passes in headless CI.
        iced::Subscription::none()
    }

    fn settings(&self) -> iced::Settings {
        iced::Settings::default()
    }

    fn window(&self) -> Option<iced::window::Settings> {
        None
    }
}
