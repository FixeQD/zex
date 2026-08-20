//! The launcher layer-shell window

use gtk4::prelude::*;
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use relm4::Sender;

use super::LauncherMsg;
use super::keys;
use super::rows::GRID_COLUMNS;
use crate::m3::{M3Button, M3Shape, M3Size, M3Type};

pub const LAUNCHER_WIDTH: i32 = 600;
pub const RESULTS_HEIGHT: i32 = 550;

pub struct LauncherWindow {
    pub root: gtk4::Window,
    pub entry: gtk4::SearchEntry,
    pub clear: M3Button,
    pub layout_toggle: M3Button,
    pub pinned: gtk4::Grid,
    pub pin_hint_icon: gtk4::Image,
    pub pin_hint_label: gtk4::Label,
    pub pinned_revealer: gtk4::Revealer,
    pub results_revealer: gtk4::Revealer,
    pub results_box: gtk4::Box,
    pub list: gtk4::ListBox,
    pub flow: gtk4::FlowBox,
    pub scroll: gtk4::ScrolledWindow,
    pub viewport: gtk4::Viewport,
    pub(crate) popover_open: std::rc::Rc<std::cell::Cell<bool>>,
}

/// Search entry glyph "search"
fn symbol_label(text: &str) -> gtk4::Label {
    let label = gtk4::Label::new(Some(text));
    label.set_css_classes(&["launcher-symbol"]);
    label
}

impl LauncherWindow {
    pub fn popover_open_cell(&self) -> std::rc::Rc<std::cell::Cell<bool>> {
        std::rc::Rc::clone(&self.popover_open)
    }

    pub fn new(sender: Sender<LauncherMsg>) -> Self {
        let root = gtk4::Window::new();
        root.init_layer_shell();
        root.set_layer(Layer::Overlay);
        root.set_namespace(Some("zex-launcher"));
        root.set_keyboard_mode(KeyboardMode::Exclusive);
        for edge in [
            gtk4_layer_shell::Edge::Top,
            gtk4_layer_shell::Edge::Bottom,
            gtk4_layer_shell::Edge::Left,
            gtk4_layer_shell::Edge::Right,
        ] {
            root.set_anchor(edge, true);
        }
        root.set_visible(false);

        let backdrop = gtk4::Button::new();
        backdrop.set_can_focus(false);
        backdrop.set_hexpand(true);
        backdrop.set_vexpand(true);
        backdrop.add_css_class("launcher-backdrop");
        backdrop.set_halign(gtk4::Align::Fill);
        backdrop.set_valign(gtk4::Align::Fill);

        let close_sender = sender.clone();
        backdrop.connect_clicked(move |_| {
            let _ = close_sender.send(LauncherMsg::Close);
        });

        let entry = gtk4::SearchEntry::new();
        entry.set_placeholder_text(Some("Search"));
        entry.set_css_classes(&["launcher-entry"]);
        entry.set_hexpand(true);
        entry.set_valign(gtk4::Align::Center);

        let query_sender = sender.clone();
        entry.connect_changed(move |entry| {
            let _ = query_sender.send(LauncherMsg::QueryChanged(entry.text().to_string()));
        });

        let clear = M3Button::new(
            Some("edit-clear-all-symbolic"),
            None,
            M3Type::Text,
            M3Size::Xs,
            M3Shape::Round,
        );
        clear.button.set_visible(false);
        clear.button.add_css_class("launcher-clear-button");
        let clear_sender = sender.clone();
        clear.connect_clicked(move |_| {
            let _ = clear_sender.send(LauncherMsg::ClearQuery);
        });

        let layout_toggle = M3Button::new(
            Some("view-grid-symbolic"),
            None,
            M3Type::Text,
            M3Size::Xs,
            M3Shape::Round,
        );
        layout_toggle.button.set_visible(false);
        layout_toggle.button.add_css_class("launcher-layout-button");
        let toggle_sender = sender.clone();
        layout_toggle.connect_clicked(move |_| {
            let _ = toggle_sender.send(LauncherMsg::ToggleLayout);
        });

        let search_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        search_bar.set_css_classes(&["launcher-search-bar"]);
        search_bar.set_hexpand(true);
        search_bar.append(&symbol_label("search"));
        search_bar.append(&entry);
        search_bar.append(&clear.button);
        search_bar.append(&layout_toggle.button);

        let pinned = gtk4::Grid::new();
        pinned.set_column_spacing(10);
        pinned.set_row_spacing(10);
        pinned.set_halign(gtk4::Align::Center);
        pinned.add_css_class("launcher-pinned");

        let pin_hint_icon = gtk4::Image::from_icon_name("view-pin-symbolic");
        pin_hint_icon.set_pixel_size(20);
        pin_hint_icon.add_css_class("launcher-pin-hint-icon");

        let pin_hint_label = gtk4::Label::new(Some("Pin an app from its right-click menu"));
        pin_hint_label.add_css_class("launcher-pin-hint");

        let pinned_revealer = gtk4::Revealer::new();
        pinned_revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        pinned_revealer.set_transition_duration(180);
        pinned_revealer.set_child(Some(&pinned));

        let list = gtk4::ListBox::new();
        list.set_selection_mode(gtk4::SelectionMode::None);
        list.add_css_class("launcher-list");

        let flow = gtk4::FlowBox::new();
        flow.set_min_children_per_line(GRID_COLUMNS);
        flow.set_max_children_per_line(GRID_COLUMNS);
        flow.set_column_spacing(2);
        flow.set_row_spacing(2);
        flow.set_homogeneous(true);
        flow.set_selection_mode(gtk4::SelectionMode::None);
        flow.add_css_class("launcher-grid");

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_height_request(RESULTS_HEIGHT);
        scroll.set_vexpand(true);

        let results_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        results_box.add_css_class("launcher-results");
        results_box.set_hexpand(true);
        results_box.append(&list);
        results_box.append(&flow);

        let viewport = gtk4::Viewport::new(None::<&gtk4::Adjustment>, None::<&gtk4::Adjustment>);
        viewport.set_child(Some(&results_box));

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_height_request(RESULTS_HEIGHT);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&viewport));

        let results_revealer = gtk4::Revealer::new();
        results_revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        results_revealer.set_transition_duration(180);
        results_revealer.set_child(Some(&scroll));

        let panel = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        panel.set_css_classes(&["launcher-panel"]);
        panel.set_width_request(LAUNCHER_WIDTH);
        panel.set_halign(gtk4::Align::Center);
        panel.set_valign(gtk4::Align::Center);
        panel.append(&search_bar);
        panel.append(&pinned_revealer);
        panel.append(&results_revealer);

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&backdrop));
        overlay.add_overlay(&panel);
        root.set_child(Some(&overlay));
        root.present();

        let window = LauncherWindow {
            root: root.clone(),
            entry,
            clear,
            layout_toggle,
            pinned,
            pin_hint_icon,
            pin_hint_label,
            pinned_revealer,
            results_revealer,
            results_box,
            list,
            flow,
            scroll,
            viewport,
            popover_open: std::rc::Rc::new(std::cell::Cell::new(false)),
        };
        keys::wire(&window, sender);
        window
    }

    pub fn show(&self) {
        self.root.set_visible(true);
        self.root.present();
        self.entry.set_text("");
        self.entry.grab_focus();
    }

    pub fn hide(&self) {
        self.root.set_visible(false);
    }
}
