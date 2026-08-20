//! Appearance tab colors category: theme, auto dark, schemes and palette row

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    PaletteRegistry, category, palette_button, separator, settings_row, spin_button, switch_row,
    theme_selector,
};

pub fn colors_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Colors");
    let snapshot = ctx.snapshot();
    let auto_dark = snapshot.appearance.wallcolors.auto_dark.clone();
    let color_scheme = snapshot.appearance.wallcolors.color_scheme.clone();
    let dark_mode = snapshot.appearance.wallcolors.dark_mode;

    let themes_row = settings_row(Some("Themes"), Some("Set your theme."), true);
    themes_row.add_css_class("colors-row");
    themes_row.append(&theme_selector(ctx));
    container.append(&themes_row);
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Auto Dark"),
        Some("Automatically set the dark theme based on the time of day."),
        auto_dark.enabled,
        |s: &mut Settings, active| s.appearance.wallcolors.auto_dark.enabled = active,
    ));
    container.append(&separator());

    let start_hour = auto_dark.start_hour as i32;
    let start_min = auto_dark.start_min as i32;
    container.append(&time_row(
        ctx,
        Some("Start Time"),
        Some("Time of day to enable dark mode when Auto Dark is enabled."),
        start_hour,
        start_min,
        |s: &mut Settings, hour| s.appearance.wallcolors.auto_dark.start_hour = hour as u8,
        |s: &mut Settings, min| s.appearance.wallcolors.auto_dark.start_min = min as u8,
    ));
    container.append(&separator());

    let end_hour = auto_dark.end_hour as i32;
    let end_min = auto_dark.end_min as i32;
    container.append(&time_row(
        ctx,
        Some("End Time"),
        Some("Time of day to disable dark mode when Auto Dark is enabled."),
        end_hour,
        end_min,
        |s: &mut Settings, hour| s.appearance.wallcolors.auto_dark.end_hour = hour as u8,
        |s: &mut Settings, min| s.appearance.wallcolors.auto_dark.end_min = min as u8,
    ));
    container.append(&separator());

    let schemes_row = settings_row(Some("Color Schemes"), Some("Set your color scheme."), true);
    schemes_row.add_css_class("colors-row");
    schemes_row.append(&palette_row(ctx, &color_scheme, dark_mode));
    container.append(&schemes_row);

    container
}

fn time_row(
    ctx: &TabContext,
    title: Option<&str>,
    description: Option<&str>,
    hour: i32,
    minute: i32,
    set_hour: impl Fn(&mut Settings, i32) + 'static,
    set_minute: impl Fn(&mut Settings, i32) + 'static,
) -> gtk4::Box {
    let row = settings_row(title, description, false);
    let hours = spin_button(ctx, 0, 23, hour, set_hour);
    hours.set_width_request(70);
    row.append(&hours);
    let minutes = spin_button(ctx, 0, 59, minute, set_minute);
    minutes.set_width_request(70);
    row.append(&minutes);
    row
}

fn palette_row(ctx: &TabContext, color_scheme: &str, dark_mode: bool) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    row.set_spacing(2);

    let store = Rc::clone(&ctx.store);
    let registry: PaletteRegistry = Rc::new(RefCell::new(Vec::new()));

    for (index, preview) in ctx.previews.iter().enumerate() {
        let selected = preview.scheme == color_scheme && preview.dark == dark_mode;
        let button = palette_button(index, preview, selected);
        registry
            .borrow_mut()
            .push((preview.scheme.clone(), preview.dark, button.clone()));
        row.append(&button);
    }

    let store = Rc::clone(&store);
    let registry = Rc::clone(&registry);
    for (scheme, dark, button) in registry.borrow().iter() {
        let scheme = scheme.clone();
        let dark = *dark;
        let registry = Rc::clone(&registry);
        let store = Rc::clone(&store);
        button.connect_clicked(move |_| {
            if let Err(err) = store.borrow_mut().update(|s| {
                let wc = &mut s.appearance.wallcolors;
                wc.color_scheme = scheme.clone();
                wc.dark_mode = dark;
            }) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
            sync_palette_selection(&registry, &store);
        });
    }

    sync_palette_selection(&registry, &store);
    row
}

fn sync_palette_selection(
    registry: &PaletteRegistry,
    store: &Rc<RefCell<zex_core::SettingsStore>>,
) {
    let binding = store.borrow();
    let wc = &binding.get().appearance.wallcolors;
    let scheme = &wc.color_scheme;
    let dark = wc.dark_mode;
    for (entry_scheme, entry_dark, button) in registry.borrow().iter() {
        let selected = entry_scheme == scheme && *entry_dark == dark;
        if selected {
            button.add_css_class("selected");
        } else {
            button.remove_css_class("selected");
        }
    }
}
