//! Pinned app registry: JSON persistence and change broadcasts

use zex_launcher::apps::{PinnedApps, default_pins_path};

fn fresh_pins(path: &std::path::Path) -> PinnedApps {
    PinnedApps::load(Some(path))
}

#[test]
fn default_path_lives_in_zex_config() {
    let path = default_pins_path();
    assert!(path.ends_with("zex/pinned.json"));
}

#[test]
fn toggles_persist_and_broadcast() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pinned.json");
    let pins = fresh_pins(&path);
    let changes = pins.changes();

    pins.toggle("a.desktop");
    pins.toggle("b.desktop");
    pins.toggle("a.desktop");

    assert_eq!(pins.pinned(), vec!["b.desktop"]);
    assert!(!pins.is_pinned("a.desktop"));
    assert!(pins.is_pinned("b.desktop"));

    assert_eq!(changes.try_recv().unwrap(), vec!["a.desktop"]);
    assert_eq!(changes.try_recv().unwrap(), vec!["a.desktop", "b.desktop"]);
    assert_eq!(changes.try_recv().unwrap(), vec!["b.desktop"]);
    assert!(changes.try_recv().is_err());
}

#[test]
fn reload_sees_last_written_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pinned.json");
    let pins = fresh_pins(&path);
    pins.toggle("a.desktop");
    pins.toggle("b.desktop");

    let reloaded = fresh_pins(&path);
    assert_eq!(reloaded.pinned(), vec!["a.desktop", "b.desktop"]);
}

#[test]
fn missing_file_loads_empty() {
    let dir = tempfile::tempdir().unwrap();
    let pins = fresh_pins(&dir.path().join("none/pinned.json"));
    assert!(pins.pinned().is_empty());
}
