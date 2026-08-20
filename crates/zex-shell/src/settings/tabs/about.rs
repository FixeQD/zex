//! About tab: software, appearance and hardware read-outs.

use std::fs;

use gtk4::prelude::*;

use super::TabContext;
use crate::settings::widgets::{category, separator, settings_row};

fn os_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(str::trim)
                    .map(|value| value.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

fn desktop_name() -> String {
    std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|name| name.trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn gtk_theme_name() -> String {
    gtk4::Settings::default()
        .map(|settings| settings.gtk_theme_name().unwrap_or_default().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn icon_theme_name() -> String {
    gtk4::Settings::default()
        .map(|settings| {
            settings
                .gtk_icon_theme_name()
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

fn cpu_name() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.trim()
                    .strip_prefix("model name")
                    .map(|rest| rest.trim_start_matches(':').trim())
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
            })
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

fn ram_gib() -> String {
    let total_kib = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|value| value.parse::<f64>().ok())
            })
        })
        .unwrap_or(0.0);
    format!("{:.2} GiB", total_kib / 1024.0 / 1024.0)
}

fn software_category() -> gtk4::Box {
    let container = category("Software");
    container.append(&settings_row(
        Some("Operating System"),
        Some(&os_name()),
        false,
    ));
    container.append(&separator());
    container.append(&settings_row(Some("Desktop"), Some(&desktop_name()), false));
    container.append(&separator());
    container.append(&settings_row(Some("Hostname"), Some(&hostname()), false));
    container
}

fn appearance_category() -> gtk4::Box {
    let container = category("Appearance");
    container.append(&settings_row(
        Some("GTK Theme"),
        Some(&gtk_theme_name()),
        false,
    ));
    container.append(&separator());
    container.append(&settings_row(
        Some("Icon Theme"),
        Some(&icon_theme_name()),
        false,
    ));
    container
}

fn hardware_category() -> gtk4::Box {
    let container = category("Hardware");
    container.append(&settings_row(Some("CPU"), Some(&cpu_name()), false));
    container.append(&separator());
    container.append(&settings_row(Some("RAM"), Some(&ram_gib()), false));
    container
}

/// Build the whole about tab.
pub fn build(ctx: &TabContext) -> gtk4::Box {
    let _ = ctx;
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body.add_css_class("settings-body");
    body.set_halign(gtk4::Align::Center);
    body.set_hexpand(false);
    body.set_width_request(800);
    body.append(&software_category());
    body.append(&appearance_category());
    body.append(&hardware_category());
    body
}
