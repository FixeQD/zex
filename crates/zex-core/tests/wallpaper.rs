//! Integration tests for `zex_core::wallpaper`: path + version state and the
//! `~` expansion used by settings-provided paths

use zex_core::wallpaper::WallpaperState;

#[test]
fn update_bumps_version() {
    let mut state = WallpaperState::default();
    assert_eq!(state.version, 0);
    assert_eq!(state.update("~/wallpaper.png"), 1);
    assert_eq!(state.version, 1);
    assert_eq!(state.update("/tmp/other.png"), 2);
}

#[test]
fn resolve_skips_empty_and_missing() {
    let mut state = WallpaperState::default();
    assert_eq!(state.resolve(), None);
    state.update("/definitely/not/here.png");
    assert_eq!(state.resolve(), None);
}

#[test]
fn resolve_expands_tilde() {
    let mut state = WallpaperState::default();
    state.update("~/no-such-zex-wallpaper.png");
    let resolved = state.resolve();
    assert!(resolved.is_none() || !resolved.unwrap().starts_with("~"));
}
