//! The settings window: header strip, navigation rail and scrolled conten.

use gtk4::prelude::*;
use relm4::Sender;

use super::SettingsMsg;
use crate::m3::{M3Button, M3Shape, M3Size, M3Type, NavigationRail};

/// The five settings tabs in rail order: (key, label, rail icon).
pub const TABS: [(&str, &str, &str); 5] = [
    ("quick", "Quick", "view-grid-symbolic"),
    (
        "appearance",
        "Appearance",
        "preferences-desktop-wallpaper-symbolic",
    ),
    ("interface", "Interface", "preferences-system-symbolic"),
    ("services", "Services", "preferences-other-symbolic"),
    ("about", "About", "help-about-symbolic"),
];

pub fn tab_label(key: &str) -> String {
    TABS.iter()
        .find(|tab| tab.0 == key)
        .map(|tab| tab.1.to_string())
        .unwrap_or_else(|| key.to_string())
}

pub struct SettingsWindow {
    pub root: gtk4::Window,
    pub rail: NavigationRail,
    pub header_title: gtk4::Label,
    pub scroll: gtk4::ScrolledWindow,
}

impl SettingsWindow {
    pub fn new(sender: Sender<SettingsMsg>) -> Self {
        let root = gtk4::Window::new();
        root.set_title(Some("Zex Settings"));
        root.set_default_size(1200, 900);
        root.set_hide_on_close(true);
        root.add_css_class("settings-window");
        root.set_visible(false);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        header.add_css_class("header-bar");
        header.set_halign(gtk4::Align::Fill);
        header.set_hexpand(true);

        let icon = gtk4::Image::from_icon_name("preferences-system-symbolic");
        icon.add_css_class("header-title-icon");
        header.append(&icon);

        let title = gtk4::Label::new(Some("Zex Settings"));
        title.add_css_class("header-title");
        header.append(&title);

        let crumb = gtk4::Label::new(Some(">"));
        crumb.add_css_class("header-crumb");
        header.append(&crumb);

        let active = gtk4::Label::new(Some("Quick"));
        active.add_css_class("header-active");
        header.append(&active);

        let rail = NavigationRail::new();
        let select_sender = sender.clone();
        rail.set_on_select(move |key| {
            let _ = select_sender.send(SettingsMsg::TabSelected(key.into()));
        });
        for (key, label, icon) in TABS {
            rail.add_item(key, icon, label);
        }

        let rail_frame = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        rail_frame.add_css_class("rail-frame");
        rail_frame.append(&rail.container);

        let reload = M3Button::new(
            Some("view-refresh-symbolic"),
            Some("Reload"),
            M3Type::Text,
            M3Size::S,
            M3Shape::Round,
        );
        reload.set_vertical(true);
        reload.button.add_css_class("rail-button");
        reload.button.set_halign(gtk4::Align::Fill);
        reload.button.set_margin_top(8);
        let reload_sender = sender.clone();
        reload.button.connect_clicked(move |_| {
            let _ = reload_sender.send(SettingsMsg::Reload);
        });
        rail_frame.append(&reload.button);

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.set_overlay_scrolling(true);

        let main = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        main.set_hexpand(true);
        main.set_vexpand(true);
        main.append(&rail_frame);
        main.append(&scroll);

        let layout = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        layout.append(&header);
        layout.append(&main);
        root.set_child(Some(&layout));

        Self {
            root,
            rail,
            header_title: active,
            scroll,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.root.is_visible()
    }

    pub fn present(&self) {
        self.root.present();
    }
}
