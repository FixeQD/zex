//! One notification row: icon, summary, age, body, actions and close.
//!
//! Used by the popup overlay (compact) and the quick center (full variant).

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_services::notifications::relative_age;
use zex_services::notifications::{Notification, NotificationClient};

use crate::bar::widgets::icon::{app_icon, has_icon};
use crate::m3::{M3Button, M3Shape, M3Size, M3Type};

fn icon_name(notification: &Notification) -> String {
    if has_icon(&notification.app_icon) {
        return notification.app_icon.clone();
    }
    let looked_up = app_icon(&notification.app_name);
    if has_icon(&looked_up) {
        return looked_up;
    }
    "dialog-information-symbolic".to_string()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub struct NotificationWidget {
    pub root: gtk4::Box,
    pub id: u32,
    age_label: gtk4::Label,
    timestamp: i64,
}

impl NotificationWidget {
    pub fn new(notification: &Notification, compact: bool, client: NotificationClient) -> Self {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_halign(gtk4::Align::Fill);
        root.add_css_class("notification");
        if compact {
            root.add_css_class("compact-popup");
        }

        let icon = gtk4::Image::new();
        icon.set_icon_name(Some(&icon_name(notification)));
        icon.set_pixel_size(if compact { 16 } else { 24 });
        icon.set_halign(gtk4::Align::Start);
        icon.set_valign(gtk4::Align::Start);
        icon.add_css_class("notification-icon");

        let summary = gtk4::Label::new(Some(&notification.summary));
        summary.set_halign(gtk4::Align::Start);
        summary.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        summary.set_css_classes(&["notification-summary"]);

        let age_label = gtk4::Label::new(None);
        age_label.set_halign(gtk4::Align::Start);
        age_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        age_label.set_css_classes(&["notification-age"]);
        age_label.set_visible(!compact);

        let body = if notification.body.is_empty() {
            None
        } else {
            let label = gtk4::Label::new(Some(&notification.body));
            label.set_halign(gtk4::Align::Start);
            label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            label.set_wrap(false);
            label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            label.set_css_classes(&["notification-body"]);
            label.set_single_line_mode(compact);
            Some(label)
        };

        let id = notification.id;
        let close = M3Button::new(
            Some("window-close-symbolic"),
            None,
            M3Type::Text,
            M3Size::Xs,
            M3Shape::Round,
        );
        close.button.add_css_class("notification-close");
        close.button.set_valign(if compact {
            gtk4::Align::Center
        } else {
            gtk4::Align::Start
        });
        let client_close = client.clone();
        close.connect_clicked(move |_| client_close.close(id));

        // Expand/collapse mutates the body label in place; state is per-row
        let expand = if body.is_some() && !compact {
            let button = M3Button::new(
                Some("go-down-symbolic"),
                None,
                M3Type::Text,
                M3Size::Xs,
                M3Shape::Round,
            );
            button.button.add_css_class("notification-expand");
            button.button.set_valign(gtk4::Align::Start);
            button.button.set_vexpand(false);
            let body = body.clone().expect("expand requires a body");
            let icon = button.icon.clone();
            let expanded = Rc::new(Cell::new(false));
            let toggle_expanded = Rc::clone(&expanded);
            button.connect_clicked(move |_| {
                let now = !toggle_expanded.get();
                toggle_expanded.set(now);
                body.set_ellipsize(if now {
                    gtk4::pango::EllipsizeMode::None
                } else {
                    gtk4::pango::EllipsizeMode::End
                });
                body.set_wrap(now);
                icon.set_icon_name(Some(if now {
                    "go-up-symbolic"
                } else {
                    "go-down-symbolic"
                }));
            });
            Some(button)
        } else {
            None
        };

        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        row.set_halign(gtk4::Align::Fill);
        row.append(&icon);

        let info = gtk4::Box::new(
            if compact {
                gtk4::Orientation::Horizontal
            } else {
                gtk4::Orientation::Vertical
            },
            if compact { 8 } else { 2 },
        );
        info.set_valign(gtk4::Align::Center);
        info.set_hexpand(true);
        info.set_halign(gtk4::Align::Fill);
        info.add_css_class("notification-info");

        if compact {
            info.append(&summary);
            if let Some(body) = body.as_ref() {
                info.append(body);
            }
        } else {
            let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
            header.set_halign(gtk4::Align::Start);
            let separator = gtk4::Label::new(Some("\u{2022}"));
            separator.set_halign(gtk4::Align::Start);
            separator.set_css_classes(&["notification-separator"]);
            header.append(&summary);
            header.append(&separator);
            header.append(&age_label);
            info.append(&header);
            if let Some(body) = body.as_ref() {
                info.append(body);
            }
            if !notification.actions.is_empty() {
                let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
                actions.set_halign(gtk4::Align::Start);
                actions.add_css_class("notification-actions-container");
                for action in &notification.actions {
                    let action_key = action.key.clone();
                    let action_label = action.label.clone();
                    let client_action = client.clone();
                    let button = M3Button::new(
                        None,
                        Some(&action_label),
                        M3Type::Text,
                        M3Size::Xs,
                        M3Shape::Round,
                    );
                    button.button.add_css_class("notification-action");
                    button.connect_clicked(move |_| {
                        client_action.invoke_action(id, &action_key);
                    });
                    actions.append(&button.button);
                }
                info.append(&actions);
            }
        }
        row.append(&info);

        let controls = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
        controls.append(&close.button);
        if let Some(expand) = expand.as_ref() {
            controls.append(&expand.button);
        }
        row.append(&controls);

        root.append(&row);

        let widget = Self {
            root,
            id,
            age_label,
            timestamp: notification.time,
        };
        widget.update_age();
        widget
    }

    /// Refresh the relative age label; called on each `AgeTick`
    pub fn update_age(&self) {
        self.age_label
            .set_label(&relative_age(now_secs(), self.timestamp));
    }
}