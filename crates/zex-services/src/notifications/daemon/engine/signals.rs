use zbus::Connection;
use zbus::object_server::SignalEmitter;

use super::super::bus::server::{NotificationsServer, NotificationsServerSignals};
use super::super::OBJECT_PATH;

pub async fn notification_closed(
    conn: &Connection,
    id: u32,
    reason: u32,
) -> zbus::fdo::Result<()> {
    let iface_ref = conn
        .object_server()
        .interface::<_, NotificationsServer>(OBJECT_PATH)
        .await?;
    NotificationsServerSignals::notification_closed(&iface_ref, id, reason)
        .await
        .map_err(zbus::fdo::Error::from)
}

pub async fn action_invoked(conn: &Connection, id: u32, key: &str) -> zbus::fdo::Result<()> {
    let iface_ref = conn
        .object_server()
        .interface::<_, NotificationsServer>(OBJECT_PATH)
        .await?;
    NotificationsServerSignals::action_invoked(&iface_ref, id, key.to_string())
        .await
        .map_err(zbus::fdo::Error::from)
}