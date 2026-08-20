//! Interface tab bar modules: per-module bar id, location and visibility

use std::rc::Rc;

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    ToggleItem, category, separator, settings_row, toggle_buttons, vertical_separator,
};

const MODULES: [(&str, &str, &str); 8] = [
    ("launcher", "Launcher", "Button to open the Launcher."),
    (
        "window_info",
        "Window Info",
        "Shows information about the active window.",
    ),
    (
        "media",
        "Media",
        "Shows the currently playing media with controls.",
    ),
    ("workspaces", "Workspaces", "Shows a list of workspaces."),
    (
        "tasks",
        "Tasks",
        "Shows pinned and currently running applications.",
    ),
    (
        "recording_indicator",
        "Recording Indicator",
        "Shows the current recording status.",
    ),
    ("systeminfotray", "System Tray", "Shows system tray icons."),
    ("clock", "Clock", "Shows the current time and date."),
];

fn bar_id_of(s: &Settings, key: &str) -> u8 {
    match key {
        "launcher" => s.interface.modules.bar_id.launcher,
        "window_info" => s.interface.modules.bar_id.window_info,
        "media" => s.interface.modules.bar_id.media,
        "workspaces" => s.interface.modules.bar_id.workspaces,
        "tasks" => s.interface.modules.bar_id.tasks,
        "recording_indicator" => s.interface.modules.bar_id.recording_indicator,
        "systeminfotray" => s.interface.modules.bar_id.systeminfotray,
        "clock" => s.interface.modules.bar_id.clock,
        other => {
            tracing::warn!("unknown module {other}");
            0
        }
    }
}

fn set_bar_id(s: &mut Settings, key: &str, value: u8) {
    match key {
        "launcher" => s.interface.modules.bar_id.launcher = value,
        "window_info" => s.interface.modules.bar_id.window_info = value,
        "media" => s.interface.modules.bar_id.media = value,
        "workspaces" => s.interface.modules.bar_id.workspaces = value,
        "tasks" => s.interface.modules.bar_id.tasks = value,
        "recording_indicator" => s.interface.modules.bar_id.recording_indicator = value,
        "systeminfotray" => s.interface.modules.bar_id.systeminfotray = value,
        "clock" => s.interface.modules.bar_id.clock = value,
        other => tracing::warn!("unknown module {other}"),
    }
}

fn location_of(s: &Settings, key: &str) -> u8 {
    let l = &s.interface.modules.location;
    match key {
        "launcher" => l.launcher,
        "window_info" => l.window_info,
        "media" => l.media,
        "workspaces" => l.workspaces,
        "tasks" => l.tasks,
        "recording_indicator" => l.recording_indicator,
        "systeminfotray" => l.systeminfotray,
        "clock" => l.clock,
        other => {
            tracing::warn!("unknown module {other}");
            0
        }
    }
}

fn set_location(s: &mut Settings, key: &str, value: u8) {
    let l = &mut s.interface.modules.location;
    match key {
        "launcher" => l.launcher = value,
        "window_info" => l.window_info = value,
        "media" => l.media = value,
        "workspaces" => l.workspaces = value,
        "tasks" => l.tasks = value,
        "recording_indicator" => l.recording_indicator = value,
        "systeminfotray" => l.systeminfotray = value,
        "clock" => l.clock = value,
        other => tracing::warn!("unknown module {other}"),
    }
}

fn visible_of(s: &Settings, key: &str) -> bool {
    let v = &s.interface.modules.visibility;
    match key {
        "launcher" => v.launcher,
        "window_info" => v.window_info,
        "media" => v.media,
        "workspaces" => v.workspaces,
        "tasks" => v.tasks,
        "recording_indicator" => v.recording_indicator,
        "systeminfotray" => v.systeminfotray,
        "clock" => v.clock,
        other => {
            tracing::warn!("unknown module {other}");
            false
        }
    }
}

fn set_visible(s: &mut Settings, key: &str, active: bool) {
    let v = &mut s.interface.modules.visibility;
    match key {
        "launcher" => v.launcher = active,
        "window_info" => v.window_info = active,
        "media" => v.media = active,
        "workspaces" => v.workspaces = active,
        "tasks" => v.tasks = active,
        "recording_indicator" => v.recording_indicator = active,
        "systeminfotray" => v.systeminfotray = active,
        "clock" => v.clock = active,
        other => tracing::warn!("unknown module {other}"),
    }
}

pub fn module_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Bar Modules");
    for (index, (key, name, description)) in MODULES.iter().enumerate() {
        if index > 0 {
            container.append(&separator());
        }
        container.append(&module_row(ctx, key, name, description));
    }
    container
}

fn module_row(ctx: &TabContext, key: &str, name: &str, description: &str) -> gtk4::Box {
    let row = settings_row(Some(&format!("{name} Widget")), Some(description), false);
    row.add_css_class("module-options");

    let key = key.to_string();
    let bar_id_key = key.clone();
    let bar_id_key_set = bar_id_key.clone();
    let bar_id = toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: Some("Bar 1"),
                icon: None,
                value: 0u8,
            },
            ToggleItem {
                label: Some("Bar 2"),
                icon: None,
                value: 1u8,
            },
        ],
        move |s: &Settings| bar_id_of(s, &bar_id_key),
        move |s: &mut Settings, value| set_bar_id(s, &bar_id_key_set, value),
    );
    row.append(&bar_id.container);
    row.append(&vertical_separator());

    let location_key = key.clone();
    let location_key_set = location_key.clone();
    let location = toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: Some("Start"),
                icon: None,
                value: 0u8,
            },
            ToggleItem {
                label: Some("Center"),
                icon: None,
                value: 1u8,
            },
            ToggleItem {
                label: Some("End"),
                icon: None,
                value: 2u8,
            },
        ],
        move |s: &Settings| location_of(s, &location_key),
        move |s: &mut Settings, value| set_location(s, &location_key_set, value),
    );
    row.append(&location.container);
    row.append(&vertical_separator());

    let visible_key = key.clone();
    let visible_key_set = visible_key.clone();
    let active = visible_of(&ctx.snapshot(), &visible_key);
    let switch = gtk4::Switch::new();
    switch.set_active(active);
    switch.set_valign(gtk4::Align::Center);
    let store = Rc::clone(&ctx.store);
    switch.connect_state_set(move |switch, active| {
        match store
            .borrow_mut()
            .update(|s| set_visible(s, &visible_key_set, active))
        {
            Ok(()) => false.into(),
            Err(err) => {
                tracing::warn!("settings persistence failed: {err:#}");
                switch.set_active(!active);
                true.into()
            }
        }
    });
    row.append(&switch);

    row
}
