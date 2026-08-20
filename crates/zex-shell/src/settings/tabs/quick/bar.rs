//! Quick tab bar category: position, density and modifiers for the primary bar

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    IndependentItem, ToggleItem, category, independent_toggle_buttons, separator, settings_row,
    toggle_buttons,
};

pub fn bar_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Bar");
    let snapshot = ctx.snapshot();
    let _bar = &snapshot.interface.bar;

    let position = toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: Some("Top"),
                icon: Some("go-up-symbolic"),
                value: "top".to_string(),
            },
            ToggleItem {
                label: Some("Bottom"),
                icon: Some("go-down-symbolic"),
                value: "bottom".to_string(),
            },
            ToggleItem {
                label: Some("Left"),
                icon: Some("go-previous-symbolic"),
                value: "left".to_string(),
            },
            ToggleItem {
                label: Some("Right"),
                icon: Some("go-next-symbolic"),
                value: "right".to_string(),
            },
        ],
        |s: &Settings| s.interface.bar.side.clone(),
        |s, value: String| s.interface.bar.side = value,
    );
    let position_row = settings_row(Some("Position"), Some("Pick a side for the bar."), false);
    position_row.append(&position.container);
    container.append(&position_row);
    container.append(&separator());

    let density = toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: Some("Condensed"),
                value: -2,
                icon: None,
            },
            ToggleItem {
                label: Some("Compact"),
                value: -1,
                icon: None,
            },
            ToggleItem {
                label: Some("Comfortable"),
                value: 0,
                icon: None,
            },
            ToggleItem {
                label: Some("Cozy"),
                value: 1,
                icon: None,
            },
        ],
        |s: &Settings| s.interface.bar.density,
        |s, value| s.interface.bar.density = value,
    );
    let density_row = settings_row(
        Some("Density"),
        Some("Pick between 4 different density options."),
        false,
    );
    density_row.append(&density.container);
    container.append(&density_row);
    container.append(&separator());

    let modifiers = independent_toggle_buttons(
        ctx,
        vec![
            IndependentItem {
                label: Some("Floating"),
                icon: Some("window-new-symbolic"),
                get: Box::new(|s: &Settings| s.interface.bar.floating),
                set: Box::new(|s: &mut Settings, active| s.interface.bar.floating = active),
            },
            IndependentItem {
                label: Some("Separated"),
                icon: Some("view-list-symbolic"),
                get: Box::new(|s: &Settings| s.interface.bar.separation),
                set: Box::new(|s: &mut Settings, active| s.interface.bar.separation = active),
            },
            IndependentItem {
                label: Some("Centered"),
                icon: Some("format-justify-center-symbolic"),
                get: Box::new(|s: &Settings| s.interface.bar.centered),
                set: Box::new(|s: &mut Settings, active| s.interface.bar.centered = active),
            },
        ],
    );
    let modifier_row = settings_row(
        Some("Extra Modifiers"),
        Some("Add extra modifiers to the bar (you can select multiple)."),
        false,
    );
    modifier_row.append(&modifiers.container);
    container.append(&modifier_row);
    container.append(&separator());

    let backgrounds = independent_toggle_buttons(
        ctx,
        vec![
            IndependentItem {
                label: Some("Bar"),
                icon: None,
                get: Box::new(|s: &Settings| s.interface.bar.bar_background),
                set: Box::new(|s: &mut Settings, active| s.interface.bar.bar_background = active),
            },
            IndependentItem {
                label: Some("Modules"),
                icon: None,
                get: Box::new(|s: &Settings| s.interface.bar.module_backgrounds),
                set: Box::new(|s: &mut Settings, active| {
                    s.interface.bar.module_backgrounds = active
                }),
            },
        ],
    );
    let background_row = settings_row(
        Some("Backgrounds"),
        Some("Add or remove the backgrounds of the bar and modules."),
        false,
    );
    background_row.append(&backgrounds.container);
    container.append(&background_row);

    container
}
