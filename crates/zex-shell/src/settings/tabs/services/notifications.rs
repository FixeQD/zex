//! Services tab notifications category

use std::collections::HashMap;

use gtk4::prelude::*;
use zex_core::Settings;
use zex_core::settings::Anchor;

use crate::m3::{ConnectedButtonGroup, M3Button, M3Shape, M3Size, M3Type};
use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    ToggleItem, category, separator, settings_row, spin_row, switch_row, toggle_buttons,
};

pub fn notifications_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Notifications");

    let snapshot = ctx.snapshot();
    let service = &snapshot.services.notifications;
    let anchor = snapshot.interface.notifications.anchor.clone();

    container.append(&switch_row(
        ctx,
        Some("Do Not Disturb"),
        Some("When enabled, stops popups for new notifications."),
        service.dnd,
        |s: &mut Settings, active| s.services.notifications.dnd = active,
    ));
    container.append(&separator());

    let timeout_secs = service.timeout_ms.div_euclid(1000) as i32;
    container.append(&spin_row(
        ctx,
        Some("Popup Timeout"),
        Some("How long (in seconds) a notification popup stays on screen."),
        1,
        60,
        timeout_secs,
        |s: &mut Settings, value| s.services.notifications.timeout_ms = i64::from(value) * 1000,
    ));
    container.append(&separator());

    let max_popups = service.max_popups.clamp(1, 20) as i32;
    container.append(&spin_row(
        ctx,
        Some("Max Popups"),
        Some("How many popup notifications can be shown at once."),
        1,
        20,
        max_popups,
        |s: &mut Settings, value| s.services.notifications.max_popups = value as usize,
    ));
    container.append(&separator());

    let anchor_row = settings_row(
        Some("Popup Location"),
        Some("Pick a location for your notification popups."),
        false,
    );
    anchor_row.append(&anchor_toggle(ctx, anchor).container);
    container.append(&anchor_row);
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Compact Pop-up"),
        Some("Show a more compact pop-up for incoming notifications."),
        snapshot.interface.notifications.compact_popup,
        |s: &mut Settings, active| s.interface.notifications.compact_popup = active,
    ));
    container.append(&separator());

    let test = M3Button::new(
        Some("dialog-information-symbolic"),
        Some("Test Notification"),
        M3Type::Tonal,
        M3Size::Xs,
        M3Shape::Round,
    );
    test.button.set_halign(gtk4::Align::Start);
    test.button.connect_clicked(|_| send_test_notification());
    let test_row = settings_row(Some("Send a Test Notification"), None, false);
    test_row.append(&test.button);
    container.append(&test_row);

    container
}

fn anchor_toggle(ctx: &TabContext, current: Vec<Anchor>) -> ConnectedButtonGroup {
    toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: None,
                icon: Some("go-first-symbolic"),
                value: vec![Anchor::Top, Anchor::Left],
            },
            ToggleItem {
                label: Some("Top"),
                icon: Some("go-up-symbolic"),
                value: vec![Anchor::Top],
            },
            ToggleItem {
                label: None,
                icon: Some("go-last-symbolic"),
                value: vec![Anchor::Top, Anchor::Right],
            },
            ToggleItem {
                label: None,
                icon: Some("go-first-symbolic"),
                value: vec![Anchor::Bottom, Anchor::Left],
            },
            ToggleItem {
                label: Some("Bottom"),
                icon: Some("go-down-symbolic"),
                value: vec![Anchor::Bottom],
            },
            ToggleItem {
                label: None,
                icon: Some("go-last-symbolic"),
                value: vec![Anchor::Bottom, Anchor::Right],
            },
        ],
        move |_s: &Settings| current.clone(),
        |s: &mut Settings, value| s.interface.notifications.anchor = value,
    )
}

/// Arguments for the org.freedesktop.Notifications `Notify` D-Bus method
type NotifyArgs = (
    &'static str,
    u32,
    &'static str,
    &'static str,
    &'static str,
    Vec<String>,
    HashMap<String, zbus::zvariant::Value<'static>>,
    i32,
);

fn send_test_notification() {
    std::thread::spawn(|| {
        let conn = match zbus::blocking::Connection::session() {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!("no session bus for test notification: {err}");
                return;
            }
        };
        let destination = "org.freedesktop.Notifications";
        let path = "/org/freedesktop/Notifications";
        let proxy = match zbus::blocking::Proxy::new(&conn, destination, path, destination) {
            Ok(proxy) => proxy,
            Err(err) => {
                tracing::warn!("notification proxy unavailable: {err}");
                return;
            }
        };
        let args: NotifyArgs = (
            "zex",
            0,
            "",
            "Zex",
            "This is a test notification!",
            Vec::new(),
            HashMap::new(),
            5000,
        );
        if let Err(err) = proxy.call_method("Notify", &args) {
            tracing::warn!("test notification failed: {err}");
        }
    });
}
