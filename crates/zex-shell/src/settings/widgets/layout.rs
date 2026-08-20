//! Layout primitives: category containers, rows and separators

use gtk4::prelude::*;

pub fn category(text: &str) -> gtk4::Box {
    let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    container.add_css_class("settings-category");
    container.append(&category_label(text));
    container
}

pub fn category_label(text: &str) -> gtk4::Label {
    let label = gtk4::Label::new(Some(text));
    label.add_css_class("settings-category-label");
    label.set_halign(gtk4::Align::Start);
    label.set_hexpand(true);
    label
}

pub fn settings_row(title: Option<&str>, description: Option<&str>, vertical: bool) -> gtk4::Box {
    let orientation = if vertical {
        gtk4::Orientation::Vertical
    } else {
        gtk4::Orientation::Horizontal
    };
    let container = gtk4::Box::new(orientation, 5);
    container.add_css_class("settings-row");
    container.set_halign(gtk4::Align::Fill);
    container.set_hexpand(true);

    let header = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    header.add_css_class("settings-row-header");
    header.set_valign(gtk4::Align::Center);
    header.set_halign(gtk4::Align::Start);
    header.set_hexpand(!vertical);
    if let Some(title) = title {
        let label = gtk4::Label::new(Some(title));
        label.add_css_class("settings-row-title");
        label.set_halign(gtk4::Align::Start);
        header.append(&label);
    }
    if let Some(description) = description {
        let label = gtk4::Label::new(Some(description));
        label.add_css_class("settings-row-description");
        label.set_halign(gtk4::Align::Start);
        header.append(&label);
    }
    container.append(&header);
    container
}

pub fn separator() -> gtk4::Separator {
    gtk4::Separator::new(gtk4::Orientation::Horizontal)
}

pub fn vertical_separator() -> gtk4::Separator {
    let sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
    sep.add_css_class("module-separator");
    sep
}
