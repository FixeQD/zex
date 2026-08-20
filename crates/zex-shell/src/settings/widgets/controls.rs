//! Rows that mount a control: switches, spin buttons and spin rows

use std::rc::Rc;

use gtk4::prelude::*;
use zex_core::Settings;

use super::layout::settings_row;
use crate::settings::tabs::TabContext;

pub fn switch_row(
    ctx: &TabContext,
    title: Option<&str>,
    description: Option<&str>,
    active: bool,
    set: impl Fn(&mut Settings, bool) + 'static,
) -> gtk4::Button {
    let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
    content.add_css_class("switch-row");

    let header = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    header.add_css_class("settings-row-header");
    header.set_valign(gtk4::Align::Center);
    header.set_halign(gtk4::Align::Start);
    header.set_hexpand(true);
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
    content.append(&header);

    let switch = gtk4::Switch::new();
    switch.set_active(active);
    switch.set_valign(gtk4::Align::Center);
    content.append(&switch);

    let button = gtk4::Button::new();
    button.add_css_class("settings-row");
    button.add_css_class("row-button");
    button.set_halign(gtk4::Align::Fill);
    button.set_hexpand(true);
    button.set_child(Some(&content));

    let store = Rc::clone(&ctx.store);
    button.connect_clicked(move |_| {
        let next = !switch.is_active();
        if let Err(err) = store.borrow_mut().update(|s| set(s, next)) {
            tracing::warn!("settings persistence failed: {err:#}");
        } else {
            switch.set_active(next);
        }
    });
    button
}

/// A numeric editor bound to a settings value.
pub fn spin_button(
    ctx: &TabContext,
    min: i32,
    max: i32,
    value: i32,
    set: impl Fn(&mut Settings, i32) + 'static,
) -> gtk4::SpinButton {
    let value = value.clamp(min, max);
    let adjustment = gtk4::Adjustment::new(value as f64, min as f64, max as f64, 1.0, 10.0, 0.0);
    let spin = gtk4::SpinButton::new(Some(&adjustment), 0.0, 0);
    spin.set_numeric(true);
    spin.set_update_policy(gtk4::SpinButtonUpdatePolicy::IfValid);
    spin.set_valign(gtk4::Align::Center);

    let store = Rc::clone(&ctx.store);
    spin.connect_value_changed(move |spin| {
        let value = spin.value().round() as i32;
        if let Err(err) = store.borrow_mut().update(|s| set(s, value)) {
            tracing::warn!("settings persistence failed: {err:#}");
        }
    });
    spin
}

pub fn spin_row(
    ctx: &TabContext,
    title: Option<&str>,
    description: Option<&str>,
    min: i32,
    max: i32,
    value: i32,
    set: impl Fn(&mut Settings, i32) + 'static,
) -> gtk4::Box {
    let row = settings_row(title, description, false);
    row.append(&spin_button(ctx, min, max, value, set));
    row
}
