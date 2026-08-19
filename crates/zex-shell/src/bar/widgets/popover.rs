//! Shared popover building block for dock and tray menus

use std::rc::Rc;

use gtk4::prelude::*;

/// One popover entry; menus are rebuilt from these on every open
pub enum PopoverItem {
    Label(String),
    Action(String, Rc<dyn Fn()>),
    Separator,
}

/// Attach a popover to the anchor and show it
pub fn show_popover(anchor: &gtk4::Widget, items: Vec<PopoverItem>) {
    let popover = gtk4::Popover::new();
    popover.set_css_classes(&["dock-menu"]);

    let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    box_.set_css_classes(&["dock-menu-box"]);
    for item in items {
        match item {
            PopoverItem::Label(label) => {
                let label = gtk4::Label::new(Some(&label));
                label.set_css_classes(&["dock-menu-label", "dim-label"]);
                label.set_halign(gtk4::Align::Start);
                box_.append(&label);
            }
            PopoverItem::Action(label, action) => {
                let button = gtk4::Button::with_label(&label);
                button.set_css_classes(&["dock-menu-button"]);
                button.connect_clicked(move |_| action());
                box_.append(&button);
            }
            PopoverItem::Separator => {
                box_.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            }
        }
    }
    popover.set_child(Some(&box_));
    popover.set_parent(anchor);
    popover.popup();
}
