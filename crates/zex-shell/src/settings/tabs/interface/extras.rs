//! Interface tab extra module options category.

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    ToggleItem, category, separator, settings_row, spin_row, switch_row, toggle_buttons,
};

pub fn extra_options_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Extra Module Options");
    let snapshot = ctx.snapshot();
    let options = &snapshot.interface.modules.options;

    let style = toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: Some("Numbers"),
                icon: Some("format-list-numbered-symbolic"),
                value: "numbers".to_string(),
            },
            ToggleItem {
                label: Some("Dots"),
                icon: Some("view-more-symbolic"),
                value: "dots".to_string(),
            },
        ],
        |s: &Settings| s.interface.modules.options.workspaces_style.clone(),
        |s, value: String| s.interface.modules.options.workspaces_style = value,
    );
    let style_row = settings_row(
        Some("Workspace Indicator Style"),
        Some("Pick between 2 different workspace indicator styles."),
        false,
    );
    style_row.append(&style.container);
    container.append(&style_row);
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Fixed Workspaces"),
        Some("Show a specific amount of workspaces."),
        options.fixed_workspaces_enabled,
        |s: &mut Settings, active| s.interface.modules.options.fixed_workspaces_enabled = active,
    ));
    container.append(&separator());

    container.append(&spin_row(
        ctx,
        Some("Fixed Workspaces Amount"),
        Some("How many workspaces to show."),
        1,
        20,
        options.fixed_workspaces_amount as i32,
        |s: &mut Settings, value| s.interface.modules.options.fixed_workspaces_amount = value as u8,
    ));
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Use 24 hour time"),
        Some("Toggle between 12-hour (AM/PM) and 24-hour time formats."),
        options.military_time,
        |s: &mut Settings, active| s.interface.modules.options.military_time = active,
    ));
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Show the date"),
        Some("Toggle the visibility of the date in the bar."),
        options.show_date,
        |s: &mut Settings, active| s.interface.modules.options.show_date = active,
    ));
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Swap the day and month"),
        Some("Use the American date format."),
        options.day_month_swapped,
        |s: &mut Settings, active| s.interface.modules.options.day_month_swapped = active,
    ));

    container
}
