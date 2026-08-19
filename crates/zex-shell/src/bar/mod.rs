//! Bars shell component

pub mod layout;
pub mod styles;
pub mod widgets;
mod window;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Mutex;

use anyhow::{Context, Result};
use gtk4::gdk;
use gtk4::gdk::prelude::*;
use gtk4::gio;
use gtk4::prelude::*;
use relm4::prelude::*;
use relm4::{Component, ComponentController};
use zex_core::store::Subscription;
use zex_core::{Settings, SettingsStore};
use zex_services::compositor::{self, CompositorEvent};
use zex_services::mpris::{self, MprisEvent, MprisPlayer};

use crate::widgets::{SharedSettings, Widgets};
use window::{BarMsg, BarWindow, BarWindowInit};

pub const BAR_CSS_SCSS: &str = include_str!("../../assets/css/bar.scss");

fn install_bar_css(provider: &gtk4::CssProvider) -> Result<()> {
    let css = grass::from_string(BAR_CSS_SCSS, &grass::Options::default())
        .context("compiling bar css")?;
    provider.load_from_string(&css);
    Ok(())
}

/// Both bar instances hosted on every monitor
const BAR_IDS: [u8; 2] = [0, 1];

#[derive(Debug)]
pub enum BarsMsg {
    SettingsChanged(Box<Settings>),
    MonitorsChanged,
    Compositor(CompositorEvent),
    Media(MprisEvent),
}

pub struct Bars {
    settings: SharedSettings,
    widgets_by_monitor: HashMap<usize, Rc<Widgets>>,
    windows: HashMap<(usize, u8), relm4::component::Connector<BarWindow>>,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
    _provider: gtk4::CssProvider,
    /// Active window manager backend, used for state queries on events
    compositor: Option<Rc<dyn compositor::Compositor>>,
    /// Live MPRIS player snapshot, fed by the media event thread
    players: HashMap<String, MprisPlayer>,
    /// Routes player commands to the MPRIS runtime thread
    media_cmds: flume::Sender<String>,
}

impl Bars {
    /// Registry for one monitor; the launcher button is a stub until the window manager land in a later commit
    fn build_registry(&self) -> Widgets {
        let settings = Rc::clone(&self.settings);
        let switcher = self.compositor.clone();
        let media_control = crate::bar::widgets::MprisControl::new(self.media_cmds.clone());
        Widgets::build(
            settings,
            || {
                tracing::info!("launcher requested; window manager lands in a later commit");
            },
            switcher,
            media_control,
            self.display_offset(),
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

    /// Reconcile the window set with the current monitor list
    fn sync_monitors(&mut self, display: &gdk::Display) {
        let monitors: Vec<gdk::Monitor> =
            display.monitors().iter().filter_map(Result::ok).collect();
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
    type Init = SettingsStore;
    type Input = BarsMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        store: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let settings = Rc::new(Mutex::new(store.get().clone()));

        let provider = gtk4::CssProvider::new();
        let mut model = Self {
            settings: Rc::clone(&settings),
            widgets_by_monitor: HashMap::new(),
            windows: HashMap::new(),
            subscription: None,
            monitors_model: None,
            _provider: provider,
            compositor: None,
            players: HashMap::new(),
            media_cmds: flume::unbounded().0,
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

        if let Some(display) = gdk::Display::default() {
            if let Err(err) = install_bar_css(&model._provider) {
                tracing::warn!("{err:#}");
            }
            gtk4::style_context_add_provider_for_display(
                &display,
                &model._provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            let monitors_model = display.monitors();
            monitors_model.connect_items_changed({
                let sender = sender.clone();
                move |_, _, _, _| sender.input(BarsMsg::MonitorsChanged)
            });
            model.monitors_model = Some(monitors_model);
            model.sync_monitors(&display);
            model.push_compositor();
            model.push_media();
        } else {
            tracing::warn!("no display available; bars will stay hidden");
        }

        // Live bridge: settings updates flow into the GTK loop as refreshes
        let bridge_tx = sender.input_sender().clone();
        let subscription = store.subscribe(move |snapshot| {
            let _ = bridge_tx.send(BarsMsg::SettingsChanged(Box::new(snapshot.clone())));
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
        }
    }
}
