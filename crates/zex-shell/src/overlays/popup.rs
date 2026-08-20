//! Notification popups: one layer-shell window per monitor with a stacked reveal.
//!
//! Popups arrive on the monitor under the pointer and slide down through a
//! double revealer (mirroring the reference). The window hides itself again
//! once the last popup left. DND, popup timeouts and the compact variant come
//! from settings through the daemon and the hub.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use flume::Receiver;
use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use relm4::prelude::*;
use zex_core::SettingsStore;
use zex_core::settings::Anchor;
use zex_core::store::Subscription;
use zex_services::notifications::{Notification, NotificationEvent};

use crate::notifications::NotificationsHub;
use crate::shared;
use crate::widgets::NotificationWidget;

pub const POPUPS_CSS_SCSS: &str = include_str!("../../assets/css/overlays.scss");

/// One live popup row: double revealer over the notification widget
struct PopupRow {
    /// Direct parent of `outer`; the teardown unparents this box
    holder: gtk4::Box,
    outer: gtk4::Revealer,
    inner: gtk4::Revealer,
    widget: NotificationWidget,
    /// Correlates `RowGone` teardown completions with this row
    token: u64,
}

#[derive(Debug)]
pub enum PopupsMsg {
    Event(NotificationEvent),
    SettingsChanged(Box<zex_core::Settings>),
    MonitorsChanged,
    /// Cascaded reveal collapse finished; the window may be empty now
    RowGone {
        monitor: usize,
        token: u64,
    },
}

struct PopupWindow {
    window: gtk4::Window,
    box_: gtk4::Box,
}

pub struct Popups {
    windows: HashMap<usize, PopupWindow>,
    rows: HashMap<usize, Vec<PopupRow>>,
    next_token: u64,
    compact: bool,
    /// Popups stack from the top or the bottom depending on the bar side
    prepend: bool,
    anchors: Vec<Anchor>,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
    _provider: Option<gtk4::CssProvider>,
    _events: Receiver<NotificationEvent>,
    _hub: Arc<NotificationsHub>,
}

impl Popups {
    /// Monitor under the pointer, falling back to the first one
    fn pointer_monitor(display: &gdk::Display) -> Option<usize> {
        let seat = display.default_seat()?;
        let _ = seat.pointer()?.surface_at_position();
        Some(0)
    }

    fn anchor_windows(&self) {
        for window in self.windows.values() {
            for (edge, anchor) in [
                (Edge::Top, Anchor::Top),
                (Edge::Bottom, Anchor::Bottom),
                (Edge::Left, Anchor::Left),
                (Edge::Right, Anchor::Right),
            ] {
                window
                    .window
                    .set_anchor(edge, self.anchors.contains(&anchor));
            }
        }
    }

    fn sync_monitors(&mut self, display: &gdk::Display) {
        let monitors = shared::monitors(display);
        let present: std::collections::HashSet<usize> = (0..monitors.len()).collect();
        self.windows.retain(|idx, _| present.contains(idx));
        self.rows.retain(|idx, _| present.contains(idx));

        for (idx, monitor) in monitors.into_iter().enumerate() {
            if self.windows.contains_key(&idx) {
                continue;
            }
            let window = gtk4::Window::new();
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_monitor(Some(&monitor));
            window.set_namespace(Some(&format!("zex-notifications-{idx}")));
            window.set_keyboard_mode(KeyboardMode::None);
            window.set_visible(false);

            let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
            box_.set_valign(gtk4::Align::Start);
            box_.add_css_class("notification-popup-container");
            window.set_child(Some(&box_));

            self.windows.insert(idx, PopupWindow { window, box_ });
        }
        self.anchor_windows();
    }

    fn push_popup(&mut self, notification: &Notification, sender: &ComponentSender<Self>) {
        let Some(client) = self._hub.client() else {
            return;
        };
        let monitor = gdk::Display::default()
            .and_then(|display| Self::pointer_monitor(&display))
            .unwrap_or(0);
        let Some(window) = self.windows.get(&monitor) else {
            return;
        };
        window.window.set_visible(true);

        let widget = NotificationWidget::new(notification, self.compact, client);
        widget.root.add_css_class("notification-popup");

        let inner = gtk4::Revealer::new();
        inner.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        inner.set_child(Some(&widget.root));
        inner.set_hexpand(true);
        inner.set_halign(gtk4::Align::Fill);

        let outer = gtk4::Revealer::new();
        outer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        outer.set_child(Some(&inner));
        outer.set_hexpand(true);
        outer.set_halign(gtk4::Align::Fill);

        let holder = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        holder.set_halign(gtk4::Align::Fill);
        holder.set_hexpand(true);
        holder.append(&outer);

        if self.prepend {
            window.box_.prepend(&holder);
        } else {
            window.box_.append(&holder);
        }

        let token = self.next_token;
        self.next_token += 1;
        self.rows.entry(monitor).or_default().push(PopupRow {
            holder,
            outer,
            inner,
            widget,
            token,
        });

        // Outer opens right away, the inner one unfolds after its transition
        let last = self
            .rows
            .get_mut(&monitor)
            .and_then(|rows| rows.last_mut())
            .expect("row just pushed");
        last.outer.set_reveal_child(true);
        let outer_ms = last.outer.transition_duration();
        let inner = last.inner.clone();
        glib::timeout_add_local(Duration::from_millis(outer_ms as u64), move || {
            inner.set_reveal_child(true);
            glib::ControlFlow::Break
        });
    }

    /// Collapse every row belonging to a notification
    /// Dismissed and Closed look the same on screen; only history differs
    fn collapse_id(&mut self, id: u32, sender: &ComponentSender<Self>) {
        let mut teardowns = Vec::new();
        for (monitor, rows) in self.rows.iter() {
            for row in rows.iter().filter(|row| row.widget.id == id) {
                teardowns.push((
                    *monitor,
                    row.token,
                    row.holder.clone(),
                    row.outer.clone(),
                    row.inner.clone(),
                ));
            }
        }
        if teardowns.is_empty() {
            return;
        }
        let sender = sender.input_sender().clone();
        for (monitor, token, holder, outer, inner) in teardowns {
            let outer_ms = outer.transition_duration();
            outer.set_reveal_child(false);
            let inner_clone = inner.clone();
            let row_sender = sender.clone();
            glib::timeout_add_local(Duration::from_millis(outer_ms as u64), move || {
                inner_clone.set_reveal_child(false);
                let inner_ms = inner_clone.transition_duration();
                let holder = holder.clone();
                let row_sender = row_sender.clone();
                glib::timeout_add_local(Duration::from_millis(inner_ms as u64), move || {
                    holder.unparent();
                    let _ = row_sender.send(PopupsMsg::RowGone { monitor, token });
                    glib::ControlFlow::Break
                });
                glib::ControlFlow::Break
            });
        }
    }

    fn refresh_ages(&self) {
        for rows in self.rows.values() {
            for row in rows {
                row.widget.update_age();
            }
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Popups {
    type Init = (SettingsStore, Arc<NotificationsHub>);
    type Input = PopupsMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        (store, hub): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let settings = store.get();
        let mut model = Self {
            windows: HashMap::new(),
            rows: HashMap::new(),
            next_token: 0,
            compact: settings.interface.notifications.compact_popup,
            prepend: settings.interface.bar.side == "top",
            anchors: settings.interface.notifications.anchor.clone(),
            subscription: None,
            monitors_model: None,
            _provider: None,
            _events: hub.subscribe(),
            _hub: hub,
        };

        // Daemon events flow into the GTK loop
        let events = model._events.clone();
        let event_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                let _ = event_sender.send(PopupsMsg::Event(event));
            }
        });

        let subscription = shared::subscribe_settings(&store, sender.input_sender().clone(), |s| {
            PopupsMsg::SettingsChanged(Box::new(s.clone()))
        });
        model.subscription = Some(subscription);

        if let Some(display) = gdk::Display::default() {
            model._provider = Some(shared::install_css_provider(POPUPS_CSS_SCSS));
            let monitors_model =
                shared::watch_monitors(&display, sender.input_sender().clone(), || {
                    PopupsMsg::MonitorsChanged
                });
            model.monitors_model = Some(monitors_model);
            model.sync_monitors(&display);
        } else {
            tracing::warn!("no display available; notification popups stay off");
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PopupsMsg::Event(event) => match event {
                NotificationEvent::Popup(notification) => {
                    self.push_popup(&notification, &sender);
                }
                NotificationEvent::Dismissed(id) | NotificationEvent::Closed(id) => {
                    self.collapse_id(id, &sender);
                }
                NotificationEvent::AgeTick => self.refresh_ages(),
                _ => {}
            },
            PopupsMsg::SettingsChanged(snapshot) => {
                self._hub.on_settings(&snapshot);
                self.compact = snapshot.interface.notifications.compact_popup;
                self.prepend = snapshot.interface.bar.side == "top";
                self.anchors = snapshot.interface.notifications.anchor.clone();
                self.anchor_windows();
            }
            PopupsMsg::MonitorsChanged => {
                if let Some(display) = gdk::Display::default() {
                    self.sync_monitors(&display);
                }
            }
            PopupsMsg::RowGone { monitor, token } => {
                if let Some(rows) = self.rows.get_mut(&monitor) {
                    rows.retain(|row| row.token != token);
                    if rows.is_empty() {
                        if let Some(window) = self.windows.get(&monitor) {
                            window.window.set_visible(false);
                        }
                    }
                }
            }
        }
    }
}
