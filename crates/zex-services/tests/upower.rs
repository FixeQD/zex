//! Tests for the UPower battery service: state/icon mapping plus an integration test against a private D-Bus bus with a fake UPower daemon

mod common;

use common::TestBus;
use zex_services::upower::{Battery, BatteryState, Upower, UpowerEvent};

#[test]
fn state_mapping() {
    assert_eq!(BatteryState::from_u32(0), BatteryState::Unknown);
    assert_eq!(BatteryState::from_u32(1), BatteryState::Charging);
    assert_eq!(BatteryState::from_u32(2), BatteryState::Discharging);
    assert_eq!(BatteryState::from_u32(3), BatteryState::Empty);
    assert_eq!(BatteryState::from_u32(4), BatteryState::FullyCharged);
    assert_eq!(BatteryState::from_u32(5), BatteryState::PendingCharge);
    assert_eq!(BatteryState::from_u32(6), BatteryState::PendingDischarge);
    assert_eq!(BatteryState::from_u32(99), BatteryState::Unknown);
    assert!(BatteryState::Charging.is_charging());
    assert!(BatteryState::PendingCharge.is_charging());
    assert!(!BatteryState::Discharging.is_charging());
}

fn battery(percent: u8, state: BatteryState) -> Battery {
    Battery {
        path: "/test/battery".into(),
        percent: percent as f32,
        state,
        is_present: true,
    }
}

#[test]
fn icon_tiers() {
    let charging = battery(30, BatteryState::Charging);
    assert_eq!(charging.icon_name(), "bolt");
    assert!(charging.charging());
    assert_eq!(
        battery(100, BatteryState::Discharging).icon_name(),
        "battery_android_full"
    );
    assert_eq!(
        battery(97, BatteryState::Discharging).icon_name(),
        "battery_android_6"
    );
    assert_eq!(
        battery(82, BatteryState::Discharging).icon_name(),
        "battery_android_5"
    );
    assert_eq!(
        battery(61, BatteryState::Discharging).icon_name(),
        "battery_android_4"
    );
    assert_eq!(
        battery(41, BatteryState::Discharging).icon_name(),
        "battery_android_3"
    );
    assert_eq!(
        battery(26, BatteryState::Discharging).icon_name(),
        "battery_android_2"
    );
    assert_eq!(
        battery(11, BatteryState::Discharging).icon_name(),
        "battery_android_1"
    );
    assert_eq!(
        battery(10, BatteryState::Discharging).icon_name(),
        "battery_android_0"
    );
    assert_eq!(
        battery(0, BatteryState::Discharging).icon_name(),
        "battery_android_0"
    );
}

// ---------------------------------------------------------------------------
// Integration tests against a private bus

use zbus::interface;

struct FakeBattery {
    percent: std::sync::Mutex<f64>,
}

#[interface(name = "org.freedesktop.UPower.Device")]
impl FakeBattery {
    #[zbus(property)]
    fn percentage(&self) -> f64 {
        *self.percent.lock().unwrap()
    }

    #[zbus(property)]
    fn set_percentage(&self, value: f64) {
        *self.percent.lock().unwrap() = value;
    }

    #[zbus(property)]
    fn state(&self) -> u32 {
        2
    }

    #[zbus(property)]
    fn is_present(&self) -> bool {
        true
    }
}

struct FakeUpower;

#[interface(name = "org.freedesktop.UPower")]
impl FakeUpower {
    fn get_all_devices(&self) -> Vec<zbus::zvariant::OwnedObjectPath> {
        vec![
            zbus::zvariant::OwnedObjectPath::try_from("/org/freedesktop/UPower/devices/BAT0")
                .unwrap(),
        ]
    }

    #[zbus(signal)]
    async fn device_added(
        signal_emitter: &zbus::object_server::SignalEmitter<'_>,
        path: &zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn device_removed(
        signal_emitter: &zbus::object_server::SignalEmitter<'_>,
        path: &zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;
}

#[tokio::test]
async fn monitors_battery_changes() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let bus_conn = bus.conn.clone();
    bus_conn
        .object_server()
        .at("/org/freedesktop/UPower", FakeUpower)
        .await
        .unwrap();
    bus_conn
        .object_server()
        .at(
            "/org/freedesktop/UPower/devices/BAT0",
            FakeBattery {
                percent: std::sync::Mutex::new(42.0),
            },
        )
        .await
        .unwrap();
    zbus::fdo::DBusProxy::new(&bus_conn)
        .await
        .unwrap()
        .request_name(
            zbus::names::WellKnownName::try_from("org.freedesktop.UPower").unwrap(),
            zbus::fdo::RequestNameFlags::ReplaceExisting.into(),
        )
        .await
        .unwrap();

    let upower = Upower::connect(bus_conn.clone()).await.unwrap();
    let initial = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        upower.events().recv_async(),
    )
    .await
    .expect("initial BatteryChanged");
    match initial {
        Ok(UpowerEvent::BatteryChanged(battery)) => {
            assert_eq!(battery.path, "/org/freedesktop/UPower/devices/BAT0");
            assert_eq!(battery.percent_u8(), 42);
            assert_eq!(battery.state, BatteryState::Discharging);
            assert!(battery.is_present);
        }
        other => panic!("unexpected initial event: {other:?}"),
    }

    let batteries = upower.batteries().await.unwrap();
    assert_eq!(batteries.len(), 1);
    assert_eq!(batteries[0].percent_u8(), 42);

    // Change the percentage on the bus; the monitor must report it
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(&bus_conn)
        .destination("org.freedesktop.UPower")
        .unwrap()
        .path("/org/freedesktop/UPower/devices/BAT0")
        .unwrap()
        .interface("org.freedesktop.UPower.Device")
        .unwrap()
        .build()
        .await
        .unwrap();
    proxy.set_property("Percentage", 77.0_f64).await.unwrap();

    let changed = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        upower.events().recv_async(),
    )
    .await
    .expect("changed BatteryChanged");
    match changed {
        Ok(UpowerEvent::BatteryChanged(battery)) => {
            assert_eq!(battery.percent_u8(), 77);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
