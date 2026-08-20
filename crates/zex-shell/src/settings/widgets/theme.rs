//! Light/dark theme selector rendered from the current palette previews

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::settings::tabs::TabContext;

/// Light/dark theme selector rendered from the current palette previews
pub fn theme_selector(ctx: &TabContext) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    row.add_css_class("theme-selector-row");
    row.set_vexpand(true);
    row.set_valign(gtk4::Align::Fill);

    let store = Rc::clone(&ctx.store);
    let registry: Rc<RefCell<Vec<(bool, gtk4::Button)>>> = Rc::new(RefCell::new(Vec::new()));

    let light = theme_button("Light", false, "light-preview", "weather-clear-symbolic");
    let dark = theme_button("Dark", true, "dark-preview", "weather-clear-night-symbolic");
    registry.borrow_mut().push((false, light.button.clone()));
    registry.borrow_mut().push((true, dark.button.clone()));
    row.append(&light.button);
    row.append(&dark.button);

    let store = Rc::clone(&store);
    let registry = Rc::clone(&registry);
    sync_theme_selection(&registry, &store);

    for (is_dark, button) in [(false, light), (true, dark)] {
        let registry = Rc::clone(&registry);
        let store = Rc::clone(&store);
        button.button.connect_clicked(move |_| {
            if let Err(err) = store.borrow_mut().update(|s| {
                s.appearance.wallcolors.dark_mode = is_dark;
            }) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
            sync_theme_selection(&registry, &store);
        });
    }

    row
}

struct ThemeButton {
    button: gtk4::Button,
}

fn theme_button(
    label: &str,
    _is_dark: bool,
    preview_class: &'static str,
    icon: &str,
) -> ThemeButton {
    let button = gtk4::Button::new();
    button.add_css_class("theme-preview-btn");
    button.set_overflow(gtk4::Overflow::Hidden);
    button.set_hexpand(true);
    button.set_halign(gtk4::Align::Fill);
    button.set_vexpand(true);
    button.set_valign(gtk4::Align::Fill);

    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    outer.add_css_class(preview_class);
    outer.set_hexpand(true);
    outer.set_vexpand(true);
    outer.set_valign(gtk4::Align::Fill);

    let container = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
    container.add_css_class("container");
    container.set_hexpand(true);
    container.set_vexpand(true);

    let surface = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    surface.add_css_class("surface");
    surface.set_width_request(40);
    surface.set_vexpand(true);
    surface.set_valign(gtk4::Align::Fill);

    let center = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    center.set_halign(gtk4::Align::Center);
    center.set_hexpand(true);
    center.set_valign(gtk4::Align::Center);
    center.set_vexpand(true);

    let icon = gtk4::Image::from_icon_name(icon);
    icon.add_css_class("icon");
    icon.set_pixel_size(18);
    center.append(&icon);
    let label = gtk4::Label::new(Some(label));
    center.append(&label);
    surface.append(&center);

    container.append(&surface);

    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
    let mut colors = Vec::new();
    for class in ["btn-1", "btn-2", "btn-3"] {
        let swatch = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        swatch.add_css_class(class);
        swatch.set_width_request(30);
        swatch.set_height_request(30);
        colors.push(swatch);
    }
    colors[0].set_hexpand(true);
    colors[0].set_halign(gtk4::Align::Fill);
    for swatch in colors {
        buttons.append(&swatch);
    }
    container.append(&buttons);

    outer.append(&container);
    button.set_child(Some(&outer));
    ThemeButton { button }
}

fn sync_theme_selection(
    registry: &Rc<RefCell<Vec<(bool, gtk4::Button)>>>,
    store: &Rc<RefCell<zex_core::SettingsStore>>,
) {
    let dark = store.borrow().get().appearance.wallcolors.dark_mode;
    for (is_dark, button) in registry.borrow().iter() {
        if *is_dark == dark {
            button.add_css_class("selected");
        } else {
            button.remove_css_class("selected");
        }
    }
}
