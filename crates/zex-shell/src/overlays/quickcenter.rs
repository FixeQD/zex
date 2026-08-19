//! Quick center: a 400 px panel with the notification center, quick sliders and
//! the power/settings shortcuts.
//!
//! Anchored next to the hosting bar's edge (or the tray segment), slides in on
//! Niri and pops on other compositors. The tray button toggles it through the
//! shared [`crate::shared::ActionHandles`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use flume::Receiver;
use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4_layer_shell::{KeyboardInteractivity, Layer, LayerShell};
use relm4::prelude::*;
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_services::audio::VolumeControl;
use zex_services::audio::volume::{VolumeState, spawn_volume_monitor};
use zex_services::backlight::Backlight;
use zex_services::compositor;
use zex_services::notifications::{Notification, NotificationEvent};

use crate::m3::{M3Button, M3Shape, M3Size, M3Slider, M3Type};
use crate::notifications::NotificationsHub;
use crate::shared::{self, ActionHandles};
use crate::widgets::NotificationWidget;

pub const QUICK_CENTER_WIDTH: i32 = 400;

#[derive(Debug)]
pub enum QuickCenterMsg {
    Event(NotificationEvent),
    SettingsChanged(Box<zex_core::Settings>),
    MonitorsChanged,
    /// Toggle the panel on the monitor under the pointer
    Toggle,
    /// Close the panel on one monitor
    Close { monitor: usize },
    Volume(VolumeState),
}

/// One row in the notification center list (full notification widget)
struct CenterRow {
    revealer: gtk4::Revealer,
    widget: NotificationWidget,
    id: u32,
}

struct QuickCenterWindow {
    window: gtk4::Window,
    revealer: gtk4::Revealer,
    center_box: gtk4::Box,
    empty_icon: gtk4::Label,
    empty_label: gtk4::Label,
    clear_all: M3Button,
    volume_slider: Option<M3Slider>,
    backlight_slider: Option<M3Slider>,
    rows: HashMap<u32, CenterRow>,
}

/// `true` while code pushes slider values; user handlers then ignore the echo
type SyncFlag = Rc<Cell<bool>>;

/// Live sink volume read by the slider handlers
type SharedVolume = Rc<RefCell<VolumeState>>;

pub struct QuickCenter {
    windows: HashMap<usize, QuickCenterWindow>,
    volume: VolumeState,
    /// Sink state shared with the slider handlers
    shared_volume: SharedVolume,
    syncing: SyncFlag,
    volume_control: VolumeControl,
    suppress_osd: Arc<AtomicBool>,
    backlight: Option<Backlight>,
    /// Slide transition is only available on Niri
    niri: bool,
    /// History loaded at least once (the client appears once the bus is up)
    history_loaded: bool,
    actions: Rc<ActionHandles>,
    sender: relm4::components::ComponentSender<QuickCenter>,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
    _events: Receiver<NotificationEvent>,
    _hub: Arc<NotificationsHub>,
}

impl QuickCenter {
    /// Monitor under the pointer, falling back to the first one
    fn pointer_monitor(display: &gdk::Display) -> Option<usize> {
        let seat = display.default_seat()?;
        let (_surface, x, y) = seat.pointer()?.surface_at_position();
        let monitor = display.monitor_at_point(x, y)?;
        shared::monitors(display).iter().position(|m| m == &monitor)
    }

    /// One volume slider wired to the shared sink state and the OSD suppression flag
    fn volume_slider(&self) -> M3Slider {
        let slider = M3Slider::new(Some("audio-volume-high-symbolic"));
        slider.set_range(0.0, 100.0);
        slider.scale.set_step_increment(1.0);
        slider.scale.set_draw_value(false);
        slider.scale.set_value_pos(gtk4::PositionType::Bottom);

        let shared = Rc::clone(&self.shared_volume);
        let syncing = Rc::clone(&self.syncing);
        let control = self.volume_control.clone();
        let suppress = Arc::clone(&self.suppress_osd);
        slider.connect_value_changed(move |value| {
            if syncing.get() {
                return;
            }
            suppress.store(true, Ordering::Relaxed);
            let suppress_reset = Arc::clone(&suppress);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                suppress_reset.store(false, Ordering::Relaxed);
                glib::ControlFlow::Break
            });

            let mut state = shared.borrow_mut();
            if state.muted {
                control.toggle_mute();
                state.muted = false;
            }
            state.volume = (value / 100.0).clamp(0.0, 1.0);
            drop(state);
            control.set_volume(value as f32 / 100.0);
        });
        slider
    }

    /// One backlight slider writing through sysfs, sharing the OSD suppression flag
    fn backlight_slider(&self, backlight: &Backlight) -> M3Slider {
        let slider = M3Slider::new(Some("display-brightness-symbolic"));
        slider.set_range(0.0, 100.0);
        slider.scale.set_step_increment(1.0);
        slider.scale.set_draw_value(false);
        slider.scale.set_value_pos(gtk4::PositionType::Bottom);

        let syncing = Rc::clone(&self.syncing);
        let suppress = Arc::clone(&self.suppress_osd);
        let device = backlight.clone();
        slider.connect_value_changed(move |value| {
            if syncing.get() {
                return;
            }
            suppress.store(true, Ordering::Relaxed);
            let suppress_reset = Arc::clone(&suppress);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                suppress_reset.store(false, Ordering::Relaxed);
                glib::ControlFlow::Break
            });
            let _ = device.set_percent((value / 100.0).clamp(0.0, 1.0));
        });
        slider
    }

    fn bottom_controls(actions: &Rc<ActionHandles>) -> (gtk4::Box, M3Button) {
        let power = M3Button::new(
            Some("system-shutdown-symbolic"),
            None,
            M3Type::Tonal,
            M3Size::Xs,
            M3Shape::Round,
        );
        let actions_power = Rc::clone(actions);
        power.connect_clicked(move |_| actions_power.toggle_powermenu());

        let settings = M3Button::new(
            Some("preferences-system-symbolic"),
            None,
            M3Type::Tonal,
            M3Size::Xs,
            M3Shape::Round,
        );
        let actions_settings = Rc::clone(actions);
        settings.connect_clicked(move |_| actions_settings.open_settings());

        let clear_all = M3Button::new(
            Some("edit-clear-all-symbolic"),
            Some("Clear all"),
            M3Type::Text,
            M3Size::Xs,
            M3Shape::Round,
        );
        clear_all.button.add_css_class("notification-clear-all");

        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        row.set_hexpand(true);
        row.set_halign(gtk4::Align::Fill);
        row.add_css_class("bottom-controls");
        row.append(&power.button);
        row.append(&settings.button);
        row.append(&clear_all.button);
        (row, clear_all)
    }

    fn new_window(
        &self,
        idx: usize,
        monitor: &gdk::Monitor,
        actions: &Rc<ActionHandles>,
        on_close: Rc<dyn Fn(usize)>,
    ) -> QuickCenterWindow {
        let window = gtk4::Window::new();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_monitor(Some(monitor));
        window.set_namespace(Some(&format!("zex-quick-center-{idx}")));
        window.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        window.set_visible(false);
        window.set_anchor(gtk4_layer_shell::Edge::Top, true);
        window.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
        window.set_anchor(gtk4_layer_shell::Edge::Left, true);
        window.set_anchor(gtk4_layer_shell::Edge::Right, true);

        let center_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        center_box.set_vexpand(true);
        center_box.add_css_class("notification-center-content");
        center_box.set_valign(gtk4::Align::Start);

        let empty_icon = gtk4::Label::new(Some("notifications_off"));
        empty_icon.set_css_classes(&["notification-center-info-icon"]);
        empty_icon.set_visible(false);
        let empty_label = gtk4::Label::new(Some("No notifications"));
        empty_label.set_css_classes(&["notification-center-info-label"]);
        empty_label.set_visible(false);
        center_box.append(&empty_icon);
        center_box.append(&empty_label);

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&center_box));

        let center = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        center.set_vexpand(true);
        center.add_css_class("notification-center");
        center.append(&scroll);

        let (bottom, clear_all) = Self::bottom_controls(actions);
        let hub = Arc::clone(&self._hub);
        clear_all.connect_clicked(move |_| {
            if let Some(client) = hub.client() {
                client.close_all();
            }
        });

        let volume_slider = self.volume_slider();
        let backlight_slider = self.backlight.as_ref().map(|backlight| {
            let slider = self.backlight_slider(backlight);
            slider
        });

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.set_css_classes(&["quick-center"]);
        content.set_width_request(QUICK_CENTER_WIDTH);
        content.append(&center);
        content.append(&volume_slider.container);
        if let Some(slider) = backlight_slider.as_ref() {
            content.append(&slider.container);
        }
        content.append(&bottom);

        // Full-window close button under the panel; clicking anywhere outside it closes
        let close = gtk4::Button::new();
        close.set_can_focus(false);
        close.set_hexpand(true);
        close.set_vexpand(true);

        let revealer = gtk4::Revealer::new();
        revealer.set_child(Some(&content));
        revealer.set_transition_duration(300);
        revealer.set_transition_type(gtk4::RevealerTransitionType::Crossfade);

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&close));
        overlay.add_overlay(&revealer);
        overlay.add_css_class("popup-close");
        window.set_child(Some(&overlay));

        let panel = QuickCenterWindow {
            window,
            revealer,
            center_box,
            empty_icon,
            empty_label,
            clear_all,
            volume_slider: Some(volume_slider),
            backlight_slider,
            rows: HashMap::new(),
        };
        panel

        // Escape closes the panel on this monitor
        let keys = gtk4::EventControllerKey::new();
        let monitor_idx = idx;
        keys.connect_key_pressed(move |_controller, key, _keycode, _state| {
            if key == gtk4::gdk::keys::constants::Escape {
                on_close(monitor_idx);
                gtk4::Propagation::Stop
            } else {
                gtk4::Propagation::Proceed
            }
        });
        panel.window.add_controller(keys);
        panel
    }

    /// Show the empty-state hint or hide it, and mirror the tray's clear-all visibility
    fn refresh_empty(&mut self) -> () {
        for window in self.windows.values_mut() {
            let empty = window.rows.is_empty();
            window.empty_icon.set_visible(empty);
            window.empty_label.set_visible(empty);
            window.clear_all.button.set_visible(!empty);
        }
    }

    /// Load the daemon history into every window (newest first)
    fn load_history(&mut self) {
        let Some(client) = self._hub.client() else {
            return;
        };
        self.history_loaded = true;
        let notifications = client.notifications();
        for notification in notifications {
            for window in self.windows.values_mut() {
                window.prepend_row(&notification, self._hub.clone());
            }
        }
        self.refresh_empty();
    }

    /// Push one live notification into every window's list
    fn prepend_live(&mut self, notification: &Notification) {
        for window in self.windows.values_mut() {
            if let Some(previous) = window.rows.remove(&notification.id) {
                previous.revealer.set_reveal_child(false);
                previous.revealer.unparent();
            }
            window.prepend_row(notification, self._hub.clone());
        }
        self.refresh_empty();
    }

    /// Close one row and unparent it after the collapse finishes
    fn close_row(&mut self, id: u32) {
        for window in self.windows.values_mut() {
            if let Some(row) = window.rows.remove(&id) {
                row.revealer.set_reveal_child(false);
                let revealer = row.revealer.clone();
                glib::timeout_add_local(
                    Duration::from_millis(revealer.transition_duration() as u64),
                    move || {
                        revealer.unparent();
                        glib::ControlFlow::Break
                    },
                );
            }
        }
        self.refresh_empty();
    }

    fn refresh_ages(&self) {
        for window in self.windows.values() {
            for row in window.rows.values() {
                row.widget.update_age();
            }
        }
    }

    /// Push the sink state into every volume slider
    fn sync_volume_sliders(&self) {
        self.syncing.set(true);
        let value = if self.volume.muted {
            0.0
        } else {
            (self.volume.volume * 100.0).clamp(0.0, 100.0)
        };
        for window in self.windows.values() {
            if let Some(slider) = window.volume_slider.as_ref() {
                slider.set_value(value);
            }
        }
        self.syncing.set(false);
    }

    /// Push the current backlight into every backlight slider
    fn sync_backlight_sliders(&self) {
        let Some(backlight) = self.backlight.as_ref() else {
            return;
        };
        let Ok(value) = backlight.percent() else {
            return;
        };
        self.syncing.set(true);
        let value = (value * 100.0).clamp(0.0, 100.0);
        for window in self.windows.values() {
            if let Some(slider) = window.backlight_slider.as_ref() {
                slider.set_value(value);
            }
        }
        self.syncing.set(false);
    }

    /// Side and transition follow the hosting bar's edge and the tray segment,
    /// mirroring the reference `update_side`
    fn apply_side(&mut self, settings: &zex_core::Settings) {
        let location = settings.interface.modules.location.systeminfotray;
        let bar_id = settings.interface.modules.bar_id.systeminfotray;
        let side = if bar_id == 0 {
            &settings.interface.bar.side
        } else {
            &settings.interface.bar2.side
        };
        let (halign, transition) = match side.as_str() {
            "left" => (gtk4::Align::Start, gtk4::RevealerTransitionType::SlideRight),
            "right" => (gtk4::Align::End, gtk4::RevealerTransitionType::SlideLeft),
            _ => {
                let (halign, transition) = match location {
                    0 => (gtk4::Align::Start, gtk4::RevealerTransitionType::SlideRight),
                    1 => (gtk4::Align::Center, gtk4::RevealerTransitionType::Crossfade),
                    _ => (gtk4::Align::End, gtk4::RevealerTransitionType::SlideLeft),
                };
                (halign, transition)
            }
        };
        let transition = if self.niri {
            transition
        } else {
            gtk4::RevealerTransitionType::None
        };
        for window in self.windows.values() {
            window.revealer.set_halign(halign);
            window.revealer.set_transition_type(transition);
            window.window.queue_resize();
        }
    }

    fn sync_monitors(&mut self, display: &gdk::Display) {
        let monitors = shared::monitors(display);
        let present: std::collections::HashSet<usize> = (0..monitors.len()).collect();
        self.windows.retain(|idx, _| present.contains(idx));

        for (idx, monitor) in monitors.into_iter().enumerate() {
            if self.windows.contains_key(&idx) {
                continue;
            }
            let on_close = {
                let sender = self.sender.input_sender().clone();
                let close: Rc<dyn Fn(usize)> = Rc::new(move |monitor| {
                    let _ = sender.send(QuickCenterMsg::Close { monitor });
                });
                close
            };
            let panel = self.new_window(idx, &monitor, &self.actions, on_close);
            self.windows.insert(idx, panel);
        }
    }
}

impl QuickCenterWindow {
    /// Prepend one notification row (revealed immediately)
    fn prepend_row(&mut self, notification: &Notification, hub: Arc<NotificationsHub>) {
        let Some(client) = hub.client() else {
            return;
        };
        let widget = NotificationWidget::new(notification, false, client);
        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        revealer.set_child(Some(&widget.root));
        let id = notification.id;
        self.center_box.prepend(&revealer);
        self.rows.insert(id, CenterRow { revealer, widget, id });
        if let Some(row) = self.rows.get_mut(&id) {
            row.revealer.set_reveal_child(true);
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for QuickCenter {
    type Init = (SettingsStore, Arc<NotificationsHub>, Rc<ActionHandles>);
    type Input = QuickCenterMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        (store, hub, actions): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();

        let mut model = Self {
            windows: HashMap::new(),
            volume: VolumeState::default(),
            shared_volume: Rc::new(RefCell::new(VolumeState::default())),
            syncing: Rc::new(Cell::new(false)),
            volume_control: VolumeControl::default(),
            suppress_osd: osd_suppress_flag(),
            backlight: Backlight::detect(),
            niri: compositor::detect_compositor()
                .is_some_and(|compositor| compositor.name() == "Niri"),
            history_loaded: false,
            actions,
            sender: sender.clone(),
            subscription: None,
            monitors_model: None,
            _events: hub.subscribe(),
            _hub: hub,
        };

        // Quick settings sink: clamps into PipeWire, clamped states come back as events
        let (volume_events, volume_event_rx) = flume::unbounded();
        model.volume_control = spawn_volume_monitor(
            std::sync::Arc::new(std::sync::Mutex::new(VolumeState::default())),
            tokio::sync::oneshot::channel().0,
            volume_events,
        );
        let volume_out = sender.input_sender().clone();
        std::thread::spawn(move || {
            while let Ok(state) = volume_event_rx.recv() {
                let _ = volume_out.send(QuickCenterMsg::Volume(state));
            }
        });

        let subscription = shared::subscribe_settings(&store, sender.input_sender().clone(), |s| {
            QuickCenterMsg::SettingsChanged(Box::new(s.clone()))
        });
        model.subscription = Some(subscription);

        // Events from the fan flow into the GTK loop
        let events = model._events.clone();
        let event_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                let _ = event_sender.send(QuickCenterMsg::Event(event));
            }
        });

        // The tray opens this panel through the shared handles
        let quickcenter = {
            let sender = sender.input_sender().clone();
            move || {
                let _ = sender.send(QuickCenterMsg::Toggle);
            }
        };
        actions.set_quickcenter(quickcenter);

        // Register once everything touching the model is ready: the panel toggle callback
        if let Some(display) = gdk::Display::default() {
            let on_close = {
                let sender = sender.input_sender().clone();
                let close: Rc<dyn Fn(usize)> = Rc::new(move |monitor| {
                    let _ = sender.send(QuickCenterMsg::Close { monitor });
                });
                close
            };
            let monitors_model =
                shared::watch_monitors(&display, sender.input_sender().clone(), || {
                    QuickCenterMsg::MonitorsChanged
                });
            model.monitors_model = Some(monitors_model);
            model.apply_side(store.get());
            model.sync_monitors(&display);
            model.load_history();
            model.sync_volume_sliders();
            model.sync_backlight_sliders();
        } else {
            tracing::warn!("no display available; quick center stays off");
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            QuickCenterMsg::Event(event) => match event {
                NotificationEvent::Notified(notification) => {
                    if !self.history_loaded {
                        self.load_history();
                    }
                    self.prepend_live(&notification);
                }
                NotificationEvent::Closed(id) => self.close_row(id),
                NotificationEvent::Dismissed(_) | NotificationEvent::Popup(_) => {}
                NotificationEvent::AgeTick => self.refresh_ages(),
                NotificationEvent::DndChanged(_) => {}
            },
            QuickCenterMsg::SettingsChanged(snapshot) => {
                self._hub.on_settings(&snapshot);
                self.apply_side(&snapshot);
            }
            QuickCenterMsg::MonitorsChanged => {
                if let Some(display) = gdk::Display::default() {
                    self.sync_monitors(&display);
                }
            }
            QuickCenterMsg::Toggle => {
                let monitor = gdk::Display::default()
                    .and_then(|display| Self::pointer_monitor(&display))
                    .unwrap_or(0);
                let Some(window) = self.windows.get(&monitor) else {
                    return;
                };
                let visible = window.window.is_visible();
                for (idx, window) in self.windows.iter() {
                    if *idx != monitor && window.window.is_visible() {
                        window.window.set_visible(false);
                        window.revealer.set_reveal_child(false);
                    }
                }
                window.window.set_visible(!visible);
                window.revealer.set_reveal_child(!visible);
            }
            QuickCenterMsg::Close { monitor } => {
                if let Some(window) = self.windows.get(&monitor) {
                    window.window.set_visible(false);
                    window.revealer.set_reveal_child(false);
                }
            }
            QuickCenterMsg::Volume(state) => {
                self.volume = state.clone();
                *self.shared_volume.borrow_mut() = state;
                self.sync_volume_sliders();
            }
        }
    }
}

/// Shared OSD suppression flag (created in `osd::mod`)
fn osd_suppress_flag() -> Arc<AtomicBool> {
    crate::overlays::osd::suppress_osd_flag()
}