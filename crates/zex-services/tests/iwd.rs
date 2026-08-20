//! Pure tests for the iwd backend. No real Wi-Fi adapter is required.

use zbus::zvariant::OwnedObjectPath;
use zex_services::iwd::{AccessPoint, Agent, network_sort_key, signal_dbm};

fn ap(ssid: &str, signal_dbm: i16, connected: bool) -> AccessPoint {
    AccessPoint {
        path: OwnedObjectPath::try_from(format!("/net/connman/iwd/0/network/{ssid}")).unwrap(),
        ssid: ssid.into(),
        signal_dbm,
        security: "psk".into(),
        connected,
        known: true,
    }
}

#[test]
fn converts_iwd_signal_from_hundredths_of_dbm() {
    assert_eq!(signal_dbm(-4700), -47);
    assert_eq!(signal_dbm(-6530), -65);
    assert_eq!(signal_dbm(0), 0);
}

#[test]
fn network_sort_key_preserves_signal_and_connection_state() {
    let network = ap("zex", -42, true);
    assert_eq!(network_sort_key(&network), (true, -42, "zex".into()));
}

#[test]
fn agent_lifecycle_is_safe_without_a_system_bus() {
    let agent = Agent::default();
    agent.set_passphrase(Some("correct horse battery staple".into()));
    agent.clear();
}
