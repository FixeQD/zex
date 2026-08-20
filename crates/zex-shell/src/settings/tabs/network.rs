//! Power-user network settings.

use gtk4::prelude::*;
use std::rc::Rc;
use zex_services::{iwd, networkmanager};

use super::TabContext;
use crate::settings::widgets::{category, separator, settings_row};

#[path = "network_ui.rs"]
mod network_ui;
use network_ui::{iwd_network_row, value_row};

fn body() -> gtk4::Box {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    body.add_css_class("settings-body");
    body.set_width_request(850);
    body.set_halign(gtk4::Align::Center);
    body
}

pub fn build(ctx: &TabContext) -> gtk4::Box {
    let root = body();

    let backend = category("Wireless backend");
    backend.append(&value_row(
        "Priority",
        "Zex tries iwd first and only uses NetworkManager as the explicit fallback.",
        "1. iwd  →  2. NetworkManager",
    ));
    backend.append(&value_row(
        "Why",
        "iwd exposes the wireless stack directly and supports native D-Bus control.",
        "native D-Bus",
    ));
    root.append(&backend);

    let wifi = category("iwd • Wi-Fi");
    let state = gtk4::Label::new(Some("Detecting iwd…"));
    state.add_css_class("status-label");
    wifi.append(&settings_row(
        Some("Station"),
        Some("State, interface and scan status."),
        false,
    ));
    wifi.append(&state);
    wifi.append(&separator());

    let power = gtk4::Switch::new();
    power.set_valign(gtk4::Align::Center);
    power.connect_state_set(|_, enabled| {
        glib::MainContext::default().spawn_local(async move {
            if let Ok(conn) = iwd::session().await {
                if let Err(err) = iwd::set_power(&conn, enabled).await {
                    tracing::warn!("iwd power toggle failed: {err:#}");
                }
            }
        });
        glib::Propagation::Proceed
    });
    let power_row = settings_row(
        Some("Radio power"),
        Some("Directly controls the iwd adapter Powered property."),
        false,
    );
    power_row.append(&power);
    wifi.append(&power_row);

    let networks = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
    networks.add_css_class("network-list-container");
    wifi.append(&settings_row(
        Some("Networks"),
        Some("Sorted by iwd's own network selection score. Signal is shown as dBm."),
        false,
    ));
    wifi.append(&networks);

    let controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let scan = gtk4::Button::with_label("Scan");
    let refresh = gtk4::Button::with_label("Refresh");
    let disconnect = gtk4::Button::with_label("Disconnect");
    controls.append(&scan);
    controls.append(&refresh);
    controls.append(&disconnect);
    wifi.append(&controls);

    let info = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    wifi.append(&info);

    let networks_scan = networks.clone();
    let state_scan = state.clone();
    scan.connect_clicked(move |_| {
        let networks = networks_scan.clone();
        let state = state_scan.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Ok(conn) = iwd::session().await {
                if let Err(err) = iwd::scan(&conn).await {
                    state.set_text(&format!("scan failed: {err}"));
                } else {
                    state.set_text("scan scheduled • press Refresh for results");
                }
                let _ = networks;
            }
        });
    });

    let ctx_reload = (
        Rc::clone(&ctx.store),
        ctx.sender.clone(),
        ctx.previews.clone(),
        ctx.gallery.clone(),
        ctx.window.clone(),
    );
    let networks_refresh = networks.clone();
    let state_refresh = state.clone();
    let power_refresh = power.clone();
    let info_refresh = info.clone();
    refresh.connect_clicked(move |_| {
        let networks = networks_refresh.clone();
        let state = state_refresh.clone();
        let power = power_refresh.clone();
        let info = info_refresh.clone();
        let ctx_data = ctx_reload.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Ok(conn) = iwd::session().await {
                if let Ok(snapshot) = iwd::snapshot(&conn).await {
                    power.set_active(snapshot.adapter_powered);
                    state.set_text(&format!(
                        "{} • {} • {}",
                        snapshot.state,
                        snapshot.interface,
                        if snapshot.scanning {
                            "scanning"
                        } else {
                            "idle"
                        }
                    ));
                    while let Some(child) = networks.first_child() {
                        networks.remove(&child);
                    }
                    for ap in &snapshot.networks {
                        let tab = TabContext {
                            store: Rc::clone(&ctx_data.0),
                            sender: ctx_data.1.clone(),
                            previews: ctx_data.2.clone(),
                            gallery: ctx_data.3.clone(),
                            window: ctx_data.4.clone(),
                        };
                        networks.append(&iwd_network_row(&tab, ap));
                    }
                    while let Some(child) = info.first_child() {
                        info.remove(&child);
                    }
                    info.append(&value_row("Adapter", "iwd adapter.", &snapshot.adapter));
                    info.append(&value_row(
                        "Interface",
                        "Wireless interface.",
                        &snapshot.interface,
                    ));
                    info.append(&value_row("State", "Station state.", &snapshot.state));
                    info.append(&value_row(
                        "Scan",
                        "Current scan state.",
                        if snapshot.scanning { "active" } else { "idle" },
                    ));
                    info.append(&value_row(
                        "Visible networks",
                        "Latest iwd scan result count.",
                        &snapshot.networks.len().to_string(),
                    ));
                }
            } else {
                state.set_text("iwd unavailable");
            }
        });
    });
    disconnect.connect_clicked(move |_| {
        glib::MainContext::default().spawn_local(async move {
            if let Ok(conn) = iwd::session().await {
                let _ = iwd::disconnect(&conn).await;
            }
        });
    });
    root.append(&wifi);

    let nm = category("NetworkManager • fallback");
    nm.append(&value_row(
        "Priority",
        "Used only as the second wireless backend. Ethernet status is also available here.",
        "2",
    ));
    nm.append(&value_row(
        "Control path",
        "Native NetworkManager D-Bus API with SecretAgent support.",
        "org.freedesktop.NetworkManager",
    ));
    let nm_refresh = gtk4::Button::with_label("Probe NetworkManager");
    nm_refresh.connect_clicked(move |_| {
        glib::MainContext::default().spawn_local(async move {
            if let Ok(conn) = networkmanager::session().await {
                match networkmanager::snapshot(&conn).await {
                    Ok(snapshot) => tracing::info!(
                        "NetworkManager: wifi={} aps={} ethernet={}",
                        snapshot.wifi_enabled,
                        snapshot.access_points.len(),
                        snapshot.ethernet.connected
                    ),
                    Err(err) => tracing::warn!("NetworkManager probe failed: {err:#}"),
                }
            }
        });
    });
    nm.append(&nm_refresh);
    root.append(&nm);

    let advanced = category("Diagnostics");
    advanced.append(&value_row(
        "Design",
        "No shelling out to iwctl, nmcli, rfkill or external control panels.",
        "D-Bus only",
    ));
    advanced.append(&value_row("Power-user data", "Backend, adapter, station state, scan state, security type, known state and raw dBm signal are exposed.", "enabled"));
    root.append(&advanced);

    let networks_init = networks.clone();
    let state_init = state.clone();
    let power_init = power.clone();
    let info_init = info.clone();
    glib::MainContext::default().spawn_local(async move {
        if let Ok(conn) = iwd::session().await {
            if let Ok(snapshot) = iwd::snapshot(&conn).await {
                power_init.set_active(snapshot.adapter_powered);
                state_init.set_text(&format!(
                    "{} • {} • {}",
                    snapshot.state,
                    snapshot.interface,
                    if snapshot.scanning {
                        "scanning"
                    } else {
                        "idle"
                    }
                ));
                while let Some(child) = networks_init.first_child() {
                    networks_init.remove(&child);
                }
                for ap in snapshot.networks {
                    networks_init.append(&settings_row(
                        Some(&ap.ssid),
                        Some(&format!("{} dBm • {}", ap.signal_dbm, ap.security)),
                        false,
                    ));
                }
                info_init.append(&value_row("Adapter", "iwd adapter.", &snapshot.adapter));
                info_init.append(&value_row(
                    "Interface",
                    "Wireless interface.",
                    &snapshot.interface,
                ));
            }
        } else {
            state_init.set_text("iwd unavailable • NetworkManager remains available below");
        }
    });

    root
}
