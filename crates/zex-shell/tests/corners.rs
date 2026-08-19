//! Corner warp geometry tests

use gtk4_layer_shell::{Edge, Layer};
use zex_core::settings::Settings;
use zex_shell::corners::{CornerKind, corner_specs, warp_size};

fn base_settings() -> Settings {
    let mut settings = Settings::default();
    settings.interface.bar2.enabled = false;
    settings.interface.misc.shell_corners = false;
    settings.interface.misc.screen_corners = "disabled".to_string();
    settings
}

#[test]
fn warp_size_matches_density() {
    assert_eq!(warp_size(0), 25);
    assert_eq!(warp_size(1), 23);
    assert_eq!(warp_size(2), 20);
    assert_eq!(warp_size(3), 18);
    assert_eq!(warp_size(9), 25);
}

#[test]
fn disabled_screen_corners_produce_nothing() {
    let settings = base_settings();
    assert!(corner_specs(&settings).is_empty());
}

#[test]
fn default_screen_corners_cover_all_four_corners() {
    let mut settings = base_settings();
    settings.interface.misc.screen_corners = "not_fullscreen".to_string();
    let specs = corner_specs(&settings);
    assert_eq!(specs.len(), 4);
    for spec in &specs {
        assert_eq!(spec.kind, CornerKind::Screen);
        assert_eq!(spec.layer, Layer::Top);
        assert_eq!(spec.size, 25);
    }
    assert!(specs.iter().any(|s| s.edges == (Edge::Top, Edge::Left)));
}

#[test]
fn always_screen_corners_use_overlay_layer() {
    let mut settings = base_settings();
    settings.interface.misc.screen_corners = "always".to_string();
    let specs = corner_specs(&settings);
    assert!(specs.iter().all(|s| s.layer == Layer::Overlay));
}

#[test]
fn floating_centered_bar_sizes_all_corners_default() {
    let mut settings = base_settings();
    let bar = &mut settings.interface.bar;
    bar.floating = true;
    bar.centered = true;
    bar.density = -2;
    let specs = corner_specs(&settings);
    assert!(specs.iter().all(|s| s.size == 25));
}

#[test]
fn floating_bar_uses_density_size_on_neighbouring_corners() {
    let mut settings = base_settings();
    let bar = &mut settings.interface.bar;
    bar.side = "bottom".to_string();
    bar.floating = true;
    bar.density = -3;
    let specs = corner_specs(&settings);
    let mut bottom = specs
        .iter()
        .filter(|s| s.edges.0 == Edge::Bottom || s.edges.1 == Edge::Bottom);
    let mut top = specs
        .iter()
        .filter(|s| s.edges.0 == Edge::Top || s.edges.1 == Edge::Top);
    assert!(bottom.all(|s| s.size == warp_size(3)));
    assert!(top.all(|s| s.size == 25));
}

#[test]
fn shell_corners_follow_hugging_bars() {
    let mut settings = base_settings();
    settings.interface.misc.shell_corners = true;
    settings.interface.bar.side = "bottom".to_string();
    let specs = corner_specs(&settings);
    let bar_corners: Vec<_> = specs.iter().filter(|s| s.kind == CornerKind::Bar).collect();
    assert_eq!(bar_corners.len(), 2);
    assert!(
        bar_corners
            .iter()
            .all(|s| s.edges.0 == Edge::Bottom || s.edges.1 == Edge::Bottom)
    );
    assert!(bar_corners.iter().all(|s| s.edges.0 == Edge::Left
        || s.edges.1 == Edge::Left
        || s.edges.0 == Edge::Right
        || s.edges.1 == Edge::Right));
}

#[test]
fn vertical_bar_caps_its_own_ends() {
    let mut settings = base_settings();
    settings.interface.misc.shell_corners = true;
    let bar = &mut settings.interface.bar;
    bar.side = "right".to_string();
    bar.vertical = true;
    let specs = corner_specs(&settings);
    let bar_corners: Vec<_> = specs.iter().filter(|s| s.kind == CornerKind::Bar).collect();
    assert_eq!(bar_corners.len(), 2);
    assert!(
        bar_corners
            .iter()
            .all(|s| s.edges.0 == Edge::Right || s.edges.1 == Edge::Right)
    );
    assert!(
        bar_corners
            .iter()
            .any(|s| s.edges.0 == Edge::Top || s.edges.1 == Edge::Top)
    );
    assert!(
        bar_corners
            .iter()
            .any(|s| s.edges.0 == Edge::Bottom || s.edges.1 == Edge::Bottom)
    );
}

#[test]
fn floating_or_centered_bars_leave_no_bar_corners() {
    let mut settings = base_settings();
    settings.interface.misc.shell_corners = true;
    settings.interface.bar.floating = true;
    assert!(
        corner_specs(&settings)
            .iter()
            .all(|s| s.kind == CornerKind::Screen)
    );

    let mut settings = base_settings();
    settings.interface.misc.shell_corners = true;
    settings.interface.bar.centered = true;
    assert!(
        corner_specs(&settings)
            .iter()
            .all(|s| s.kind == CornerKind::Screen)
    );
}

#[test]
fn bars_sharing_an_edge_deduplicate() {
    let mut settings = base_settings();
    settings.interface.misc.shell_corners = true;
    settings.interface.bar.side = "top".to_string();
    settings.interface.bar2.enabled = true;
    settings.interface.bar2.side = "top".to_string();
    let specs = corner_specs(&settings);
    let bar_corners: Vec<_> = specs.iter().filter(|s| s.kind == CornerKind::Bar).collect();
    assert_eq!(bar_corners.len(), 2);
}

#[test]
fn disabled_bar2_adds_no_corners_of_its_own() {
    let mut settings = base_settings();
    settings.interface.misc.shell_corners = true;
    settings.interface.bar.side = "top".to_string();
    settings.interface.bar2.enabled = false;
    settings.interface.bar2.side = "bottom".to_string();
    let specs = corner_specs(&settings);
    let bar_corners: Vec<_> = specs.iter().filter(|s| s.kind == CornerKind::Bar).collect();
    assert_eq!(bar_corners.len(), 2);
    assert!(
        bar_corners
            .iter()
            .all(|s| s.edges.0 == Edge::Top || s.edges.1 == Edge::Top)
    );
}
