//! Schema and persistence tests

use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use zex_core::settings::{Anchor, Settings};
use zex_core::store::SettingsStore;

fn test_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "zex-settings-test-{name}-{}-{stamp}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn defaults_match_reference() {
    let s = Settings::default();

    // Exo/ignis/user_settings.py: Appearance.WallpaperColors
    let wc = &s.appearance.wallcolors;
    assert_eq!(wc.quickselect_path, "");
    assert_eq!(wc.wallpaper_path, "");
    assert_eq!(wc.color_scheme, "tonal_spot");
    assert!(wc.dark_mode);
    assert!(!wc.auto_dark.enabled);
    assert_eq!((wc.auto_dark.start_hour, wc.auto_dark.start_min), (22, 0));
    assert_eq!((wc.auto_dark.end_hour, wc.auto_dark.end_min), (6, 0));

    // Modules.Locations: launcher/left = 0, center = 1, tray/right = 2
    let m = &s.interface.modules;
    assert_eq!(m.location.launcher, 0);
    assert_eq!(m.location.window_info, 0);
    assert_eq!(m.location.media, 0);
    assert_eq!(m.location.workspaces, 1);
    assert_eq!(m.location.tasks, 1);
    assert_eq!(m.location.recording_indicator, 2);
    assert_eq!(m.location.systeminfotray, 2);
    assert_eq!(m.location.clock, 2);

    // Modules.Visibility
    assert!(m.visibility.clock);
    assert!(!m.visibility.tasks);
    assert!(!m.visibility.launcher);

    // Modules.BarID: everything on the primary bar
    assert_eq!(m.bar_id.clock, 0);
    assert_eq!(m.bar_id.workspaces, 0);

    // Modules.ModuleOptions
    assert!(m.options.show_date);
    assert_eq!(m.options.recording_indicator, "recording");
    assert_eq!(m.options.workspaces_style, "numbers");
    assert_eq!(m.options.fixed_workspaces_amount, 5);

    // Interface.Bar / Bar2
    let bar = &s.interface.bar;
    assert_eq!(bar.side, "bottom");
    assert!(bar.bar_background);
    let bar2 = &s.interface.bar2;
    assert!(bar2.enabled);
    assert_eq!(bar2.side, "top");

    // Interface.Notifications / Launcher / Misc
    assert_eq!(
        s.interface.notifications.anchor,
        vec![Anchor::Top, Anchor::Right]
    );
    assert_eq!(s.interface.launcher.layout, "grid");
    assert_eq!(s.interface.launcher.ai.endpoint, "http://localhost:11434");
    assert_eq!(s.interface.launcher.ai.model, "Qwythos-9B-v2:latest");
    assert_eq!(s.interface.launcher.clipboard.history_limit, 500);
    assert!(!s.interface.launcher.clipboard.keep_passwords);
    assert!(s.interface.misc.shell_corners);
    assert_eq!(s.interface.misc.screen_corners, "not_fullscreen");

    // Services
    assert!(s.services.recorder.start_notification);
    assert!(s.services.recorder.record_audio);
    assert_eq!(s.services.osd.anchor, vec![Anchor::Bottom, Anchor::Right]);
    assert!(s.services.lockscreen.blur);
    assert!(s.services.lockscreen.clock);
}

#[test]
fn json_round_trip() {
    let mut s = Settings::default();
    s.appearance.wallcolors.wallpaper_path = "/tmp/wall.png".to_string();
    s.appearance.wallcolors.dark_mode = false;
    s.interface.bar.floating = true;
    s.interface.notifications.compact_popup = true;
    s.services.recorder.record_audio = false;

    let raw = serde_json::to_string_pretty(&s).unwrap();
    let back: Settings = serde_json::from_str(&raw).unwrap();
    assert_eq!(back, s);
}

#[test]
fn anchors_serialize_lowercase() {
    let raw = serde_json::to_string(&vec![Anchor::Top, Anchor::Right]).unwrap();
    assert_eq!(raw, r#"["top","right"]"#);

    let parsed: Vec<Anchor> = serde_json::from_str(r#"["bottom"]"#).unwrap();
    assert_eq!(parsed, vec![Anchor::Bottom]);
}

#[test]
fn load_missing_file_returns_defaults() {
    let dir = test_dir("missing");
    let store = SettingsStore::load_from(dir.join("settings.json")).unwrap();
    assert_eq!(store.get(), &Settings::default());
}

#[test]
fn load_partial_file_merges_with_defaults() {
    let dir = test_dir("partial");
    let path = dir.join("settings.json");
    fs::write(
        &path,
        r#"{"appearance":{"wallcolors":{"dark_mode":false,"wallpaper_path":"/w.png"}}}"#,
    )
    .unwrap();

    let store = SettingsStore::load_from(&path).unwrap();
    assert!(!store.get().appearance.wallcolors.dark_mode);
    assert_eq!(store.get().appearance.wallcolors.wallpaper_path, "/w.png");
    // untouched groups keep their defaults
    assert_eq!(store.get().appearance.wallcolors.color_scheme, "tonal_spot");
    assert_eq!(store.get().interface.bar.side, "bottom");
}

#[test]
fn unknown_keys_are_rejected() {
    let dir = test_dir("unknown");
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"appearance":{"wallcolors":{"dark_mod":true}}}"#).unwrap();
    assert!(SettingsStore::load_from(&path).is_err());

    let path2 = dir.join("settings2.json");
    fs::write(&path2, r#"{"totally_new_group":{}}"#).unwrap();
    assert!(SettingsStore::load_from(&path2).is_err());
}

#[test]
fn invalid_json_is_rejected() {
    let dir = test_dir("invalid");
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"appearance": "#).unwrap();
    assert!(SettingsStore::load_from(&path).is_err());
}

#[test]
fn update_persists_atomically() {
    let dir = test_dir("update");
    let path = dir.join("settings.json");
    let mut store = SettingsStore::load_from(&path).unwrap();

    store
        .update(|s| {
            s.appearance.wallcolors.wallpaper_path = "/tmp/wall2.png".to_string();
            s.services.lockscreen.blur = false;
        })
        .unwrap();

    let reloaded = SettingsStore::load_from(&path).unwrap();
    assert_eq!(
        reloaded.get().appearance.wallcolors.wallpaper_path,
        "/tmp/wall2.png"
    );
    assert!(!reloaded.get().services.lockscreen.blur);

    // no temporary file left behind
    assert!(!path.parent().unwrap().join(".settings.json.tmp").exists());
    // file is a single complete JSON object
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: Settings = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed, *reloaded.get());
}

#[test]
fn subscription_fires_and_unsubscribes() {
    let dir = test_dir("subscribe");
    let path = dir.join("settings.json");
    let mut store = SettingsStore::load_from(&path).unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(std::sync::Mutex::new(None));
    let sub = store.subscribe({
        let calls = Arc::clone(&calls);
        let observed = Arc::clone(&observed);
        move |settings: &Settings| {
            calls.fetch_add(1, Ordering::SeqCst);
            *observed.lock().unwrap() = Some(settings.appearance.wallcolors.dark_mode);
        }
    });

    store
        .update(|s| s.appearance.wallcolors.dark_mode = false)
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(*observed.lock().unwrap(), Some(false));

    drop(sub);
    store
        .update(|s| s.appearance.wallcolors.dark_mode = true)
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn default_path_lives_under_zex_config_dir() {
    let path = SettingsStore::default_path();
    assert_eq!(path.file_name().unwrap(), "settings.json");
    assert!(path.to_string_lossy().ends_with("zex/settings.json"));
}
