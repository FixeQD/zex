//! Palette swatch button for one material preview

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_core::theme::matugen::Preview;

/// One entry of a palette grid: (scheme, dark, swatch button)
pub type PaletteEntry = (String, bool, gtk4::Button);
/// Shared registry of swatch buttons built by a palette grid
pub type PaletteRegistry = Rc<RefCell<Vec<PaletteEntry>>>;

pub fn palette_button(index: usize, preview: &Preview, selected: bool) -> gtk4::Button {
    let swatch = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    swatch.add_css_class("preview");
    swatch.add_css_class(&format!("pv-{index}"));
    swatch.set_size_request(50, 50);
    swatch.set_overflow(gtk4::Overflow::Hidden);

    let primary = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    primary.add_css_class("primary");
    primary.set_size_request(50, 25);
    primary.set_halign(gtk4::Align::Start);
    swatch.append(&primary);

    let bottom = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let secondary = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    secondary.add_css_class("secondary");
    secondary.set_size_request(25, 25);
    bottom.append(&secondary);
    let tertiary = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    tertiary.add_css_class("tertiary");
    tertiary.set_size_request(25, 25);
    bottom.append(&tertiary);
    swatch.append(&bottom);

    let button = gtk4::Button::new();
    button.add_css_class("palette-preview-btn");
    button.set_hexpand(true);
    button.set_halign(gtk4::Align::Fill);
    button.set_tooltip_text(Some(&preview.scheme));
    button.set_child(Some(&swatch));
    if selected {
        button.add_css_class("selected");
    }
    button
}
