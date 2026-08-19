//! System info tray: SNI items, wifi/bluetooth placeholder, audio and battery.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::prelude::*;
use zex_services::audio::VolumeControl;
use zex_services::tray::{MenuEntry, TrayIcon, TrayItem};
use zex_services::upower::Battery;

use super::battery::BatteryWidget;
use super::icon::has_icon;
use super::popover::{PopoverItem, show_popover};
use crate::bar::StatusCommand;

/// Placeholder network snapshot. The NetworkManager backend lands with the settings commit
#[derive(Clone, Copy, Default, PartialEq)]
pub struct NetworkState {
    pub ethernet_connected: bool,
    pub wifi_enabled: bool,
    pub wifi_connected: bool,
    pub wifi_strength: Option<u8>,
}

/// Placeholder bluetooth snapshot; the BlueZ backend lands with the settings commit
#[derive(Clone, Copy, Default, PartialEq)]
pub struct BluetoothState {
    pub found: bool,
    pub powered: bool,
    pub connected_devices: usize,
}

/// Material-glyph wifi icon, mirroring the reference strength tiers
pub fn wifi_icon(enabled: bool, connected: bool, strength: Option<u8>) -> Option<&'static str> {
    if !enabled {
        return None;
    }
    if !connected {
        return Some("signal_wifi_off");
    }
    Some(match strength {
        None => "signal_wifi_off",
        Some(75..) => "signal_wifi_4_bar",
        Some(50..) => "network_wifi_3_bar",
        Some(25..) => "network_wifi_2_bar",
        Some(1..) => "network_wifi_1_bar",
        Some(_) => "signal_wifi_0_bar",
    })
}

/// Material-glyph bluetooth icon: powered/off/connected by device presence
pub fn bluetooth_icon(state: BluetoothState) -> Option<&'static str> {
    if !state.found {
        return None;
    }
    Some(if !state.powered {
        "bluetooth_disabled"
    } else if state.connected_devices > 0 {
        "bluetooth_connected"
    } else {
        "bluetooth"
    })
}

/// Material-glyph audio icon, mirroring the reference tiers and mute
pub fn audio_icon(volume: f32, muted: bool) -> &'static str {
    if muted {
        "volume_off"
    } else if volume < 0.33 {
        "volume_mute"
    } else if volume < 0.67 {
        "volume_down"
    } else {
        "volume_up"
    }
}

/// Pointer position on the default display, for SNI activate coordinates
fn pointer_pos() -> (i32, i32) {
    let Some(display) = gdk::Display::default() else {
        return (0, 0);
    };
    let Some(seat) = display.default_seat() else {
        return (0, 0);
    };
    let Some(pointer) = seat.pointer() else {
        return (0, 0);
    };
    let (_surface, x, y) = pointer.surface_at_position();
    (x as i32, y as i32)
}

/// Routes tray commands to the tray runtime thread (never the GTK thread)
#[derive(Clone)]
pub struct TrayControl {
    tx: flume::Sender<StatusCommand>,
}

impl TrayControl {
    pub fn new(tx: flume::Sender<StatusCommand>) -> Self {
        Self { tx }
    }

    /// Ask the item to activate (left click)
    pub fn activate(&self, service: &str) {
        let (x, y) = pointer_pos();
        let _ = self.tx.send(StatusCommand::TrayActivate {
            service: service.to_owned(),
            x,
            y,
        });
    }

    /// Fetch the dbusmenu layout; entries come back on the reply channel
    pub fn menu(&self, service: &str, reply: flume::Sender<Vec<MenuEntry>>) {
        let _ = self.tx.send(StatusCommand::TrayMenu {
            service: service.to_owned(),
            reply,
        });
    }

    pub fn menu_action(&self, service: &str, id: i32) {
        let _ = self.tx.send(StatusCommand::TrayMenuAction {
            service: service.to_owned(),
            id,
        });
    }
}

/// RGBA texture from the SNI ARGB32 pixmap closest to the target size
fn pixmap_texture(icon: &TrayIcon, target: i32) -> Option<gdk::Texture> {
    let pixmap = icon
        .pixmap
        .iter()
        .min_by_key(|p| (p.width - target).abs())?;
    let width = pixmap.width as usize;
    let height = pixmap.height as usize;
    if width == 0 || height == 0 || pixmap.data.len() < width * height * 4 {
        return None;
    }
    let mut rgba = pixmap.data[..width * height * 4].to_vec();
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 3); // SNI ships ARGB32, textures want RGBA
    }
    crate::shared::texture_from_rgba(width as u32, height as u32, &rgba).ok()
}

fn apply_glyph(label: &gtk4::Label, glyph: Option<&'static str>) {
    match glyph {
        Some(glyph) => {
            label.set_label(glyph);
            label.set_visible(true);
        }
        None => label.set_visible(false),
    }
}

struct State {
    vertical: bool,
    network: NetworkState,
    bluetooth: BluetoothState,
    volume: f32,
    muted: bool,
}

pub struct SystemInfoTray {
    container: gtk4::Box,
    tray_box: gtk4::Box,
    wifi: gtk4::Label,
    bluetooth: gtk4::Label,
    audio: gtk4::Label,
    battery: Rc<BatteryWidget>,
    volume: VolumeControl,
    tray: TrayControl,
    state: RefCell<State>,
}

impl SystemInfoTray {
    /// The whole tray toggles the QuickCenter through the injected handler
    pub fn new(
        vertical: bool,
        on_quickcenter: impl Fn() + 'static,
        tray: TrayControl,
        volume: VolumeControl,
        battery: Rc<BatteryWidget>,
    ) -> Rc<Self> {
        let wifi = gtk4::Label::new(None);
        wifi.set_css_classes(&["icon"]);

        let bluetooth = gtk4::Label::new(None);
        bluetooth.set_css_classes(&["icon"]);

        let audio = gtk4::Label::new(None);
        audio.set_css_classes(&["icon"]);

        let state = State {
            vertical,
            network: NetworkState {
                ethernet_connected: false,
                wifi_enabled: true,
                wifi_connected: true,
                wifi_strength: Some(80),
            },
            bluetooth: BluetoothState {
                found: false,
                powered: false,
                connected_devices: 0,
            },
            volume: 1.0,
            muted: false,
        };

        let widget = Rc::new(Self {
            container: gtk4::Box::new(gtk4::Orientation::Horizontal, 10),
            tray_box: gtk4::Box::new(gtk4::Orientation::Horizontal, 10),
            wifi,
            bluetooth,
            audio,
            battery,
            volume,
            tray,
            state: RefCell::new(state),
        });
        widget.container.set_css_classes(&["system-info-tray"]);
        widget.tray_box.set_css_classes(&["tray"]);
        widget.container.append(&widget.tray_box);
        widget.apply_layout();

        let audio_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        audio_container.set_halign(gtk4::Align::Center);
        audio_container.set_valign(gtk4::Align::Center);
        audio_container.append(&widget.audio);

        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        content.append(&widget.wifi);
        content.append(&widget.bluetooth);
        content.append(&audio_container);
        content.append(&widget.battery.widget());

        let button = gtk4::Button::new();
        button.set_css_classes(&["system-info-tray-container"]);
        button.set_child(Some(&content));
        button.connect_clicked(move |_| on_quickcenter());
        widget.container.append(&button);

        // audio scroll: ±5 percent steps, clamped to the reference 0-100 range
        let this = Rc::clone(&widget);
        let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(move |_controller, _dx, dy| {
            let step = if dy > 0.0 { -5.0 } else { 5.0 };
            let mut state = this.state.borrow_mut();
            let volume = ((state.volume * 100.0) + step).clamp(0.0, 100.0) / 100.0;
            state.volume = volume;
            this.volume.set_volume(volume);
            drop(state);
            this.refresh_icons();
            glib::Propagation::Proceed
        });
        audio_container.add_controller(scroll);

        // middle and right click toggle mute
        let this = Rc::clone(&widget);
        let click = gtk4::GestureClick::new();
        click.connect_pressed(move |gesture, _n, _x, _y| {
            let button = gesture.current_button();
            if button == gtk4::gdk::BUTTON_SECONDARY || button == gtk4::gdk::BUTTON_MIDDLE {
                let mut state = this.state.borrow_mut();
                state.muted = !state.muted;
                this.volume.toggle_mute();
                drop(state);
                this.refresh_icons();
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            }
        });
        audio_container.add_controller(click);

        widget.refresh_icons();
        widget
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    pub fn on_tray(&self, items: &[TrayItem]) {
        while let Some(child) = self.tray_box.first_child() {
            self.tray_box.remove(&child);
        }
        for item in items {
            self.tray_box
                .append(&make_item_button(item.clone(), self.tray.clone()));
        }
        self.tray_box.set_visible(!items.is_empty());
    }

    pub fn on_volume(&self, volume: f32, muted: bool) {
        let mut state = self.state.borrow_mut();
        state.volume = volume;
        state.muted = muted;
        drop(state);
        self.refresh_icons();
    }

    pub fn on_batteries(&self, batteries: &[Battery]) {
        let vertical = self.state.borrow().vertical;
        self.battery.update(batteries, vertical);
    }

    fn refresh_icons(&self) {
        let state = self.state.borrow();
        let network = state.network;
        let bluetooth = state.bluetooth;
        let volume = state.volume;
        let muted = state.muted;

        let network_glyph = if network.ethernet_connected {
            Some("settings_ethernet")
        } else {
            wifi_icon(
                network.wifi_enabled,
                network.wifi_connected,
                network.wifi_strength,
            )
        };
        apply_glyph(&self.wifi, network_glyph);
        apply_glyph(&self.bluetooth, bluetooth_icon(bluetooth));
        self.audio.set_label(audio_icon(volume, muted));
    }

    /// Orientation and spacing follow the bar side, mirroring the reference
    pub fn apply_layout(&self) {
        let vertical = self.state.borrow().vertical;
        self.container.set_orientation(if vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        });
        self.container.set_spacing(if vertical { 5 } else { 10 });
        self.tray_box.set_orientation(if vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        });
        self.tray_box.set_spacing(if vertical { 0 } else { 10 });
    }
}

/// One SNI item button: themed icon or pixmap texture; click activates, PPM opens the menu
fn make_item_button(item: TrayItem, tray: TrayControl) -> gtk4::Button {
    let button = gtk4::Button::new();
    button.set_css_classes(&["tray-item"]);

    let image = gtk4::Image::new();
    image.set_pixel_size(16);
    image.set_css_classes(&["tray-icon"]);

    let themed = item
        .icon
        .name
        .as_deref()
        .filter(|name| !name.is_empty() && has_icon(name));
    if let Some(name) = themed {
        image.set_icon_name(Some(name));
    } else if let Some(texture) = pixmap_texture(&item.icon, 16) {
        image.set_paintable(Some(&texture));
    }
    button.set_child(Some(&image));

    let service = item.service.clone();
    let tray = Rc::new(tray);
    button.connect_clicked({
        let tray = Rc::clone(&tray);
        let service = service.clone();
        move |_| tray.activate(&service)
    });

    let gesture = gtk4::GestureClick::new();
    gesture.set_button(gtk4::gdk::BUTTON_SECONDARY);
    let anchor = button.clone();
    let service = item.service.clone();
    gesture.connect_pressed(move |_gesture, _n, _x, _y| {
        tray_menu(&anchor, &service, Rc::clone(&tray));
    });
    button.add_controller(gesture);

    button
}

/// Fetch the dbusmenu layout through the status thread, then pop up next to the button.
/// The wait is one dbus round-trip; a full async popover could land in a later commit.
fn tray_menu(anchor: &gtk4::Button, service: &str, tray: Rc<TrayControl>) {
    let (reply, rx) = flume::unbounded();
    tray.menu(service, reply);
    let Ok(entries) = rx.recv_timeout(std::time::Duration::from_secs(3)) else {
        tracing::warn!("tray menu timed out for {service}");
        return;
    };
    if entries.is_empty() {
        return;
    }
    let items = menu_items(&entries, service, &tray);
    show_popover(anchor.upcast_ref(), items);
}

/// Flatten the dbusmenu layout: nested groups land under a separator
fn menu_items(entries: &[MenuEntry], service: &str, tray: &TrayControl) -> Vec<PopoverItem> {
    let mut items = Vec::new();
    for entry in entries {
        if entry.visible {
            if !entry.enabled {
                items.push(PopoverItem::Label(entry.label.clone()));
            } else {
                let label = entry.label.clone();
                let service = service.to_owned();
                let tray = tray.clone();
                let id = entry.id;
                items.push(PopoverItem::Action(
                    label,
                    Rc::new(move || {
                        tray.menu_action(&service, id);
                    }),
                ));
            }
        }
        if !entry.children.is_empty() {
            let mut children = menu_items(&entry.children, service, tray);
            if !children.is_empty() {
                items.push(PopoverItem::Separator);
                items.append(&mut children);
            }
        }
    }
    items
}
