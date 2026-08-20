//! Pure tests for NetworkManager fallback data handling.

use zbus::zvariant::OwnedObjectPath;
use zex_services::network::{AccessPoint, is_secure, sort_access_points};

fn ap(ssid: &str, strength: u8, id: u8) -> AccessPoint {
    AccessPoint {
        path: OwnedObjectPath::try_from(format!(
            "/org/freedesktop/NetworkManager/AccessPoint/{id}"
        ))
        .unwrap(),
        ssid: ssid.into(),
        strength,
        secured: false,
        frequency: 5180,
    }
}

#[test]
fn access_points_are_sorted_by_signal_then_ssid() {
    let mut points = vec![ap("zeta", 60, 1), ap("alpha", 90, 2), ap("beta", 90, 3)];
    sort_access_points(&mut points);
    assert_eq!(
        points.iter().map(|p| p.ssid.as_str()).collect::<Vec<_>>(),
        ["alpha", "beta", "zeta"]
    );
}

#[test]
fn security_flags_are_detected_without_guessing_from_ssid() {
    assert!(!is_secure(0, 0, 0));
    assert!(is_secure(1, 0, 0));
    assert!(is_secure(0, 1, 0));
    assert!(is_secure(0, 0, 1));
}

#[test]
fn duplicate_ssids_can_have_distinct_access_points() {
    let a = ap("wifi", 70, 1);
    let b = ap("wifi", 40, 2);
    assert_eq!(a.ssid, b.ssid);
    assert_ne!(a.path, b.path);
}
