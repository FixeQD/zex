//! Network settings widgets.

use super::TabContext;
use crate::settings::widgets::settings_row;
use gtk4::prelude::*;
use std::rc::Rc;
use zex_services::iwd;

pub(super) fn value_row(title: &str, description: &str, value: &str) -> gtk4::Box {
    let row = settings_row(Some(title), Some(description), false);
    let label = gtk4::Label::new(Some(value));
    label.add_css_class("status-label");
    label.set_selectable(true);
    label.set_xalign(1.0);
    label.set_hexpand(true);
    row.append(&label);
    row
}

pub(super) fn password_dialog(
    parent: &gtk4::Window,
    ssid: &str,
    on_submit: Rc<dyn Fn(Option<String>)>,
) {
    let dialog = gtk4::Window::new();
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.set_title(Some(&format!("Connect to {ssid}")));
    dialog.set_default_size(460, 190);

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    root.set_margin_top(20);
    root.set_margin_bottom(20);
    root.set_margin_start(20);
    root.set_margin_end(20);
    root.append(&gtk4::Label::new(Some(
        "Passphrase is kept in memory and handed directly to the wireless backend.",
    )));
    let entry = gtk4::PasswordEntry::new();
    entry.set_show_peek_icon(true);
    root.append(&entry);
    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    buttons.set_halign(gtk4::Align::End);
    let cancel = gtk4::Button::with_label("Cancel");
    let connect = gtk4::Button::with_label("Connect");
    buttons.append(&cancel);
    buttons.append(&connect);
    root.append(&buttons);
    dialog.set_child(Some(&root));

    let d = dialog.clone();
    cancel.connect_clicked(move |_| d.close());
    let d = dialog.clone();
    connect.connect_clicked(move |_| {
        on_submit(Some(entry.text().to_string()));
        d.close();
    });
    dialog.present();
}

pub(super) fn iwd_network_row(ctx: &TabContext, ap: &iwd::AccessPoint) -> gtk4::Box {
    let signal = format!("{} dBm", ap.signal_dbm);
    let flags = format!(
        "{} • {}",
        ap.security.to_uppercase(),
        if ap.known { "saved" } else { "new" }
    );
    let row = settings_row(Some(&ap.ssid), Some(&format!("{signal} • {flags}")), false);
    let button = gtk4::Button::with_label(if ap.connected {
        "Disconnect"
    } else {
        "Connect"
    });
    button.add_css_class("network-row");
    let path = ap.path.clone();
    let ssid = ap.ssid.clone();
    let secured = ap.security != "open";
    let parent = ctx.window.clone();
    button.connect_clicked(move |_| {
        let path = path.clone();
        let ssid = ssid.clone();
        let display_ssid = ssid.clone();
        let parent = parent.clone();
        if !secured && false {
            return;
        }
        let dialog_ssid = display_ssid.clone();
        let connect = Rc::new(move |password: Option<String>| {
            let path = path.clone();
            let log_ssid = display_ssid.clone();
            glib::MainContext::default().spawn_local(async move {
                let Ok(conn) = iwd::session().await else {
                    return;
                };
                let agent = iwd::Agent::default();
                if let Err(err) = iwd::register_agent(&conn, agent.clone()).await {
                    tracing::warn!("iwd agent registration failed: {err:#}");
                    return;
                }
                if let Err(err) = iwd::connect(&conn, &agent, &path, password).await {
                    tracing::warn!("iwd connection to {log_ssid} failed: {err:#}");
                }
            });
        });
        if secured {
            password_dialog(&parent, &dialog_ssid, connect);
        } else {
            connect(None);
        }
    });
    row.append(&button);
    row
}
