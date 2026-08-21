use std::collections::HashMap;

use iced::Theme;
use iced::window::Id as IcedId;
use iced_exwlshell::reexport::Anchor;
use zex_core::Settings;
use zex_launcher::ipc::Mode;

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
}
