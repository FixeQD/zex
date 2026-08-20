//! Quick settings tab: appearance, bar and miscellaneous shortcuts

mod bar;
mod misc;
mod wallcolors;

pub use bar::bar_category;
pub use misc::misc_category;

use gtk4::prelude::*;

use super::TabContext;

/// Build the whole quick tab.
pub fn build(ctx: &TabContext) -> gtk4::Box {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body.add_css_class("settings-body");
    body.set_halign(gtk4::Align::Center);
    body.set_hexpand(false);
    body.set_width_request(800);
    body.append(&wallcolors::wallcolor_category(ctx));
    body.append(&bar_category(ctx));
    body.append(&misc_category(ctx));
    body
}
