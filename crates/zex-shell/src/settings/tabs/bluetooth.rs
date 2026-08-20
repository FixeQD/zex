//! Bluetooth settings backed directly by BlueZ over D-Bus.

use gtk4::prelude::*;
use zex_services::bluetooth::{self, BluetoothDevice};

use super::TabContext;
use crate::settings::widgets::{category, separator, settings_row, switch_row};

fn body() -> gtk4::Box {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body.add_css_class("settings-body");
    body.set_width_request(800);
    body.set_halign(gtk4::Align::Center);
    body
}

fn device_row(device: &BluetoothDevice, conn: &zbus::Connection) -> gtk4::Box {
    let row = settings_row(
        Some(&device.alias),
        Some(&format!(
            "{} • {}",
            device.address,
            if device.connected {
                "Connected"
            } else if device.paired {
                "Paired"
            } else {
                "Not paired"
            }
        )),
        false,
    );
    let button = gtk4::Button::with_label(if device.connected {
        "Disconnect"
    } else {
        "Connect"
    });
    let path = device.path.clone();
    let connected = device.connected;
    let conn = conn.clone();
    button.connect_clicked(move |_| {
        let conn = conn.clone();
        let path = path.clone();
        glib::MainContext::default().spawn_local(async move {
            let result = if connected {
                bluetooth::disconnect(&conn, &path).await
            } else {
                bluetooth::connect(&conn, &path).await
            };
            if let Err(err) = result {
                tracing::warn!("Bluetooth operation failed: {err:#}");
            }
        });
    });
    row.append(&button);
    row
}

pub fn build(ctx: &TabContext) -> gtk4::Box {
    let root = body();
    let adapter = category("Bluetooth Adapter");
    let state = gtk4::Label::new(Some("Loading Bluetooth state…"));
    state.add_css_class("status-label");
    adapter.append(&state);

    let powered = switch_row(
        ctx,
        Some("Bluetooth"),
        Some("Toggle the Bluetooth adapter."),
        false,
        |_s, value| {
            glib::MainContext::default().spawn_local(async move {
                if let Ok(conn) = bluetooth::session().await {
                    if let Err(err) = bluetooth::set_powered(&conn, value).await {
                        tracing::warn!("Bluetooth toggle failed: {err:#}");
                    }
                }
            });
        },
    );
    adapter.append(&separator());
    adapter.append(&powered);
    root.append(&adapter);

    let devices = category("Devices");
    let list = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
    list.add_css_class("network-list-container");
    devices.append(&settings_row(
        Some("Paired & Discovered Devices"),
        Some("Select a device to connect or disconnect."),
        false,
    ));
    devices.append(&list);
    let scan = gtk4::Button::with_label("Scan for Devices");
    let list_clone = list.clone();
    let state_clone = state.clone();
    let ctx_data = (
        std::rc::Rc::clone(&ctx.store),
        ctx.sender.clone(),
        ctx.previews.clone(),
        ctx.gallery.clone(),
        ctx.window.clone(),
    );
    scan.connect_clicked(move |button| {
        button.set_sensitive(false);
        button.set_label("Scanning…");
        let list = list_clone.clone();
        let state = state_clone.clone();
        let button = button.clone();
        let _ = &ctx_data;
        glib::MainContext::default().spawn_local(async move {
            match bluetooth::session().await {
                Ok(conn) => {
                    let _ = bluetooth::start_discovery(&conn).await;
                    glib::timeout_future_seconds(15).await;
                    let _ = bluetooth::stop_discovery(&conn).await;
                    match bluetooth::snapshot(&conn).await {
                        Ok(snapshot) => {
                            state.set_text(if snapshot.available {
                                if snapshot.powered {
                                    "Bluetooth is on"
                                } else {
                                    "Bluetooth is off"
                                }
                            } else {
                                "No Bluetooth adapter found"
                            });
                            while let Some(child) = list.first_child() {
                                list.remove(&child);
                            }
                            for device in &snapshot.devices {
                                list.append(&device_row(device, &conn));
                            }
                        }
                        Err(err) => state.set_text(&format!("Bluetooth error: {err}")),
                    }
                }
                Err(err) => state.set_text(&format!("Bluetooth unavailable: {err}")),
            }
            button.set_sensitive(true);
            button.set_label("Scan for Devices");
        });
    });
    devices.append(&scan);
    root.append(&devices);
    root
}
