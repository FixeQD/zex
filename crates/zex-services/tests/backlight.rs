//! Tests for the sysfs backlight backend, using fixture directories.

use std::fs;
use std::path::PathBuf;
use zex_services::backlight::Backlight;

fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("zex-backlight-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("max_brightness"), "100").unwrap();
    fs::write(dir.join("brightness"), "40").unwrap();
    dir
}

#[test]
fn reads_brightness_and_max() {
    let backlight = Backlight::from_dir(&fixture("reads"));
    assert_eq!(backlight.brightness().unwrap(), 40);
    assert_eq!(backlight.max_brightness().unwrap(), 100);
    assert_eq!(backlight.percent().unwrap(), 0.4);
}

#[test]
fn writes_brightness() {
    let backlight = Backlight::from_dir(&fixture("writes"));
    backlight.set_brightness(75).unwrap();
    assert_eq!(backlight.brightness().unwrap(), 75);
}

#[test]
fn write_is_clamped_to_max() {
    let backlight = Backlight::from_dir(&fixture("clamp"));
    backlight.set_brightness(500).unwrap();
    assert_eq!(backlight.brightness().unwrap(), 100);
}

#[test]
fn percent_roundtrip() {
    let backlight = Backlight::from_dir(&fixture("percent"));
    backlight.set_percent(0.5).unwrap();
    assert_eq!(backlight.brightness().unwrap(), 50);
    assert_eq!(backlight.percent().unwrap(), 0.5);
}

#[test]
fn percent_out_of_range_is_rejected() {
    let backlight = Backlight::from_dir(&fixture("range"));
    assert!(backlight.set_percent(1.5).is_err());
    assert!(backlight.set_percent(-0.1).is_err());
}

#[test]
fn missing_files_error() {
    let dir = std::env::temp_dir().join("zex-backlight-empty");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let backlight = Backlight::from_dir(&dir);
    assert!(backlight.brightness().is_err());
    assert!(backlight.max_brightness().is_err());
}

#[test]
fn detect_with_dir_finds_first_device() {
    let dir = std::env::temp_dir().join("zex-backlight-detect");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("intel_backlight")).unwrap();
    fs::create_dir_all(dir.join("acpi_video0")).unwrap();
    fs::write(dir.join("intel_backlight/max_brightness"), "1").unwrap();
    fs::write(dir.join("intel_backlight/brightness"), "0").unwrap();
    let backlight = Backlight::detect_with_dir(&dir).unwrap();
    assert_eq!(backlight.device_name().unwrap(), "acpi_video0");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn detect_with_dir_returns_none_when_missing() {
    let dir = std::env::temp_dir().join("zex-backlight-none");
    let _ = fs::remove_dir_all(&dir);
    assert!(Backlight::detect_with_dir(&dir).is_none());
}
