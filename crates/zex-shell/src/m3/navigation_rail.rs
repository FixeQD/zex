//! M3 navigation rail: vertical icon+label items with a single selected state.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use super::button::{M3Button, M3Shape, M3Size, M3Type};

type Buttons = Rc<RefCell<Vec<(String, gtk4::Button)>>>;
type OnSelect = Rc<RefCell<Option<Box<dyn FnMut(&str)>>>>;

#[derive(Clone)]
pub struct NavigationRail {
    pub container: gtk4::Box,
    buttons: Buttons,
    on_select: OnSelect,
}

impl NavigationRail {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
        container.set_halign(gtk4::Align::Start);
        container.add_css_class("navigation-rail");
        Self {
            container,
            buttons: Rc::new(RefCell::new(Vec::new())),
            on_select: Rc::new(RefCell::new(None)),
        }
    }

    /// Append an item; clicking it selects it and fires [`NavigationRail::set_on_select`]
    pub fn add_item(&self, key: impl Into<String>, icon: &str, label: &str) {
        let key: String = key.into();
        let button = M3Button::new(
            Some(icon),
            Some(label),
            M3Type::Text,
            M3Size::S,
            M3Shape::Round,
        );
        button.button.add_css_class("rail-button");
        button.set_vertical(true);

        let buttons = Rc::clone(&self.buttons);
        let on_select = Rc::clone(&self.on_select);
        let selected_key = key.clone();
        button.connect_clicked(move |_| dispatch(&buttons, &on_select, &selected_key));

        self.buttons.borrow_mut().push((key, button.button.clone()));
        self.container.append(&button.button);
    }

    /// Callback invoked when an item becomes selected
    pub fn set_on_select(&self, f: impl FnMut(&str) + 'static) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    /// Mark `key` as selected, clearing the previous one
    pub fn select(&self, key: &str) {
        dispatch(&self.buttons, &self.on_select, key);
    }
}

impl Default for NavigationRail {
    fn default() -> Self {
        Self::new()
    }
}

fn dispatch(buttons: &Buttons, on_select: &OnSelect, key: &str) {
    let mut found = false;
    for (name, button) in buttons.borrow().iter() {
        if name == key {
            button.add_css_class("selected");
            found = true;
        } else {
            button.remove_css_class("selected");
        }
    }
    if !found {
        tracing::warn!("no rail item named {key}");
        return;
    }
    if let Some(callback) = on_select.borrow_mut().as_mut() {
        callback(key);
    }
}
