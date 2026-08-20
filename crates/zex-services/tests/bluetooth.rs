//! Pure tests for BlueZ device presentation.

use zbus::zvariant::OwnedObjectPath;
use zex_services::bluetooth::{BluetoothDevice, sort_devices};

fn device(alias: &str) -> BluetoothDevice {
    BluetoothDevice {
        path: OwnedObjectPath::try_from(format!("/org/bluez/hci0/dev_{alias}")).unwrap(),
        address: "00:11:22:33:44:55".into(),
        alias: alias.into(),
        icon: "audio-card".into(),
        connected: false,
        paired: false,
        rssi: Some(-55),
        class: Some(0x240404),
    }
}

#[test]
fn devices_are_sorted_case_insensitively() {
    let mut devices = vec![device("zeta"), device("Alpha"), device("beta")];
    sort_devices(&mut devices);
    assert_eq!(
        devices.iter().map(|d| d.alias.as_str()).collect::<Vec<_>>(),
        ["Alpha", "beta", "zeta"]
    );
}

#[test]
fn device_metadata_is_preserved() {
    let device = device("Headphones");
    assert_eq!(device.rssi, Some(-55));
    assert_eq!(device.class, Some(0x240404));
    assert_eq!(device.icon, "audio-card");
}
