//! M3 slider: an icon plus a filled GTK scale in a rounded shell.

use gtk4::glib;
use gtk4::prelude::*;

pub struct M3Slider {
    pub container: gtk4::Box,
    pub icon: gtk4::Image,
    pub scale: gtk4::Scale,
}

impl M3Slider {
    /// Build a horizontal slider (0..1 by default) with an optional icon
    pub fn new(icon_name: Option<&str>) -> Self {
        let icon = gtk4::Image::new();
        icon.add_css_class("m3-slider-icon");
        match icon_name {
            Some(name) => icon.set_icon_name(Some(name)),
            None => icon.set_visible(false),
        }

        let adjustment = gtk4::Adjustment::new(0.0, 0.0, 1.0, 0.01, 0.0, 0.0);
        let scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&adjustment));
        scale.set_halign(gtk4::Align::Fill);
        scale.set_hexpand(true);
        scale.set_draw_value(true);
        scale.set_value_pos(gtk4::PositionType::Top);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        container.set_hexpand(true);
        container.add_css_class("m3-slider");
        container.append(&icon);
        container.append(&scale);

        Self {
            container,
            icon,
            scale,
        }
    }

    pub fn set_range(&self, min: f64, max: f64) {
        let adjustment = self.scale.adjustment();
        adjustment.set_lower(min);
        adjustment.set_upper(max);
    }

    pub fn value(&self) -> f64 {
        self.scale.value()
    }

    pub fn set_value(&self, value: f64) {
        self.scale.set_value(value);
    }

    pub fn set_sensitive(&self, sensitive: bool) {
        self.scale.set_sensitive(sensitive);
    }

    pub fn connect_value_changed<F: FnMut(f64) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        let callback = std::cell::RefCell::new(f);
        self.scale.connect_value_changed(move |scale| {
            callback.borrow_mut()(scale.value());
        })
    }
}
