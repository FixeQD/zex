//! Launcher button widget

use gtk4::Align;
use gtk4::prelude::*;

/// Build the launcher button; `on_click` fires on every click
pub fn new(on_click: impl Fn() + 'static) -> gtk4::Button {
    let button = gtk4::Button::with_label("apps");
    button.set_css_classes(&["m3-icon", "launcher-button"]);
    button.set_hexpand(true);
    button.set_vexpand(true);
    button.set_halign(Align::Fill);
    button.set_valign(Align::Fill);
    button.connect_clicked(move |_| on_click());
    button
}
