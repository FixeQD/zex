//! Tests for the StatusNotifier tray service: dbusmenu parsing plus an integration test against a private D-Bus bus with a fake tray item

mod common;

use common::TestBus;
use zbus::zvariant::OwnedValue;
use zex_services::tray::{MenuEntry, SystemTray, TrayEvent, parse_layout};

fn layout_value(entries: &[(i32, &str, bool)]) -> OwnedValue {
    type Value = zbus::zvariant::Value<'static>;
    let layout: Vec<(i32, std::collections::BTreeMap<String, Value>, Vec<Value>)> = entries
        .iter()
        .map(|(id, label, enabled)| {
            let mut props = std::collections::BTreeMap::new();
            props.insert("label".into(), Value::from(label.to_string()));
            props.insert("enabled".into(), Value::from(*enabled));
            (*id, props, Vec::new())
        })
        .collect();
    let entries: Vec<Value> = layout
        .into_iter()
        .map(|(id, props, children)| {
            Value::Structure(zbus::zvariant::Structure::from((
                id,
                Value::Dict(zbus::zvariant::Dict::from(props)),
                Value::Array(zbus::zvariant::Array::from(children)),
            )))
        })
        .collect();
    OwnedValue::try_from(Value::Array(zbus::zvariant::Array::from(entries))).unwrap()
}

#[test]
fn parses_flat_menu() {
    let value = layout_value(&[(1, "Play", true), (2, "Quit", false)]);
    let menu = parse_layout(&value);
    assert_eq!(
        menu,
        vec![
            MenuEntry {
                id: 1,
                label: "Play".into(),
                enabled: true,
                visible: true,
                children: vec![],
            },
            MenuEntry {
                id: 2,
                label: "Quit".into(),
                enabled: false,
                visible: true,
                children: vec![],
            },
        ]
    );
}

// ---------------------------------------------------------------------------
// Integration tests against a private bus

use std::sync::Mutex;
use zbus::interface;

struct FakeItem {
    activate_calls: Mutex<u32>,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl FakeItem {
    #[zbus(property)]
    fn id(&self) -> String {
        "fake-item".into()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        "Fake Item".into()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "passive".into()
    }

    #[zbus(property)]
    fn icon_name(&self) -> String {
        "fake-icon".into()
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        vec![(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )]
    }

    #[zbus(property)]
    fn menu(&self) -> zbus::zvariant::OwnedObjectPath {
        zbus::zvariant::OwnedObjectPath::try_from("/com/canonical/dbusmenu").unwrap()
    }

    fn activate(&self, _x: i32, _y: i32) {
        *self.activate_calls.lock().unwrap() += 1;
    }

    fn context_menu(&self, _x: i32, _y: i32) {}
}

struct FakeMenu;

#[interface(name = "com.canonical.dbusmenu")]
impl FakeMenu {
    fn get_layout(
        &self,
        _parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (i32, OwnedValue) {
        (1, layout_value(&[(10, "Menu Action", true)]))
    }

    fn event(&self, _id: i32, _event_id: String, _data: OwnedValue, _timestamp: u32) {}
}

#[tokio::test]
async fn registers_items_and_controls_them() {
    let Some(bus) = TestBus::spawn().await else {
        return;
    };
    let bus_conn = bus.conn.clone();
    let tray = SystemTray::host(bus_conn.clone()).await.unwrap();

    let item = FakeItem {
        activate_calls: Mutex::new(0),
    };
    bus_conn
        .object_server()
        .at("/StatusNotifierItem", item)
        .await
        .unwrap();
    bus_conn
        .object_server()
        .at("/com/canonical/dbusmenu", FakeMenu)
        .await
        .unwrap();
    zbus::fdo::DBusProxy::new(&bus_conn)
        .await
        .unwrap()
        .request_name(
            zbus::names::WellKnownName::try_from("org.kde.StatusNotifierItem-1-1").unwrap(),
            zbus::fdo::RequestNameFlags::ReplaceExisting.into(),
        )
        .await
        .unwrap();

    // Register the item against our watcher.
    let watcher: zbus::Proxy<'_> = zbus::proxy::Builder::new(&bus_conn)
        .destination("org.kde.StatusNotifierWatcher")
        .unwrap()
        .path("/StatusNotifierWatcher")
        .unwrap()
        .interface("org.kde.StatusNotifierWatcher")
        .unwrap()
        .build()
        .await
        .unwrap();
    watcher
        .call::<_, _, ()>(
            "RegisterStatusNotifierItem",
            &("org.kde.StatusNotifierItem-1-1".to_string()),
        )
        .await
        .unwrap();

    let added = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tray.events().recv_async(),
    )
    .await
    .expect("ItemAdded");
    match added {
        Ok(TrayEvent::ItemAdded(item)) => {
            assert_eq!(item.service, "org.kde.StatusNotifierItem-1-1");
            assert_eq!(item.id, "fake-item");
            assert_eq!(item.title, "Fake Item");
            assert_eq!(item.icon.name.as_deref(), Some("fake-icon"));
            assert_eq!(item.icon.pixmap.len(), 1);
            assert_eq!(item.icon.pixmap[0].width, 2);
            assert_eq!(item.icon.pixmap[0].height, 2);
            assert_eq!(item.menu.as_deref(), Some("/com/canonical/dbusmenu"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let items = tray.items().await.unwrap();
    assert_eq!(items.len(), 1);

    let menu = tray.menu("org.kde.StatusNotifierItem-1-1").await.unwrap();
    assert_eq!(menu.len(), 1);
    assert_eq!(menu[0].id, 10);
    assert_eq!(menu[0].label, "Menu Action");

    tray.activate("org.kde.StatusNotifierItem-1-1", 10, 20)
        .await
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let calls = loop {
        let calls = *bus_conn
            .object_server()
            .interface::<_, FakeItem>("/StatusNotifierItem")
            .await
            .unwrap()
            .get()
            .await
            .activate_calls
            .lock()
            .unwrap();
        if calls == 1 || std::time::Instant::now() >= deadline {
            break calls;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert_eq!(calls, 1);
}
