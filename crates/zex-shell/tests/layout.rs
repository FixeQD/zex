//! Layout engine integration tests

use zex_core::settings::Modules;
use zex_shell::bar::layout::{Area, Layout, Module};

#[test]
fn default_layout_matches_reference_order() {
    let layout = Layout::new(&Modules::default());
    let placements = layout.placements().to_vec();

    let expected = [
        (Module::Launcher, Area::Left),
        (Module::WindowInfo, Area::Left),
        (Module::Media, Area::Left),
        (Module::Workspaces, Area::Center),
        (Module::Tasks, Area::Center),
        (Module::RecordingIndicator, Area::Right),
        (Module::SystemInfoTray, Area::Right),
        (Module::Clock, Area::Right),
    ];
    assert_eq!(placements.len(), expected.len());
    for (placement, (module, area)) in placements.iter().zip(expected) {
        assert_eq!(placement.module, module);
        assert_eq!(placement.area, area);
        assert_eq!(placement.bar_id, 0);
    }
}

#[test]
fn bar2_is_auto_disabled_with_empty_layout() {
    let layout = Layout::new(&Modules::default());
    assert!(layout.bar_in_use(0));
    assert!(!layout.bar_in_use(1));
    assert!(!layout.is_empty());
}

#[test]
fn moving_clock_to_bar2_uses_second_bar() {
    let mut modules = Modules::default();
    modules.bar_id.clock = 1;
    let layout = Layout::new(&modules);

    assert!(layout.bar_in_use(1));
    let on_bar2: Vec<_> = layout.for_bar(1).collect();
    assert_eq!(on_bar2.len(), 1);
    assert_eq!(on_bar2[0].module, Module::Clock);
}

#[test]
fn invalid_location_and_bar_id_are_skipped() {
    let mut modules = Modules::default();
    modules.location.clock = 7;
    modules.bar_id.media = 9;

    let layout = Layout::new(&modules);
    let names: Vec<_> = layout
        .placements()
        .iter()
        .map(|p| p.module.name())
        .collect();
    assert!(!names.contains(&"clock"));
    assert!(!names.contains(&"media"));
}

#[test]
fn visibility_is_carried_through() {
    let mut modules = Modules::default();
    modules.visibility.clock = false;
    let layout = Layout::new(&modules);

    let clock = layout
        .placements()
        .iter()
        .find(|p| p.module == Module::Clock)
        .expect("clock still placed");
    assert!(!clock.visible);
}

#[test]
fn area_mapping_covers_valid_range() {
    assert_eq!(Area::from_location(0), Some(Area::Left));
    assert_eq!(Area::from_location(1), Some(Area::Center));
    assert_eq!(Area::from_location(2), Some(Area::Right));
    assert_eq!(Area::from_location(3), None);
}
