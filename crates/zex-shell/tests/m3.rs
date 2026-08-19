//! M3 widget component tests

use gtk4::prelude::*;
use zex_shell::m3::{
    ConnectedButtonGroup, M3Button, M3Shape, M3Size, M3Slider, M3Type, NavigationRail,
};

fn init_gtk() {
    gtk4::init().expect("gtk initialization failed");
}

fn classes(widget: &impl gtk4::prelude::WidgetExt) -> Vec<String> {
    widget
        .css_classes()
        .into_iter()
        .map(|c| c.to_string())
        .collect()
}

#[test]
fn stylesheet_compiles_with_grass() {
    let css = zex_core::theme::css::compile(zex_shell::m3::M3_CSS_SCSS).expect("m3.scss compiles");
    assert!(!css.is_empty());
}

#[gtk4::test]
fn button_applies_type_size_shape_classes() {
    init_gtk();
    let button = M3Button::new(
        Some("edit-symbolic"),
        Some("Label"),
        M3Type::Filled,
        M3Size::M,
        M3Shape::Square,
    );
    let classes = classes(&button.button);
    for expected in ["m3-button", "filled", "m", "square"] {
        assert!(classes.iter().any(|c| c == expected), "missing {expected}");
    }
    assert!(!classes.iter().any(|c| c == "icon-only"));
}

#[gtk4::test]
fn icon_only_class_follows_visibility() {
    init_gtk();
    let button = M3Button::new(
        Some("edit-symbolic"),
        None,
        M3Type::Tonal,
        M3Size::S,
        M3Shape::Round,
    );
    assert!(classes(&button.button).iter().any(|c| c == "icon-only"));

    button.set_label(Some("Now labeled"));
    assert!(!classes(&button.button).iter().any(|c| c == "icon-only"));

    button.set_icon(None);
    assert!(!classes(&button.button).iter().any(|c| c == "icon-only"));
}

#[gtk4::test]
fn connected_group_toggles_active_segment() {
    init_gtk();
    let group = ConnectedButtonGroup::new();
    let first = M3Button::new(
        None,
        Some("First"),
        M3Type::Tonal,
        M3Size::S,
        M3Shape::Square,
    );
    let second = M3Button::new(
        None,
        Some("Second"),
        M3Type::Tonal,
        M3Size::S,
        M3Shape::Square,
    );
    group.add(&first);
    group.add(&second);

    assert_eq!(
        group
            .container
            .first_child()
            .as_ref()
            .map(|w| w.downcast_ref::<gtk4::Button>()),
        Some(Some(&first.button))
    );
    first.set_active(true);
    assert!(classes(&first.button).iter().any(|c| c == "active"));
}

#[gtk4::test]
fn slider_reports_value_and_range() {
    init_gtk();
    let slider = M3Slider::new(Some("edit-symbolic"));
    slider.set_range(0.0, 100.0);
    slider.set_value(33.5);
    assert_eq!(slider.value(), 33.5);

    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let _handler = slider.connect_value_changed({
        let seen = std::rc::Rc::clone(&seen);
        move |value| seen.borrow_mut().push(value)
    });
    slider.set_value(55.0);
    assert!(!seen.borrow().is_empty());
}

fn rail_buttons(rail: &NavigationRail) -> Vec<gtk4::Button> {
    let mut buttons = Vec::new();
    let mut child = rail.container.first_child();
    while let Some(child_widget) = child {
        buttons.push(
            child_widget
                .clone()
                .downcast::<gtk4::Button>()
                .expect("rail child is a button"),
        );
        child = child_widget.next_sibling();
    }
    buttons
}

#[gtk4::test]
fn rail_selects_one_item_at_a_time() {
    init_gtk();
    let rail = NavigationRail::new();
    let selected = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    rail.set_on_select({
        let selected = std::rc::Rc::clone(&selected);
        move |key| selected.borrow_mut().push(key.to_string())
    });
    rail.add_item("first", "edit-symbolic", "First");
    rail.add_item("second", "edit-symbolic", "Second");
    rail.select("second");

    let buttons = rail_buttons(&rail);
    assert_eq!(buttons.len(), 2);
    assert!(classes(&buttons[1]).iter().any(|c| c == "selected"));
    assert!(!classes(&buttons[0]).iter().any(|c| c == "selected"));
    assert_eq!(*selected.borrow(), vec!["second".to_string()]);

    rail.select("first");
    assert!(classes(&buttons[0]).iter().any(|c| c == "selected"));
    assert!(!classes(&buttons[1]).iter().any(|c| c == "selected"));
    assert_eq!(
        *selected.borrow(),
        vec!["second".to_string(), "first".to_string()]
    );
}
