//! iwd data types and pure helpers.

use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    pub path: OwnedObjectPath,
    pub ssid: String,
    pub signal_dbm: i16,
    pub security: String,
    pub connected: bool,
    pub known: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub adapter: String,
    pub adapter_powered: bool,
    pub interface: String,
    pub state: String,
    pub scanning: bool,
    pub connected: Option<String>,
    pub networks: Vec<AccessPoint>,
}

pub fn signal_dbm(signal_hundredths_dbm: i16) -> i16 {
    signal_hundredths_dbm / 100
}

pub fn network_sort_key(network: &AccessPoint) -> (bool, i16, String) {
    (network.connected, network.signal_dbm, network.ssid.clone())
}
