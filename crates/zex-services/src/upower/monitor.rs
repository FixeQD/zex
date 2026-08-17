//! Background monitor: device lifecycle and battery changes

use super::{
    DEVICE_INTERFACE, DEVICES_PATH_PREFIX, UPOWER_DESTINATION, UPOWER_INTERFACE, UpowerEvent,
    device,
};
use flume::Sender;
use futures_util::StreamExt;
use std::collections::HashMap;
use tracing::warn;
use zbus::match_rule::MatchRule;
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MessageStream};

pub async fn run(conn: Connection, tx: Sender<UpowerEvent>) {
    let rule = |member: &'static str, iface: &'static str| {
        MatchRule::builder()
            .sender(UPOWER_DESTINATION)
            .unwrap()
            .member(member)
            .unwrap()
            .interface(iface)
            .unwrap()
            .msg_type(Type::Signal)
            .build()
    };
    let mut added =
        match MessageStream::for_match_rule(rule("DeviceAdded", UPOWER_INTERFACE), &conn, None)
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                warn!(%error, "upower: cannot subscribe to DeviceAdded");
                return;
            }
        };
    let mut removed =
        match MessageStream::for_match_rule(rule("DeviceRemoved", UPOWER_INTERFACE), &conn, None)
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                warn!(%error, "upower: cannot subscribe to DeviceRemoved");
                return;
            }
        };
    let changed_rule = MatchRule::builder()
        .sender(UPOWER_DESTINATION)
        .unwrap()
        .member("PropertiesChanged")
        .unwrap()
        .interface("org.freedesktop.DBus.Properties")
        .unwrap()
        .path_namespace(DEVICES_PATH_PREFIX)
        .unwrap()
        .msg_type(Type::Signal)
        .build();
    let mut changed = match MessageStream::for_match_rule(changed_rule, &conn, None).await {
        Ok(stream) => stream,
        Err(error) => {
            warn!(%error, "upower: cannot subscribe to PropertiesChanged");
            return;
        }
    };

    let mut last: Vec<device::Battery> = Vec::new();
    refresh(&conn, &tx, &mut last).await;
    loop {
        tokio::select! {
            _ = added.next() => {}
            _ = removed.next() => {}
            message = changed.next() => {
                let Some(message) = message else { continue };
                let Ok(message) = message else { continue };
                let Ok((iface, _, _)) = message
                    .body()
                    .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
                else {
                    continue;
                };
                if iface != DEVICE_INTERFACE {
                    continue;
                }
                let header = message.header();
                let Some(path) = header.path() else { continue };
                let path = path.to_string();
                if let Some(battery) = device::fetch(&conn, &path).await.ok().flatten() {
                    emit_if_changed(&tx, &mut last, battery);
                }
                continue;
            }
        }
        refresh(&conn, &tx, &mut last).await;
    }
}

async fn refresh(conn: &Connection, tx: &Sender<UpowerEvent>, last: &mut Vec<device::Battery>) {
    let paths = match device::paths(conn).await {
        Ok(paths) => paths,
        Err(error) => {
            warn!(%error, "upower: refresh failed");
            return;
        }
    };
    for path in &paths {
        if let Some(battery) = device::fetch(conn, path).await.ok().flatten() {
            emit_if_changed(tx, last, battery);
        }
    }
    last.retain(|battery| paths.contains(&battery.path));
    for path in &paths {
        if !last.iter().any(|battery| battery.path == *path) {
            let _ = tx.send(UpowerEvent::DeviceAdded(path.clone()));
        }
    }
}

fn emit_if_changed(
    tx: &Sender<UpowerEvent>,
    last: &mut Vec<device::Battery>,
    battery: device::Battery,
) {
    if let Some(previous) = last.iter_mut().find(|entry| entry.path == battery.path) {
        if previous == &battery {
            return;
        }
        *previous = battery.clone();
    } else {
        last.push(battery.clone());
    }
    let _ = tx.send(UpowerEvent::BatteryChanged(battery));
}
