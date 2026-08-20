//! Segmented control groups: single-select toggles and independent toggles

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_core::Settings;

use crate::m3::{ConnectedButtonGroup, M3Button, M3Shape, M3Size, M3Type};
use crate::settings::tabs::TabContext;

pub struct ToggleItem<V> {
    pub label: Option<&'static str>,
    pub icon: Option<&'static str>,
    pub value: V,
}

type ToggleRegistry<V> = Rc<RefCell<Vec<(gtk4::Button, V)>>>;

pub fn toggle_buttons<V: PartialEq + Clone + 'static>(
    ctx: &TabContext,
    items: Vec<ToggleItem<V>>,
    get: impl Fn(&Settings) -> V + 'static,
    set: impl Fn(&mut Settings, V) + 'static,
) -> ConnectedButtonGroup {
    let group = ConnectedButtonGroup::new();
    let store = Rc::clone(&ctx.store);
    let get = Rc::new(get);
    let set = Rc::new(set);
    let registry: ToggleRegistry<V> = Rc::new(RefCell::new(Vec::new()));

    for item in items {
        let button = M3Button::new(
            item.icon,
            item.label,
            M3Type::Tonal,
            M3Size::Xs,
            M3Shape::Square,
        );
        button.button.set_hexpand(true);
        button.button.set_halign(gtk4::Align::Fill);
        group.add(&button);

        let value = item.value.clone();
        registry
            .borrow_mut()
            .push((button.button.clone(), item.value));

        let registry = Rc::clone(&registry);
        let store = Rc::clone(&store);
        let set = Rc::clone(&set);
        let get = Rc::clone(&get);
        button.connect_clicked(move |_| {
            if let Err(err) = store.borrow_mut().update(|s| set(s, value.clone())) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
            sync_toggle_group(&registry, &store, &*get);
        });
    }

    sync_toggle_group(&registry, &store, &*get);
    group
}

fn sync_toggle_group<V: PartialEq>(
    registry: &ToggleRegistry<V>,
    store: &Rc<RefCell<zex_core::SettingsStore>>,
    get: impl Fn(&Settings) -> V,
) {
    let current = get(&store.borrow().get().clone());
    for (button, value) in registry.borrow().iter() {
        if *value == current {
            button.add_css_class("active");
            button.add_css_class("filled");
        } else {
            button.remove_css_class("active");
            button.remove_css_class("filled");
        }
    }
}

type Getter = Box<dyn Fn(&Settings) -> bool>;
type Setter = Box<dyn Fn(&mut Settings, bool)>;

/// One option of a multi-select (independent) toggle group.
pub struct IndependentItem {
    pub label: Option<&'static str>,
    pub icon: Option<&'static str>,
    pub get: Getter,
    pub set: Setter,
}

type IndependentRegistry = Rc<RefCell<Vec<(gtk4::Button, Rc<dyn Fn(&Settings) -> bool>)>>>;

pub fn independent_toggle_buttons(
    ctx: &TabContext,
    items: Vec<IndependentItem>,
) -> ConnectedButtonGroup {
    let group = ConnectedButtonGroup::new();
    let store = Rc::clone(&ctx.store);
    let registry: IndependentRegistry = Rc::new(RefCell::new(Vec::new()));

    for item in items {
        let button = M3Button::new(
            item.icon,
            item.label,
            M3Type::Tonal,
            M3Size::Xs,
            M3Shape::Square,
        );
        button.button.set_hexpand(true);
        button.button.set_halign(gtk4::Align::Fill);
        group.add(&button);

        let get: Rc<dyn Fn(&Settings) -> bool> = Rc::from(item.get);
        registry.borrow_mut().push((button.button.clone(), get));

        let registry = Rc::clone(&registry);
        let store = Rc::clone(&store);
        let set = item.set;
        let button_handle = button.button.clone();
        button.connect_clicked(move |_| {
            let value = registry
                .borrow()
                .iter()
                .rev()
                .find(|(b, _)| b == &button_handle)
                .map(|(_, get)| get(&store.borrow().get().clone()))
                .unwrap_or(false);
            let next = !value;
            if let Err(err) = store.borrow_mut().update(|s| set(s, next)) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
            sync_independent_group(&registry, &store);
        });
    }

    sync_independent_group(&registry, &store);
    group
}

fn sync_independent_group(
    registry: &IndependentRegistry,
    store: &Rc<RefCell<zex_core::SettingsStore>>,
) {
    let snapshot = store.borrow().get().clone();
    for (button, get) in registry.borrow().iter() {
        if get(&snapshot) {
            button.add_css_class("active");
            button.add_css_class("filled");
        } else {
            button.remove_css_class("active");
            button.remove_css_class("filled");
        }
    }
}
