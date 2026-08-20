//! Settings window tab builders

pub mod about;
pub mod appearance;
pub mod interface;
pub mod quick;
pub mod services;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use relm4::ComponentSender;
use zex_core::SettingsStore;
use zex_core::theme::matugen::Preview;

use super::Settings;

pub struct TabContext {
    pub store: Rc<RefCell<SettingsStore>>,
    pub sender: ComponentSender<Settings>,
    pub previews: Vec<Preview>,
    pub gallery: Vec<String>,
    pub window: gtk4::Window,
}

impl TabContext {
    pub fn snapshot(&self) -> zex_core::Settings {
        self.store.borrow().get().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut zex_core::Settings)) {
        if let Err(err) = self.store.borrow_mut().update(f) {
            tracing::warn!("settings persistence failed: {err:#}");
        }
    }
}

/// Build the widget for tab `key`.
pub fn build_tab(key: &str, ctx: &TabContext) -> gtk4::Widget {
    match key {
        "quick" => quick::build(ctx).upcast(),
        "appearance" => appearance::build(ctx).upcast(),
        "interface" => interface::build(ctx).upcast(),
        "services" => services::build(ctx).upcast(),
        "about" => about::build(ctx).upcast(),
        other => {
            tracing::warn!("unknown settings tab \"{other}\"");
            quick::build(ctx).upcast()
        }
    }
}
