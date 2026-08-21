//! Bar style computation integration tests

#![allow(clippy::field_reassign_with_default)]

use zex_core::settings::{Bar, Bar2};
use zex_shell::bar::styles::{BarLike, BarStyle, compute};

fn style_of(bar: &dyn BarLike) -> BarStyle {
    compute(bar)
}

#[test]
fn default_bar_style() {
    let style = style_of(&Bar::default());
    assert_eq!(
        style.css_classes,
        [
            "bar",
            "hug",
            "extrapadding",
            "full",
            "horizontal",
            "module-backgrounds",
            "bar-background",
            "bottom",
        ]
    );
    assert_eq!(style.thickness, 40);
    assert_eq!(style.margins, [0, 0, 0, 0]);
    assert_eq!(style.anchors, [false, true, true, true]);
}

#[test]
fn floating_adds_margins_without_side_gap() {
    let mut bar = Bar::default();
    bar.floating = true;
    let style = style_of(&bar);
    assert!(style.css_classes.contains(&"floating"));
    assert_eq!(style.margins, [0, 5, 5, 5]);
    assert!(!style.css_classes.contains(&"hug"));

    bar.side = "top".into();
    let style = style_of(&bar);
    assert_eq!(style.margins, [5, 5, 5, 0]);
    assert_eq!(style.anchors, [true, true, true, false]);
}

#[test]
fn vertical_side_anchors_three_edges() {
    let mut bar = Bar::default();
    bar.side = "right".into();
    bar.vertical = true;
    let style = style_of(&bar);
    assert_eq!(style.anchors, [true, false, true, true]);
    assert!(style.css_classes.contains(&"vertical"));
    assert!(style.css_classes.contains(&"right"));
}

#[test]
fn centered_anchors_only_the_side() {
    let mut bar = Bar::default();
    bar.centered = true;
    let style = style_of(&bar);
    assert_eq!(style.anchors, [false, false, false, true]);
    assert!(style.css_classes.contains(&"round"));
    assert!(!style.css_classes.contains(&"extrapadding"));
}

#[test]
fn density_maps_to_thickness_and_class() {
    let mut bar = Bar::default();
    bar.density = -1;
    assert_eq!(style_of(&bar).thickness, 35);
    assert!(style_of(&bar).css_classes.contains(&"compact"));

    bar.density = -2;
    assert_eq!(style_of(&bar).thickness, 30);
    assert!(style_of(&bar).css_classes.contains(&"compact-plus"));

    bar.density = -3;
    assert_eq!(style_of(&bar).thickness, 25);
    assert!(style_of(&bar).css_classes.contains(&"ultracompact"));

    bar.density = 4;
    assert_eq!(style_of(&bar).thickness, 40);
}

#[test]
fn separation_and_background_flags_map_to_classes() {
    let mut bar = Bar::default();
    bar.separation = true;
    bar.bar_background = false;
    bar.module_backgrounds = false;
    let style = style_of(&bar);
    assert!(style.css_classes.contains(&"separated"));
    assert!(!style.css_classes.contains(&"full"));
    assert!(!style.css_classes.contains(&"bar-background"));
    assert!(!style.css_classes.contains(&"module-backgrounds"));
}

#[test]
fn bar2_uses_same_style_rules() {
    let bar2 = Bar2::default();
    let style = style_of(&bar2);
    assert_eq!(style.thickness, 40);
    assert!(style.css_classes.contains(&"top"));
    assert_eq!(style.margins, [0, 0, 0, 0]);
}
