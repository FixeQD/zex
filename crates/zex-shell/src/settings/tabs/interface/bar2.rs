//! Interface tab second bar category: position, density, modifiers.

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    IndependentItem, ToggleItem, category, independent_toggle_buttons, separator, settings_row,
    toggle_buttons,
};

pub fn bar2_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Second Bar");
    let snapshot = ctx.snapshot();
    let _bar = &snapshot.interface.bar2;

    let description = settings_row(
        None,
        Some(
            "The second bar is automatically enabled when a module is located \
             in it, and stays enabled while any modules remain there.",
        ),
        false,
    );
    container.append(&description);
    container.append(&separator());

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
        |s: &Settings| s.interface.bar2.side.clone(),
        |s, value: String| s.interface.bar2.side = value,
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
        |s: &Settings| s.interface.bar2.density,
        |s, value| s.interface.bar2.density = value,
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
        modifiers_items(
            |s: &Settings| s.interface.bar2.floating,
            |s: &mut Settings, active| s.interface.bar2.floating = active,
            |s: &Settings| s.interface.bar2.separation,
            |s: &mut Settings, active| s.interface.bar2.separation = active,
            |s: &Settings| s.interface.bar2.centered,
            |s: &mut Settings, active| s.interface.bar2.centered = active,
        ),
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
        backgrounds_items(
            |s: &Settings| s.interface.bar2.bar_background,
            |s: &mut Settings, active| s.interface.bar2.bar_background = active,
            |s: &Settings| s.interface.bar2.module_backgrounds,
            |s: &mut Settings, active| s.interface.bar2.module_backgrounds = active,
        ),
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

#[allow(clippy::too_many_arguments)]
fn modifiers_items(
    get_floating: impl Fn(&Settings) -> bool + 'static,
    set_floating: impl Fn(&mut Settings, bool) + 'static,
    get_separation: impl Fn(&Settings) -> bool + 'static,
    set_separation: impl Fn(&mut Settings, bool) + 'static,
    get_centered: impl Fn(&Settings) -> bool + 'static,
    set_centered: impl Fn(&mut Settings, bool) + 'static,
) -> Vec<IndependentItem> {
    vec![
        IndependentItem {
            label: Some("Floating"),
            icon: Some("window-new-symbolic"),
            get: Box::new(get_floating),
            set: Box::new(set_floating),
        },
        IndependentItem {
            label: Some("Separated"),
            icon: Some("view-list-symbolic"),
            get: Box::new(get_separation),
            set: Box::new(set_separation),
        },
        IndependentItem {
            label: Some("Centered"),
            icon: Some("format-justify-center-symbolic"),
            get: Box::new(get_centered),
            set: Box::new(set_centered),
        },
    ]
}

fn backgrounds_items(
    get_bar: impl Fn(&Settings) -> bool + 'static,
    set_bar: impl Fn(&mut Settings, bool) + 'static,
    get_modules: impl Fn(&Settings) -> bool + 'static,
    set_modules: impl Fn(&mut Settings, bool) + 'static,
) -> Vec<IndependentItem> {
    vec![
        IndependentItem {
            label: Some("Bar"),
            icon: None,
            get: Box::new(get_bar),
            set: Box::new(set_bar),
        },
        IndependentItem {
            label: Some("Modules"),
            icon: None,
            get: Box::new(get_modules),
            set: Box::new(set_modules),
        },
    ]
}
