//! Battery: percentage fill bar with charging state icons and low/charging colors.

use std::rc::Rc;

use gtk4::prelude::*;
use zex_services::upower::Battery;

/// Material-glyph status for the battery, mirroring the reference tiers
pub fn status_icon(charging: bool, percent: u8) -> &'static str {
    if charging {
        "bolt"
    } else {
        match percent {
            100 => "battery_android_full",
            96..=99 => "battery_android_6",
            81..=95 => "battery_android_5",
            61..=80 => "battery_android_4",
            41..=60 => "battery_android_3",
            26..=40 => "battery_android_2",
            11..=25 => "battery_android_1",
            0..=10 => "battery_android_0",
            _ => "battery_android_question",
        }
    }
}

pub struct BatteryWidget {
    container: gtk4::Box,
    battery_box: gtk4::Overlay,
    fill: gtk4::Box,
    status: gtk4::Label,
    percent: gtk4::Label,
}

impl BatteryWidget {
    pub fn new() -> Rc<Self> {
        let status = gtk4::Label::new(None);
        status.set_css_classes(&["battery-status"]);

        let percent = gtk4::Label::new(None);
        percent.set_css_classes(&["battery-percent-label"]);

        let text_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        text_container.set_spacing(2);
        text_container.set_halign(gtk4::Align::Center);
        text_container.set_valign(gtk4::Align::Center);
        text_container.append(&status);
        text_container.append(&percent);

        let fill = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        fill.set_css_classes(&["battery-fill"]);

        let battery_box = gtk4::Overlay::new();
        battery_box.set_css_classes(&["battery-box"]);
        battery_box.set_overflow(gtk4::Overflow::Hidden);
        battery_box.set_halign(gtk4::Align::Center);
        battery_box.set_valign(gtk4::Align::Center);
        battery_box.set_child(Some(&fill));
        battery_box.add_overlay(&text_container);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        container.set_css_classes(&["battery-widget"]);
        container.append(&battery_box);

        Rc::new(Self {
            container,
            battery_box,
            fill,
            status,
            percent,
        })
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    /// Refresh on battery events; vertical layout grows the fill downward
    pub fn update(&self, batteries: &[Battery], vertical: bool) {
        let battery = batteries.iter().find(|battery| battery.is_present);
        let Some(battery) = battery else {
            self.container.set_visible(false);
            return;
        };

        let percent = battery.percent_u8();
        let charging = battery.charging();

        if vertical {
            self.fill.set_vexpand(true);
            self.fill.set_valign(gtk4::Align::End);
            self.fill.set_height_request((40 * percent as i32) / 100);
            self.fill.set_width_request(26);
            self.percent.set_label(&percent.to_string());
        } else {
            self.fill.set_hexpand(true);
            self.fill.set_halign(gtk4::Align::Start);
            self.fill.set_height_request(26);
            self.fill.set_width_request((50 * percent as i32) / 100);
            self.percent.set_label(&format!("{percent}%"));
        }
        if vertical {
            self.battery_box.set_size_request(26, 40);
        } else {
            self.battery_box.set_size_request(50, 26);
        }
        self.status.set_label(status_icon(charging, percent));

        self.container.set_visible(true);
        if charging {
            self.container.add_css_class("charging");
        } else {
            self.container.remove_css_class("charging");
        }
        if percent <= 20 {
            self.container.add_css_class("low");
        } else {
            self.container.remove_css_class("low");
        }
    }
}
