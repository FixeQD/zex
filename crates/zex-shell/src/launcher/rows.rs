//! Result rows, the pinned grid and the per-app context menu

use std::rc::Rc;

use gtk4::prelude::*;
use relm4::Sender;
use zex_launcher::apps::{AppInfo, PinnedApps};
use zex_launcher::items::Item;

use super::LauncherMsg;
use super::icons::{self, ICON_SIZE};

pub const GRID_COLUMNS: u32 = 5;
pub const PINNED_PER_ROW: i32 = 8;

pub struct ItemRow {
    pub container: gtk4::Widget,
    pub button: gtk4::Button,
    pub app: Option<AppInfo>,
}

fn wire_context_menu(
    button: &gtk4::Button,
    app: AppInfo,
    pins: Rc<PinnedApps>,
    guard: Rc<std::cell::Cell<bool>>,
    sender: Sender<LauncherMsg>,
) {
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3);
    let target = button.clone();
    gesture.connect_pressed(move |_, _, _, _| {
        guard.set(true);
        let close_guard = Rc::clone(&guard);
        let popover = app_popover(&app, &pins, sender.clone());
        popover.connect_closed(move |_| {
            close_guard.set(false);
        });
        popover.set_parent(&target);
        popover.popup();
    });
    button.add_controller(gesture);
}

pub fn item_row(
    item: &Item,
    index: usize,
    grid: bool,
    featured: bool,
    pins: Rc<PinnedApps>,
    guard: Rc<std::cell::Cell<bool>>,
    sender: Sender<LauncherMsg>,
) -> ItemRow {
    let app = match item {
        Item::App(app) => Some(app.clone()),
        _ => None,
    };

    let (icon_size, icon_class) = if featured {
        (icons::FEATURED_ICON_SIZE, "launcher-icon-featured")
    } else if grid {
        (icons::GRID_ICON_SIZE, "launcher-grid-icon")
    } else {
        (ICON_SIZE, "launcher-icon")
    };

    let icon = match app.as_ref() {
        Some(app) => icons::icon_widget(app, icon_size, icon_class),
        None => icons::fallback_icon(item, icon_size, icon_class),
    };

    let (title, subtitle) = (item.title(), item.subtitle());

    let labels = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    labels.set_halign(gtk4::Align::Start);
    labels.set_valign(gtk4::Align::Center);
    labels.set_hexpand(true);
    labels.set_css_classes(&["launcher-row-text"]);

    let name = gtk4::Label::new(Some(&title));
    name.set_css_classes(&["launcher-row-name"]);
    name.set_halign(gtk4::Align::Start);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name.set_xalign(0.0);
    labels.append(&name);

    if let Some(subtitle) = subtitle {
        let description = gtk4::Label::new(Some(&subtitle));
        description.set_css_classes(&["launcher-row-description"]);
        description.set_halign(gtk4::Align::Start);
        description.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        description.set_xalign(0.0);
        labels.append(&description);
    }

    let content = if grid {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
        box_.set_halign(gtk4::Align::Center);
        box_.set_valign(gtk4::Align::Center);
        box_.append(&icon);
        box_.append(&labels);
        box_
    } else {
        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        box_.set_halign(gtk4::Align::Fill);
        box_.set_valign(gtk4::Align::Center);
        box_.append(&icon);
        box_.append(&labels);
        box_
    };

    let button = gtk4::Button::new();
    button.set_can_focus(false);
    if featured {
        button.add_css_class("featured");
    }
    if grid {
        button.add_css_class("launcher-grid-item");
    } else {
        button.add_css_class("launcher-row");
    }
    if featured {
        button.add_css_class("launcher-row");
    }
    button.set_child(Some(&content));
    button.set_hexpand(true);

    let click_sender = sender.clone();
    button.connect_clicked(move |_| {
        let _ = click_sender.send(LauncherMsg::ActivateAt(index));
    });

    let container: gtk4::Widget = if grid {
        let cell = gtk4::FlowBoxChild::new();
        cell.set_child(Some(&button));
        cell.upcast()
    } else {
        let row = gtk4::ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);
        row.set_child(Some(&button));
        row.upcast()
    };

    if let Some(app) = app.clone() {
        wire_context_menu(&button, app, pins, guard, sender);
    }

    ItemRow {
        container,
        button,
        app,
    }
}

pub fn pinned_button(
    app: &AppInfo,
    pins: Rc<PinnedApps>,
    guard: Rc<std::cell::Cell<bool>>,
    sender: Sender<LauncherMsg>,
) -> gtk4::Button {
    let button = gtk4::Button::new();
    button.set_can_focus(false);
    button.set_size_request(64, 64);
    button.add_css_class("launcher-pinned-item");
    button.set_child(Some(&icons::icon_widget(app, 32, "launcher-pinned-icon")));

    let click_sender = sender.clone();
    let id = app.id.clone();
    button.connect_clicked(move |_| {
        let _ = click_sender.send(LauncherMsg::LaunchApp { id: id.clone() });
    });

    wire_context_menu(&button, app.clone(), pins, guard, sender);
    button
}

pub fn app_popover(app: &AppInfo, pins: &PinnedApps, sender: Sender<LauncherMsg>) -> gtk4::Popover {
    let popover = gtk4::Popover::new();
    popover.set_has_arrow(false);

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    content.set_css_classes(&["launcher-menu"]);

    let heading = gtk4::Label::new(Some(&app.title));
    heading.set_css_classes(&["launcher-menu-heading"]);
    heading.set_halign(gtk4::Align::Start);
    heading.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    heading.set_xalign(0.0);
    content.append(&heading);

    let pin_label = if pins.is_pinned(&app.id) {
        "Unpin App"
    } else {
        "Pin App"
    };
    let pin = gtk4::Button::with_label(pin_label);
    pin.set_can_focus(false);
    pin.set_css_classes(&["launcher-menu-item"]);
    pin.set_halign(gtk4::Align::Fill);
    let pin_sender = sender.clone();
    let id = app.id.clone();
    pin.connect_clicked(move |_| {
        let _ = pin_sender.send(LauncherMsg::Pin { id: id.clone() });
    });
    content.append(&pin);

    if !app.actions.is_empty() {
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        content.append(&separator);
    }
    for action in &app.actions {
        let row = gtk4::Button::with_label(&action.name);
        row.set_can_focus(false);
        row.set_css_classes(&["launcher-menu-item"]);
        row.set_halign(gtk4::Align::Fill);
        let action_sender = sender.clone();
        let command = action.command.clone();
        row.connect_clicked(move |_| {
            let _ = action_sender.send(LauncherMsg::LaunchAction {
                command: command.clone(),
            });
        });
        content.append(&row);
    }

    popover.set_child(Some(&content));
    popover
}
