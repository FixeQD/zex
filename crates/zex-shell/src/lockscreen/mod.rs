//! Lockscreen component: PAM-authenticated fullscreen lock windows

mod auth;
mod ui;

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use flume::Receiver;
use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use relm4::prelude::*;
use zeroize::Zeroizing;
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_core::wallpaper::WallpaperState;
use zex_services::lockscreen::LockEvent;

use crate::shared;
use crate::widgets::SharedSettings;
use ui::LockWindow;

pub const LOCKSCREEN_CSS_SCSS: &str = include_str!("../../assets/css/lockscreen.scss");

#[derive(Debug)]
pub enum LockMsg {
    Locked,
    SettingsChanged(Box<zex_core::Settings>),
    MonitorsChanged,
    Tick,
    Submit { monitor: usize },
    AuthResult { monitor: usize, ok: bool },
    Cancel { monitor: usize },
}

pub struct Lockscreen {
    windows: HashMap<usize, LockWindow>,
    settings: SharedSettings,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
    _provider: Option<gtk4::CssProvider>,
    _events: Receiver<LockEvent>,
    locked: bool,
    clock_source: Option<glib::SourceId>,
    user: String,
}

impl Lockscreen {
    fn sync_monitors(&mut self, display: &gdk::Display, sender: &ComponentSender<Self>) {
        let monitors = shared::monitors(display);
        let present: std::collections::HashSet<usize> = (0..monitors.len()).collect();
        self.windows.retain(|idx, _| present.contains(idx));
        for (idx, monitor) in monitors.into_iter().enumerate() {
            if self.windows.contains_key(&idx) {
                continue;
            }
            let user = self.user.clone();
            let submit = {
                let sender = sender.clone();
                move || {
                    let _ = sender.input_sender().send(LockMsg::Submit { monitor: idx });
                }
            };
            let cancel = {
                let sender = sender.clone();
                move || {
                    let _ = sender.input_sender().send(LockMsg::Cancel { monitor: idx });
                }
            };
            let window = LockWindow::new(idx, &monitor, user, submit, cancel);
            self.windows.insert(idx, window);
        }
    }

    fn lock(&mut self, sender: &ComponentSender<Self>) {
        if self.locked {
            return;
        }
        self.locked = true;
        if let Some(display) = gdk::Display::default() {
            self.sync_monitors(&display, sender);
        }
        let (path, blur) = self.wallpaper_target();
        for window in self.windows.values_mut() {
            window.apply_wallpaper(&path, blur);
            window.refresh_clock(&self.settings.lock().expect("settings mutex poisoned"));
            window.clear_state();
            window.focus();
        }
    }

    /// Current wallpaper path + blur flag from the settings snapshot
    fn wallpaper_target(&self) -> (Option<PathBuf>, bool) {
        let settings = self.settings.lock().expect("settings mutex poisoned");
        let path = WallpaperState::from_settings(&settings).resolve();
        let blur = settings.services.lockscreen.blur;
        (path, blur)
    }

    /// Unlock: stop the clock and close every lock window
    fn unlock(&mut self) {
        self.locked = false;
        if let Some(source) = self.clock_source.take() {
            source.remove();
        }
        self.windows.clear();
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Lockscreen {
    type Init = (SettingsStore, Receiver<LockEvent>);
    type Input = LockMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        (store, events): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let settings = Rc::new(Mutex::new(store.get().clone()));
        let mut model = Self {
            windows: HashMap::new(),
            settings,
            subscription: None,
            monitors_model: None,
            _provider: None,
            _events: events,
            locked: false,
            clock_source: None,
            user: auth::current_user(),
        };

        if let Some(display) = gdk::Display::default() {
            model._provider = Some(shared::install_css_provider(LOCKSCREEN_CSS_SCSS));
            let monitors_model =
                shared::watch_monitors(&display, sender.input_sender().clone(), || {
                    LockMsg::MonitorsChanged
                });
            model.monitors_model = Some(monitors_model);
        }

        let subscription = shared::subscribe_settings(&store, sender.input_sender().clone(), |s| {
            LockMsg::SettingsChanged(Box::new(s.clone()))
        });
        model.subscription = Some(subscription);

        // Lock events from the session bus flow into the GTK loop
        let lock_sender = sender.input_sender().clone();
        let events_rx = model._events.clone();
        std::thread::spawn(move || {
            while let Ok(event) = events_rx.recv() {
                match event {
                    LockEvent::Locked => {
                        let _ = lock_sender.send(LockMsg::Locked);
                    }
                }
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LockMsg::Locked => {
                tracing::info!("lock requested");
                self.lock(&sender);
                self.start_clock(sender);
            }
            LockMsg::SettingsChanged(snapshot) => {
                *self.settings.lock().expect("settings mutex poisoned") = *snapshot;
                if self.locked {
                    let (path, blur) = self.wallpaper_target();
                    let settings = self
                        .settings
                        .lock()
                        .expect("settings mutex poisoned")
                        .clone();
                    for window in self.windows.values_mut() {
                        window.apply_wallpaper(&path, blur);
                        window.refresh_clock(&settings);
                    }
                }
            }
            LockMsg::MonitorsChanged => {
                if self.locked
                    && let Some(display) = gdk::Display::default()
                {
                    self.sync_monitors(&display, &sender);
                }
            }
            LockMsg::Tick => {
                if !self.locked {
                    return;
                }
                let settings = self
                    .settings
                    .lock()
                    .expect("settings mutex poisoned")
                    .clone();
                for window in self.windows.values_mut() {
                    window.refresh_clock(&settings);
                }
            }
            LockMsg::Submit { monitor } => {
                let Some(window) = self.windows.get_mut(&monitor) else {
                    return;
                };
                if !self.locked || window.busy {
                    return;
                }
                let password = Zeroizing::new(window.entry.text().to_string());
                window.entry.set_text("");
                window.busy = true;
                let user = window.user.clone();
                let auth_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let ok = auth::authenticate(&user, password);
                    let _ = auth_sender.send(LockMsg::AuthResult { monitor, ok });
                });
            }
            LockMsg::AuthResult { monitor, ok } => {
                let Some(window) = self.windows.get_mut(&monitor) else {
                    return;
                };
                window.busy = false;
                if ok {
                    tracing::info!("session unlocked");
                    self.unlock();
                } else {
                    window.show_error("Incorrect password");
                    window.entry.set_text("");
                    window.entry.grab_focus();
                }
            }
            LockMsg::Cancel { monitor } => {
                if let Some(window) = self.windows.get_mut(&monitor) {
                    window.clear_state();
                    window.focus();
                }
            }
        }
    }
}

impl Lockscreen {
    /// 1 s tick driving the clock labels while locked
    fn start_clock(&mut self, sender: ComponentSender<Self>) {
        if self.clock_source.is_some() {
            return;
        }
        let id = glib::timeout_add_local(Duration::from_secs(1), move || {
            sender.input(LockMsg::Tick);
            glib::ControlFlow::Continue
        });
        self.clock_source = Some(id);
    }
}
