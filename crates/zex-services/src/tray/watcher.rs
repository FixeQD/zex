//! The `org.kde.StatusNotifierWatcher` D-Bus service implementation

use super::{HOST_PREFIX, TrayEvent, WATCHER_PATH, item};
use flume::Sender;
use std::sync::{Arc, Mutex};
use zbus::Connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;

#[derive(Debug, Default)]
pub struct WatcherState {
    pub(crate) items: Mutex<Vec<String>>,
    pub(crate) host_registered: Mutex<bool>,
}

pub struct StatusNotifierWatcher {
    conn: Connection,
    state: Arc<WatcherState>,
    tx: Sender<TrayEvent>,
}

impl StatusNotifierWatcher {
    pub fn new(conn: Connection, state: Arc<WatcherState>, tx: Sender<TrayEvent>) -> Self {
        Self { conn, state, tx }
    }

    async fn announce_item(&self, service: String) -> zbus::fdo::Result<()> {
        let iface_ref = self
            .conn
            .object_server()
            .interface::<_, StatusNotifierWatcher>(WATCHER_PATH)
            .await?;
        StatusNotifierWatcherSignals::status_notifier_item_registered(&iface_ref, &service)
            .await
            .map_err(zbus::fdo::Error::from)
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    #[zbus(property)]
    pub fn registered_status_notifier_items(&self) -> Vec<String> {
        self.state.items.lock().unwrap().clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        *self.state.host_registered.lock().unwrap()
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    async fn register_status_notifier_item(&self, service: String) -> zbus::fdo::Result<()> {
        let is_new = {
            let mut items = self.state.items.lock().unwrap();
            if items.contains(&service) {
                false
            } else {
                items.push(service.clone());
                true
            }
        };
        if is_new {
            let tx = self.tx.clone();
            let conn = self.conn.clone();
            let service = service.clone();
            tokio::spawn(async move {
                if let Ok(Some(item)) =
                    item::fetch_item(&conn, &service, super::ITEM_DEFAULT_PATH).await
                {
                    let _ = tx.send(TrayEvent::ItemAdded(item));
                }
            });
        }
        self.announce_item(service).await
    }

    async fn register_status_notifier_host(&self, service: String) -> zbus::fdo::Result<()> {
        if service.starts_with(HOST_PREFIX) {
            *self.state.host_registered.lock().unwrap() = true;
        }
        let iface_ref = self
            .conn
            .object_server()
            .interface::<_, StatusNotifierWatcher>(WATCHER_PATH)
            .await?;
        StatusNotifierWatcherSignals::status_notifier_host_registered(&iface_ref, &service)
            .await
            .map_err(zbus::fdo::Error::from)
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;
}
