mod notifications;
mod osd;
mod recording;

use gtk4::prelude::*;

use super::TabContext;

/// Build the whole services tab.
pub fn build(ctx: &TabContext) -> gtk4::Box {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body.add_css_class("settings-body");
    body.set_halign(gtk4::Align::Center);
    body.set_hexpand(false);
    body.set_width_request(800);
    body.append(&notifications::notifications_category(ctx));
    body.append(&recording::recording_category(ctx));
    body.append(&osd::osd_category(ctx));
    body
}
