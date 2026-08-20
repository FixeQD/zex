//! Services tab OSD category

use gtk4::prelude::*;
use zex_core::Settings;
use zex_core::settings::Anchor;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    ToggleItem, category, separator, settings_row, switch_row, toggle_buttons,
};

pub fn osd_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("OSD");

    let snapshot = ctx.snapshot();
    let anchor = snapshot.services.osd.anchor.clone();
    let vertical = snapshot.services.osd.vertical;

    let anchor_row = settings_row(
        Some("Popup Location"),
        Some("Pick a location for your OSD popups."),
        false,
    );
    let osd_anchor = toggle_buttons(
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
                icon: Some("go-previous-symbolic"),
                value: vec![Anchor::Left],
            },
            ToggleItem {
                label: None,
                icon: Some("go-next-symbolic"),
                value: vec![Anchor::Right],
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
        move |_s: &Settings| anchor.clone(),
        |s: &mut Settings, value| s.services.osd.anchor = value,
    );
    anchor_row.append(&osd_anchor.container);
    container.append(&anchor_row);
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Vertical"),
        Some("Use a vertical OSD. Only takes effect in corners."),
        vertical,
        |s: &mut Settings, active| s.services.osd.vertical = active,
    ));

    container
}
