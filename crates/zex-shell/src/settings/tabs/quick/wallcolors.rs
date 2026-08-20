//! Quick tab appearance category

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    PaletteRegistry, category, palette_button, theme_selector, wallpaper_overlay,
};

pub fn wallcolor_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Appearance");

    let snapshot = ctx.snapshot();
    let color_scheme = snapshot.appearance.wallcolors.color_scheme.clone();
    let dark_mode = snapshot.appearance.wallcolors.dark_mode;

    let wallpaper = wallpaper_overlay(ctx);

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    column.append(&theme_selector(ctx));
    column.append(&palette_grid(ctx, &color_scheme, dark_mode));

    let main = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    main.set_halign(gtk4::Align::Center);
    main.set_valign(gtk4::Align::Center);
    main.append(&wallpaper);
    main.append(&column);
    container.append(&main);
    container
}

fn palette_grid(ctx: &TabContext, color_scheme: &str, dark_mode: bool) -> gtk4::Grid {
    let grid = gtk4::Grid::new();
    grid.set_column_spacing(5);
    grid.set_row_spacing(5);
    grid.add_css_class("palette-selector-row");

    let store = Rc::clone(&ctx.store);
    let registry: PaletteRegistry = Rc::new(RefCell::new(Vec::new()));

    for (index, preview) in ctx.previews.iter().enumerate() {
        let selected = preview.scheme == color_scheme && preview.dark == dark_mode;
        let button = palette_button(index, preview, selected);
        registry
            .borrow_mut()
            .push((preview.scheme.clone(), preview.dark, button.clone()));
        grid.attach(&button, (index % 3) as i32, (index / 3) as i32, 1, 1);
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
    grid
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
