//! `zex-m3test`: a window showing every M3 variant for visual checks.

use gtk4::prelude::*;

use super::button::{ConnectedButtonGroup, M3Button, M3Shape, M3Size, M3Type};
use super::navigation_rail::NavigationRail;
use super::slider::M3Slider;

const TYPES: [M3Type; 5] = [
    M3Type::Elevated,
    M3Type::Filled,
    M3Type::Tonal,
    M3Type::Outlined,
    M3Type::Text,
];

const SIZES: [M3Size; 5] = [M3Size::Xs, M3Size::S, M3Size::M, M3Size::L, M3Size::Xl];

/// Build the showcase window; the caller keeps it alive and presents it
pub fn window() -> gtk4::Window {
    let window = gtk4::Window::new();
    window.set_title(Some("Material 3 Testing Window"));
    window.set_default_size(680, 900);
    window.set_hide_on_close(true);
    window.add_css_class("m3-testing-window");

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    content.set_halign(gtk4::Align::Center);
    content.set_hexpand(true);
    content.set_valign(gtk4::Align::Start);

    let section = |title: &str| {
        let label = gtk4::Label::new(Some(title));
        label.add_css_class("settings-category-label");
        content.append(&label);
    };
    let button_row = |icon: Option<&str>, label: &str| {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        for kind in TYPES {
            let button = M3Button::new(icon, Some(label), kind, M3Size::S, M3Shape::Round);
            row.append(&button.button);
        }
        content.append(&row);
    };

    section("Label Only");
    button_row(None, "Button");

    section("Icon + Label");
    button_row(Some("edit-symbolic"), "Button");

    section("Icon Only");
    button_row(Some("edit-symbolic"), "");

    for size in SIZES {
        section(match size {
            M3Size::Xs => "Extra Small",
            M3Size::S => "Small",
            M3Size::M => "Medium",
            M3Size::L => "Large",
            M3Size::Xl => "Extra Large",
        });
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        for kind in TYPES {
            let button = M3Button::new(
                Some("edit-symbolic"),
                Some("Button"),
                kind,
                size,
                M3Shape::Round,
            );
            row.append(&button.button);
        }
        content.append(&row);
    }

    section("Connected Button Groups");
    let group = ConnectedButtonGroup::new();
    for index in 0..5 {
        let button = M3Button::new(
            None,
            Some(format!("Segment {}", index + 1).as_str()),
            M3Type::Tonal,
            M3Size::S,
            M3Shape::Square,
        );
        if index == 0 {
            button.set_active(true);
        }
        group.add(&button);
    }
    content.append(&group.container);

    section("Sliders");
    let slider = M3Slider::new(Some("audio-volume-high-symbolic"));
    slider.set_range(0.0, 100.0);
    slider.set_value(60.0);
    content.append(&slider.container);
    let disabled = M3Slider::new(Some("audio-volume-high-symbolic"));
    disabled.set_range(0.0, 100.0);
    disabled.set_value(10.0);
    disabled.set_sensitive(false);
    content.append(&disabled.container);

    section("Navigation Rail");
    let rail = NavigationRail::new();
    rail.set_on_select(|key| tracing::info!("rail selected {key}"));
    rail.add_item("home", "home-symbolic", "Home");
    rail.add_item("search", "system-search-symbolic", "Search");
    rail.add_item("settings", "emblem-system-symbolic", "Settings");
    rail.select("home");
    content.append(&rail.container);

    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&content));
    window.set_child(Some(&scrolled));

    window
}
