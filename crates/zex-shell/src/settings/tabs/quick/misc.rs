//! Quick tab miscellaneous category: shell and screen corner options

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    ToggleItem, category, separator, settings_row, switch_row, toggle_buttons,
};

pub fn misc_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Miscellaneous");

    let snapshot = ctx.snapshot();
    let shell_corners = snapshot.interface.misc.shell_corners;

    let shell = switch_row(
        ctx,
        Some("Rounded Shell Corners"),
        Some("Add a curve outside the shell that warps around the screen."),
        shell_corners,
        |s: &mut Settings, active| s.interface.misc.shell_corners = active,
    );
    container.append(&shell);
    container.append(&separator());

    let corners = toggle_buttons(
        ctx,
        vec![
            ToggleItem {
                label: Some("Disabled"),
                icon: Some("window-close-symbolic"),
                value: "disabled".to_string(),
            },
            ToggleItem {
                label: Some("When not fullscreen"),
                icon: None,
                value: "not_fullscreen".to_string(),
            },
            ToggleItem {
                label: Some("Always"),
                icon: Some("select-all-symbolic"),
                value: "always".to_string(),
            },
        ],
        |s: &Settings| s.interface.misc.screen_corners.clone(),
        |s, value: String| s.interface.misc.screen_corners = value,
    );
    let corners_row = settings_row(
        Some("Rounded Screen Corners"),
        Some("Round the corners of the screen."),
        false,
    );
    corners_row.append(&corners.container);
    container.append(&corners_row);

    container
}
