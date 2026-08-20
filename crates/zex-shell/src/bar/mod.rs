//! Bars shell component

pub mod layout;
pub mod styles;
pub mod widgets;
mod window;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Mutex;

use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use relm4::prelude::*;
use relm4::{Component, ComponentController};
use zex_core::store::Subscription;
use zex_core::{Settings, SettingsStore};
use zex_launcher::apps::{self, AppInfo, PinnedApps};
use zex_services::audio::VolumeControl;
use zex_services::audio::volume::{VolumeState, spawn_volume_monitor};
use zex_services::compositor::{self, CompositorEvent};
use zex_services::mpris::{self, MprisEvent, MprisPlayer};
use zex_services::tray::{SystemTray, TrayEvent, TrayItem};
use zex_services::upower::{Battery, Upower};

use crate::shared;
use crate::shared::ActionHandles;
use crate::widgets::{DockDeps, SharedSettings, Widgets};
use window::{BarMsg, BarWindow, BarWindowInit};

pub const BAR_CSS_SCSS: &str = include_str!("../../assets/css/bar.scss");

/// Both bar instances hosted on every monitor
const BAR_IDS: [u8; 2] = [0, 1];

#[derive(Debug)]
pub enum BarsMsg {
    SettingsChanged(Box<Settings>),
    MonitorsChanged,
    Compositor(CompositorEvent),
    Media(MprisEvent),
    Apps(apps::Change),
    Pins(Vec<String>),
    Tray(TrayEvent),
    Batteries(Vec<Battery>),
    Volume(VolumeState),
}

/// Commands routed to the status runtime thread (tray + power events)
pub enum StatusCommand {
    TrayActivate {
        service: String,
        x: i32,
        y: i32,
    },
    TrayMenu {
        service: String,
        reply: flume::Sender<Vec<zex_services::tray::MenuEntry>>,
    },
    TrayMenuAction {
        service: String,
        id: i32,
    },
}

pub struct Bars {
    settings: SharedSettings,
    widgets_by_monitor: HashMap<usize, Rc<Widgets>>,
    windows: HashMap<(usize, u8), relm4::component::Connector<BarWindow>>,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
    _provider: Option<gtk4::CssProvider>,
    /// Active window manager backend, used for state queries on events
    compositor: Option<Rc<dyn compositor::Compositor>>,
    /// Live MPRIS player snapshot, fed by the media event thread
    players: HashMap<String, MprisPlayer>,
    /// Routes player commands to the MPRIS runtime thread
    media_cmds: flume::Sender<String>,
    /// Routes tray and battery commands to the status runtime thread
    status_cmds: flume::Sender<StatusCommand>,
    /// Live application catalog, refreshed on launcher changes
    apps: Vec<AppInfo>,
    /// Pinned app ids, watched for reordering and quick-centering
    pins: Rc<PinnedApps>,
    /// Shared overlay actions: launcher/quick-center toggles
    actions: Rc<ActionHandles>,
    /// Live tray item snapshot, fed by the status thread
    tray_items: Vec<TrayItem>,
    /// Live battery snapshot, fed by the status thread
    batteries: Vec<Battery>,
    /// Live quick settings volume level, fed by the volume thread
    volume: VolumeState,
    /// Sends clamp/scroll commands to the volume thread
    volume_control: VolumeControl,
}

impl Bars {
    /// Registry for one monitor; overlay buttons route through the shared handles
    fn build_registry(&self) -> Widgets {
        let settings = Rc::clone(&self.settings);
        let switcher = self.compositor.clone();
        let media_control = crate::bar::widgets::MprisControl::new(self.media_cmds.clone());
        let actions = Rc::clone(&self.actions);
        let quickcenter = Rc::clone(&actions);
        Widgets::build(
            settings,
            move || actions.toggle_launcher(),
            switcher,
            media_control,
            self.display_offset(),
            DockDeps {
                on_quickcenter: Rc::new(move || quickcenter.toggle_quickcenter()),
                tray: crate::bar::widgets::systeminfotray::TrayControl::new(
                    self.status_cmds.clone(),
                ),
                volume: self.volume_control.clone(),
                apps: self.apps.clone(),
                pins: Rc::clone(&self.pins),
            },
        )
    }

    /// Raw-id to display-id offset, mirroring each backend's id space
    fn display_offset(&self) -> i32 {
        match self.compositor.as_ref().map(|c| c.name()) {
            Some("Niri") => 1,
            _ => 0,
        }
    }

    /// Fresh compositor state pushed into every monitor registry
    fn push_compositor(&self) {
        let Some(compositor) = self.compositor.as_ref() else {
            return;
        };
        let workspaces = compositor.workspaces().unwrap_or_default();
        let windows = compositor.windows().unwrap_or_default();
        let active = compositor.active_window().ok().flatten();
        for registry in self.widgets_by_monitor.values() {
            registry.on_compositor(&workspaces, &windows, active.as_ref());
        }
    }

    /// Player snapshot pushed into every monitor registry
    fn push_media(&self) {
        let players: Vec<MprisPlayer> = self.players.values().cloned().collect();
        for registry in self.widgets_by_monitor.values() {
            registry.on_media(&players);
        }
    }

    /// Applications catalog pushed into every monitor registry
    fn push_apps(&self) {
        for registry in self.widgets_by_monitor.values() {
            registry.on_apps(&self.apps);
        }
    }

    /// Pin set pushed into every monitor registry (the registry reads it live)
    fn push_pins(&self) {
        for registry in self.widgets_by_monitor.values() {
            registry.on_pins();
        }
    }

    /// Tray items pushed into every monitor registry
    fn push_tray(&self) {
        for registry in self.widgets_by_monitor.values() {
            registry.on_tray(&self.tray_items);
        }
    }

    /// Battery snapshot pushed into every monitor registry
    fn push_batteries(&self) {
        for registry in self.widgets_by_monitor.values() {
            registry.on_batteries(&self.batteries);
        }
    }

    /// Volume state pushed into every monitor registry
    fn push_volume(&self) {
        for registry in self.widgets_by_monitor.values() {
            registry.on_volume(self.volume.volume, self.volume.muted);
        }
    }

    /// Reconcile the window set with the current monitor list
    fn sync_monitors(&mut self, display: &gdk::Display) {
        let monitors = shared::monitors(display);
        let present: HashSet<(usize, u8)> = (0..monitors.len())
            .flat_map(|idx| BAR_IDS.iter().map(move |bar_id| (idx, *bar_id)))
            .collect();
        self.windows.retain(|key, _| present.contains(key));

        for (idx, monitor) in monitors.into_iter().enumerate() {
            let widgets = match self.widgets_by_monitor.get(&idx) {
                Some(existing) => Rc::clone(existing),
                None => {
                    let built = Rc::new(self.build_registry());
                    self.widgets_by_monitor.insert(idx, Rc::clone(&built));
                    built
                }
            };
            for bar_id in BAR_IDS {
                if self.windows.contains_key(&(idx, bar_id)) {
                    continue;
                }
                let controller = BarWindow::builder().launch(BarWindowInit {
                    monitor_idx: idx,
                    monitor: monitor.clone(),
                    bar_id,
                    settings: Rc::clone(&self.settings),
                    widgets: Rc::clone(&widgets),
                });
                self.windows.insert((idx, bar_id), controller);
            }
        }
    }

    /// Push a refresh to every live bar window
    fn emit_refresh(&mut self) {
        for controller in self.windows.values() {
            controller.emit(BarMsg::Refresh);
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Bars {
    type Init = (SettingsStore, Rc<ActionHandles>);
    type Input = BarsMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        (store, actions): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let settings = Rc::new(Mutex::new(store.get().clone()));

        let (status_cmds, status_cmd_rx) = flume::unbounded();
        let mut model = Self {
            settings: Rc::clone(&settings),
            widgets_by_monitor: HashMap::new(),
            windows: HashMap::new(),
            subscription: None,
            monitors_model: None,
            _provider: None,
            compositor: None,
            players: HashMap::new(),
            media_cmds: flume::unbounded().0,
            status_cmds,
            apps: Vec::new(),
            pins: Rc::new(PinnedApps::load(None)),
            actions,
            tray_items: Vec::new(),
            batteries: Vec::new(),
            volume: VolumeState::default(),
            volume_control: VolumeControl::default(),
        };

        // Compositor backend: events stream into the GTK loop, no polling
        if let Some(compositor) = compositor::detect_compositor() {
            let compositor: Rc<dyn compositor::Compositor> = Rc::from(compositor);
            let events = compositor.events();
            let sender = sender.clone();
            std::thread::spawn(move || {
                while let Ok(event) = events.recv() {
                    sender.input(BarsMsg::Compositor(event));
                }
            });
            tracing::info!("bars bound to {}", compositor.name());
            model.compositor = Some(compositor);
        } else {
            tracing::warn!("no compositor backend detected; workspaces and window info stay off");
        }

        // MPRIS on a small dedicated runtime: events out, play/pause in
        let media_out = sender.clone();
        let (media_cmds, media_cmd_rx) = flume::unbounded();
        model.media_cmds = media_cmds;
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    tracing::warn!("media runtime unavailable: {err}");
                    return;
                }
            };
            runtime.block_on(async {
                let Ok(connection) = zbus::Connection::session().await else {
                    tracing::warn!("session bus unavailable; media stays off");
                    return;
                };
                let Ok(media) = mpris::Mpris::connect(connection).await else {
                    tracing::warn!("media service unavailable");
                    return;
                };
                loop {
                    tokio::select! {
                        command = media_cmd_rx.recv_async() => {
                            let Ok(player) = command else { break };
                            if let Err(err) = media.play_pause(&player).await {
                                tracing::debug!("media play/pause failed: {err:#}");
                            }
                        }
                        event = media.events().recv_async() => {
                            let Ok(event) = event else { break };
                            media_out.input(BarsMsg::Media(event));
                        }
                    }
                }
            });
        });

        // Tray host + battery events on a small dedicated runtime: commands in, events out
        let status_out = sender.clone();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    tracing::warn!("status runtime unavailable: {err}");
                    return;
                }
            };
            runtime.block_on(async {
                let Ok(host_conn) = zbus::Connection::session().await else {
                    tracing::warn!("session bus unavailable; tray stays off");
                    return;
                };
                let Ok(tray) = SystemTray::host(host_conn).await else {
                    tracing::warn!("tray service unavailable");
                    return;
                };
                let Ok(upower_conn) = zbus::Connection::session().await else {
                    tracing::warn!("session bus unavailable; battery stays off");
                    return;
                };
                let Ok(upower) = Upower::connect(upower_conn).await else {
                    tracing::warn!("upower service unavailable");
                    return;
                };
                if let Ok(batteries) = upower.batteries().await {
                    status_out.input(BarsMsg::Batteries(batteries));
                }
                let tray_events = tray.events().clone();
                let battery_events = upower.events().clone();
                loop {
                    tokio::select! {
                        command = status_cmd_rx.recv_async() => {
                            let Ok(command) = command else { break };
                            match command {
                                StatusCommand::TrayActivate { service, x, y } => {
                                    if let Err(err) = tray.activate(&service, x, y).await {
                                        tracing::debug!("tray activate failed: {err:#}");
                                    }
                                }
                                StatusCommand::TrayMenu { service, reply } => {
                                    let entries = tray.menu(&service).await.unwrap_or_default();
                                    let _ = reply.send(entries);
                                }
                                StatusCommand::TrayMenuAction { service, id } => {
                                    let _ = tray.menu_action(&service, id).await;
                                }
                            }
                        }
                        event = tray_events.recv_async() => {
                            let Ok(event) = event else { break };
                            status_out.input(BarsMsg::Tray(event));
                        }
                        event = battery_events.recv_async() => {
                            let Ok(_event) = event else { break };
                            if let Ok(batteries) = upower.batteries().await {
                                status_out.input(BarsMsg::Batteries(batteries));
                            }
                        }
                    }
                }
            });
        });

        // Quick settings volume: clamps into PipeWire, clamped states come back as events
        let (volume_events, volume_event_rx) = flume::unbounded();
        let volume_state = std::sync::Arc::new(std::sync::Mutex::new(VolumeState::default()));
        model.volume_control = spawn_volume_monitor(
            volume_state,
            tokio::sync::oneshot::channel().0,
            volume_events,
        );
        let volume_out = sender.clone();
        std::thread::spawn(move || {
            while let Ok(state) = volume_event_rx.recv() {
                volume_out.input(BarsMsg::Volume(state));
            }
        });

        // Applications catalog: initial scan, then re-scan on file change events
        model.apps = match apps::load_apps(Some(&apps::default_store_path())) {
            Ok(catalog) => catalog,
            Err(err) => {
                tracing::warn!("applications scan failed: {err:#}");
                Vec::new()
            }
        };
        if let Ok(watchdog) = apps::Watchdog::start() {
            let watchdog_sender = sender.clone();
            std::thread::spawn(move || {
                loop {
                    let changes = watchdog.next(std::time::Duration::from_secs(30));
                    for change in changes {
                        watchdog_sender.input(BarsMsg::Apps(change));
                    }
                }
            });
        }

        // Pin changes (reordered in the launcher) rebuild the task dock
        let pins_rx = model.pins.changes();
        let pins_sender = sender.clone();
        std::thread::spawn(move || {
            while let Ok(ids) = pins_rx.recv() {
                pins_sender.input(BarsMsg::Pins(ids));
            }
        });

        if let Some(display) = gdk::Display::default() {
            model._provider = Some(shared::install_css_provider(BAR_CSS_SCSS));
            let monitors_model =
                shared::watch_monitors(&display, sender.input_sender().clone(), || {
                    BarsMsg::MonitorsChanged
                });
            model.monitors_model = Some(monitors_model);
            model.sync_monitors(&display);
            model.push_compositor();
            model.push_media();
            model.push_apps();
            model.push_tray();
            model.push_batteries();
            model.push_volume();
        } else {
            tracing::warn!("no display available; bars will stay hidden");
        }

        // Live bridge: settings updates flow into the GTK loop as refreshes
        let subscription = shared::subscribe_settings(&store, sender.input_sender().clone(), |s| {
            BarsMsg::SettingsChanged(Box::new(s.clone()))
        });
        model.subscription = Some(subscription);

        model.emit_refresh();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            BarsMsg::SettingsChanged(snapshot) => {
                *self.settings.lock().expect("settings mutex poisoned") = *snapshot;
                self.emit_refresh();
            }
            BarsMsg::MonitorsChanged => {
                if let Some(display) = gdk::Display::default() {
                    self.sync_monitors(&display);
                    self.emit_refresh();
                }
            }
            BarsMsg::Compositor(event) => {
                tracing::trace!("compositor event: {event:?}");
                self.push_compositor();
            }
            BarsMsg::Media(event) => {
                match event {
                    MprisEvent::PlayerAdded(player) => {
                        self.players.insert(player.name.clone(), player);
                    }
                    MprisEvent::PlayerRemoved(name) => {
                        self.players.remove(&name);
                    }
                    MprisEvent::PlayerChanged(name, info) => {
                        if let Some(player) = self.players.get_mut(&name) {
                            player.info = info;
                        }
                    }
                }
                self.push_media();
            }
            BarsMsg::Apps(_change) => {
                self.apps = match apps::load_apps(Some(&apps::default_store_path())) {
                    Ok(catalog) => catalog,
                    Err(err) => {
                        tracing::warn!("applications re-scan failed: {err:#}");
                        return;
                    }
                };
                self.push_apps();
            }
            BarsMsg::Pins(_ids) => {
                self.push_pins();
                self.push_apps();
            }
            BarsMsg::Tray(event) => {
                self.apply_tray_event(&event);
                self.push_tray();
            }
            BarsMsg::Batteries(batteries) => {
                self.batteries = batteries;
                self.push_batteries();
            }
            BarsMsg::Volume(state) => {
                self.volume = state;
                self.push_volume();
            }
        }
    }
}

impl Bars {
    fn apply_tray_event(&mut self, event: &TrayEvent) {
        match event {
            TrayEvent::ItemAdded(item) => {
                self.tray_items
                    .retain(|existing| existing.service != item.service);
                self.tray_items.push(item.clone());
            }
            TrayEvent::ItemRemoved(service) => {
                self.tray_items
                    .retain(|existing| &existing.service != service);
            }
            TrayEvent::ItemChanged(service, icon) => {
                if let Some(item) = self
                    .tray_items
                    .iter_mut()
                    .find(|existing| &existing.service == service)
                {
                    item.icon = icon.clone();
                }
            }
        }
    }
}
