//! Interface tab: bars, bar modules and extra module options.

mod bar2;
mod extras;
mod modules;

use gtk4::prelude::*;

use super::TabContext;
use super::quick::{bar_category, misc_category};

/// Build the whole interface tab.
pub fn build(ctx: &TabContext) -> gtk4::Box {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body.add_css_class("settings-body");
    body.set_halign(gtk4::Align::Center);
    body.set_hexpand(false);
    body.set_width_request(800);
    body.append(&bar_category(ctx));
    body.append(&bar2::bar2_category(ctx));
    body.append(&modules::module_category(ctx));
    body.append(&extras::extra_options_category(ctx));
    body.append(&misc_category(ctx));
    body
}
