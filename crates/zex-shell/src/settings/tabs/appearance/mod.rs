//! Appearance tab: wallpaper, colors and the quick-select gallery.

mod colors;
mod quickselect;

use gtk4::prelude::*;

use super::TabContext;
use crate::settings::widgets::{category, wallpaper_overlay};

fn wallpaper_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Wallpaper");
    container.append(&wallpaper_overlay(ctx));
    container
}

/// Build the whole appearance tab.
pub fn build(ctx: &TabContext) -> gtk4::Box {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body.add_css_class("settings-body");
    body.set_halign(gtk4::Align::Center);
    body.set_hexpand(false);
    body.set_width_request(800);
    body.append(&wallpaper_category(ctx));
    body.append(&colors::colors_category(ctx));
    body.append(&quickselect::quick_select_category(ctx));
    body
}
