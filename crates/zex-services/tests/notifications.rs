//! Tests for the notification daemon: age labels plus integration tests against a private D-Bus bus

mod common;

use common::TestBus;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::time::Duration;
use zbus::proxy;
use zbus::zvariant::OwnedValue;
use zex_services::notifications::{
    NotificationAction, NotificationEvent, Notifications, NotificationsConfig, Urgency,
    relative_age,
};

// ---------------------------------------------------------------------------
// Unit tests

#[test]
fn age_labels() {
    let now = 1_000_000;
    assert_eq!(relative_age(now, now), "now");
    assert_eq!(relative_age(now, now - 5), "now");
    assert_eq!(relative_age(now, now - 59), "now");
    assert_eq!(relative_age(now, now - 60), "1m");
    assert_eq!(relative_age(now, now - 3_599), "59m");
    assert_eq!(relative_age(now, now - 3_600), "1h");
    assert_eq!(relative_age(now, now - 86_399), "23h");
    assert_eq!(relative_age(now, now - 86_400), "1d");
    assert_eq!(relative_age(now, now - 172_800), "2d");
    assert_eq!(relative_age(now, now + 10), "now");
}

#[test]
fn urgency_mapping() {
    assert_eq!(Urgency::from_u8(0), Urgency::Low);
    assert_eq!(Urgency::from_u8(1), Urgency::Normal);
    assert_eq!(Urgency::from_u8(2), Urgency::Critical);
    assert_eq!(Urgency::from_u8(42), Urgency::Normal);
    assert_eq!(Urgency::Critical.as_u8(), 2);
}

#[test]
fn config_defaults() {
    let config = NotificationsConfig::default();
    assert_eq!(config.timeout_ms, 5000);
    assert_eq!(config.max_popups, 3);
    assert_eq!(config.history_size, 100);
    assert!(!config.dnd);
}

// ---------------------------------------------------------------------------
// Integration tests against a private bus

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications",
    gen_blocking = false
)]
trait NotificationsClient {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
    fn close_notification(&self, id: u32) -> zbus::Result<()>;
    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;
    fn get_server_information(&self) -> zbus::Result<(String, String, String, String)>;
    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;
}

async fn next_event(service: &Notifications) -> NotificationEvent {
    tokio::time::timeout(Duration::from_secs(3), service.events().recv_async())
        .await
        .expect("event within 3 s")
        .expect("channel open")
}

async fn notify(proxy: &NotificationsClientProxy<'_>, summary: &str, body: &str) -> u32 {
    proxy
        .notify("app", 0, "icon", summary, body, vec![], HashMap::new(), -1)
        .await
        .unwrap()
}

#[tokio::test]
async fn notify_reports_id_and_events() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let id = notify(&proxy, "Summary", "Body").await;
    assert!(id >= 1);

    match next_event(&service).await {
        NotificationEvent::Popup(notification) => {
            assert_eq!(notification.id, id);
            assert_eq!(notification.app_name, "app");
            assert_eq!(notification.app_icon, "icon");
            assert_eq!(notification.summary, "Summary");
            assert_eq!(notification.body, "Body");
            assert_eq!(notification.urgency, Urgency::Normal);
            assert_eq!(notification.timeout_ms, 5000);
            assert!(notification.popup);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match next_event(&service).await {
        NotificationEvent::Notified(notification) => assert_eq!(notification.id, id),
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(service.notifications().len(), 1);
    assert_eq!(service.notifications()[0].id, id);
    assert_eq!(service.popups(), vec![id]);
}

#[tokio::test]
async fn server_information_and_capabilities() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let _service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let capabilities = proxy.get_capabilities().await.unwrap();
    for expected in ["actions", "body", "icon-static", "persistence"] {
        assert!(
            capabilities.iter().any(|cap| cap == expected),
            "missing capability {expected}"
        );
    }
    let (name, vendor, version, spec_version) = proxy.get_server_information().await.unwrap();
    assert!(!name.is_empty());
    assert!(!vendor.is_empty());
    assert!(!version.is_empty());
    assert_eq!(spec_version, "1.2");
}

#[tokio::test]
async fn close_notification_emits_signal_and_events() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let id = notify(&proxy, "Summary", "Body").await;
    let _ = next_event(&service).await; // Popup
    let _ = next_event(&service).await; // Notified

    let mut signals = proxy.receive_notification_closed().await.unwrap();
    service.close(id).await.unwrap();

    let signal = tokio::time::timeout(Duration::from_secs(3), signals.next())
        .await
        .expect("signal within 3 s")
        .expect("signal stream open");
    let args = signal.args().unwrap();
    assert_eq!(args.id, id);
    assert_eq!(args.reason, 2);

    match next_event(&service).await {
        NotificationEvent::Closed(closed_id) => assert_eq!(closed_id, id),
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(service.notifications().is_empty());
    assert!(service.popups().is_empty());
}

#[tokio::test]
async fn replace_keeps_id_and_closes_old() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let id = notify(&proxy, "First", "Body").await;
    let _ = next_event(&service).await; // Popup
    let _ = next_event(&service).await; // Notified

    let mut signals = proxy.receive_notification_closed().await.unwrap();
    let replaced = proxy
        .notify(
            "app",
            id,
            "icon",
            "Second",
            "Body",
            vec![],
            HashMap::new(),
            -1,
        )
        .await
        .unwrap();
    assert_eq!(replaced, id);

    match next_event(&service).await {
        NotificationEvent::Closed(closed_id) => assert_eq!(closed_id, id),
        other => panic!("unexpected event: {other:?}"),
    }
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => assert_eq!(notification.summary, "Second"),
        other => panic!("unexpected event: {other:?}"),
    }
    match next_event(&service).await {
        NotificationEvent::Notified(notification) => {
            assert_eq!(notification.id, id);
            assert_eq!(notification.summary, "Second");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let signal = tokio::time::timeout(Duration::from_secs(3), signals.next())
        .await
        .expect("signal within 3 s")
        .expect("signal stream open");
    let args = signal.args().unwrap();
    assert_eq!(args.id, id);
    assert_eq!(args.reason, 2);

    let history = service.notifications();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].summary, "Second");
}

#[tokio::test]
async fn dnd_suppresses_popups_but_keeps_history() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    service.set_dnd(true);
    match next_event(&service).await {
        NotificationEvent::DndChanged(true) => {}
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(service.dnd());

    let id = notify(&proxy, "Summary", "Body").await;
    match next_event(&service).await {
        NotificationEvent::Notified(notification) => {
            assert_eq!(notification.id, id);
            assert!(!notification.popup);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(service.popups().is_empty());
    assert!(
        tokio::time::timeout(Duration::from_millis(300), service.events().recv_async())
            .await
            .is_err(),
        "no Popup event expected while DND is on"
    );

    service.set_dnd(false);
    match next_event(&service).await {
        NotificationEvent::DndChanged(false) => {}
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn popup_limit_dismisses_oldest() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let config = NotificationsConfig {
        max_popups: 1,
        ..Default::default()
    };
    let service = Notifications::connect(bus.conn.clone(), config)
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let first = notify(&proxy, "First", "Body").await;
    let _ = next_event(&service).await; // Popup(first)
    let _ = next_event(&service).await; // Notified(first)

    let second = notify(&proxy, "Second", "Body").await;
    match next_event(&service).await {
        NotificationEvent::Dismissed(dismissed) => assert_eq!(dismissed, first),
        other => panic!("unexpected event: {other:?}"),
    }
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => assert_eq!(notification.id, second),
        other => panic!("unexpected event: {other:?}"),
    }
    match next_event(&service).await {
        NotificationEvent::Notified(notification) => assert_eq!(notification.id, second),
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(service.popups(), vec![second]);
    assert_eq!(service.notifications().len(), 2);
}

#[tokio::test]
async fn popup_dismissed_after_timeout() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let config = NotificationsConfig {
        timeout_ms: 300,
        ..Default::default()
    };
    let service = Notifications::connect(bus.conn.clone(), config)
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let id = notify(&proxy, "Summary", "Body").await;

    let mut saw_popup = false;
    let mut saw_notified = false;
    let dismissed = loop {
        match next_event(&service).await {
            NotificationEvent::Popup(_) => saw_popup = true,
            NotificationEvent::Notified(_) => saw_notified = true,
            NotificationEvent::Dismissed(dismissed) => break dismissed,
            other => panic!("unexpected event: {other:?}"),
        }
    };
    assert!(
        saw_popup && saw_notified,
        "popup and notified must precede dismissal"
    );
    assert_eq!(dismissed, id);

    assert!(service.popups().is_empty());
    assert!(
        service.notifications().iter().any(|entry| entry.id == id),
        "dismissed notification stays in the history"
    );

    service.close(id).await.unwrap();
    match next_event(&service).await {
        NotificationEvent::Closed(closed_id) => assert_eq!(closed_id, id),
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(service.notifications().is_empty());
}

#[tokio::test]
async fn history_is_bounded_ring_buffer() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let config = NotificationsConfig {
        history_size: 2,
        ..Default::default()
    };
    let service = Notifications::connect(bus.conn.clone(), config)
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let first = notify(&proxy, "One", "Body").await;
    let _ = next_event(&service).await; // Popup(first)
    let _ = next_event(&service).await; // Notified(first)
    let second = notify(&proxy, "Two", "Body").await;
    let _ = next_event(&service).await; // Popup(second)
    let _ = next_event(&service).await; // Notified(second)

    let third = notify(&proxy, "Three", "Body").await;
    let mut saw_dismissed = false;
    loop {
        match next_event(&service).await {
            NotificationEvent::Dismissed(dismissed) => {
                assert_eq!(dismissed, first);
                saw_dismissed = true;
            }
            NotificationEvent::Popup(notification) => {
                assert_eq!(notification.id, third);
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
    assert!(saw_dismissed, "evicted popup must be dismissed");
    let _ = next_event(&service).await; // Notified(third)

    let history = service.notifications();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, second);
    assert_eq!(history[1].id, third);
    assert!(history.iter().all(|entry| entry.id != first));
}

#[tokio::test]
async fn empty_notifications_are_rejected() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let id = proxy
        .notify("app", 0, "", "", "", vec![], HashMap::new(), -1)
        .await
        .unwrap();
    assert_eq!(id, 0);
    assert!(
        tokio::time::timeout(Duration::from_millis(300), service.events().recv_async())
            .await
            .is_err(),
        "no events for rejected notification"
    );
    assert!(service.notifications().is_empty());
}

#[tokio::test]
async fn actions_are_parsed_and_invokable() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let actions = vec!["open".into(), "Open".into(), "reply".into(), "Reply".into()];
    let id = proxy
        .notify("app", 0, "", "Summary", "Body", actions, HashMap::new(), -1)
        .await
        .unwrap();
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => {
            assert_eq!(
                notification.actions,
                vec![
                    NotificationAction {
                        key: "open".into(),
                        label: "Open".into(),
                    },
                    NotificationAction {
                        key: "reply".into(),
                        label: "Reply".into(),
                    },
                ]
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
    let _ = next_event(&service).await; // Notified

    let mut signals = proxy.receive_action_invoked().await.unwrap();
    service.invoke_action(id, "open").await.unwrap();
    let signal = tokio::time::timeout(Duration::from_secs(3), signals.next())
        .await
        .expect("signal within 3 s")
        .expect("signal stream open");
    let args = signal.args().unwrap();
    assert_eq!(args.id, id);
    assert_eq!(args.action_key, "open");

    // Odd action arrays are truncated to whole pairs
    let odd = proxy
        .notify(
            "app",
            0,
            "",
            "Summary",
            "Body",
            vec!["first".into(), "First".into(), "orphan".into()],
            HashMap::new(),
            -1,
        )
        .await
        .unwrap();
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => {
            assert_eq!(notification.id, odd);
            assert_eq!(notification.actions.len(), 1);
            assert_eq!(notification.actions[0].key, "first");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn hints_set_urgency_icon_and_timeout() {
    let Some(bus) = TestBus::spawn().await else {
        eprintln!("skipping: dbus-daemon not available");
        return;
    };
    let service = Notifications::connect(bus.conn.clone(), NotificationsConfig::default())
        .await
        .unwrap();
    let proxy = NotificationsClientProxy::new(&bus.conn).await.unwrap();

    let mut hints = HashMap::new();
    hints.insert("urgency".into(), OwnedValue::try_from(2u8).unwrap());
    hints.insert(
        "image-path".into(),
        OwnedValue::from(zbus::zvariant::Str::from("/tmp/logo.png")),
    );
    let id = proxy
        .notify("app", 0, "", "Summary", "Body", vec![], hints, 0)
        .await
        .unwrap();
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => {
            assert_eq!(notification.id, id);
            assert_eq!(notification.urgency, Urgency::Critical);
            assert_eq!(notification.app_icon, "/tmp/logo.png");
            assert_eq!(notification.timeout_ms, 0, "expire_timeout 0 never expires");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    let _ = next_event(&service).await; // Notified

    let mut resident = HashMap::new();
    resident.insert("resident".into(), OwnedValue::try_from(true).unwrap());
    let resident_id = proxy
        .notify("app", 0, "icon", "Summary", "Body", vec![], resident, -1)
        .await
        .unwrap();
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => {
            assert_eq!(notification.id, resident_id);
            assert_eq!(notification.timeout_ms, 0, "resident never expires");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    let _ = next_event(&service).await; // Notified

    let explicit_id = proxy
        .notify(
            "app",
            0,
            "icon",
            "Summary",
            "Body",
            vec![],
            HashMap::new(),
            1200,
        )
        .await
        .unwrap();
    match next_event(&service).await {
        NotificationEvent::Popup(notification) => {
            assert_eq!(notification.id, explicit_id);
            assert_eq!(notification.timeout_ms, 1200);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
