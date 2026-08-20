//! NetworkManager data types and pure helpers.

use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    pub path: OwnedObjectPath,
    pub ssid: String,
    pub strength: u8,
    pub secured: bool,
    pub frequency: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthernetStatus {
    pub connected: bool,
    pub interface: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSnapshot {
    pub wifi_enabled: bool,
    pub access_points: Vec<AccessPoint>,
    pub ethernet: EthernetStatus,
}

pub fn sort_access_points(points: &mut [AccessPoint]) {
    points.sort_by(|a, b| {
        b.strength
            .cmp(&a.strength)
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
}

pub fn is_secure(flags: u32, wpa_flags: u32, rsn_flags: u32) -> bool {
    flags != 0 || wpa_flags != 0 || rsn_flags != 0
}
