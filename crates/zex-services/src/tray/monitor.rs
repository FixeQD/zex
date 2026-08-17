//! Background watcher: item lifecycle and icon/status changes

use super::{HOST_PREFIX, ITEM_DEFAULT_PATH, TrayEvent, WATCHER_PATH, item, watcher};
use flume::Sender;
use futures_util::StreamExt;
use futures_util::stream::{BoxStream, SelectAll};
use std::sync::Arc;
use tracing::warn;
use watcher::{StatusNotifierWatcher, StatusNotifierWatcherSignals};
use zbus::match_rule::MatchRule;
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MessageStream};

pub async fn run(conn: Connection, tx: Sender<TrayEvent>, state: Arc<watcher::WatcherState>) {
    let rule = MatchRule::builder()
        .sender("org.freedesktop.DBus")
        .unwrap()
        .member("NameOwnerChanged")
        .unwrap()
        .interface("org.freedesktop.DBus")
        .unwrap()
        .msg_type(Type::Signal)
        .build();
    let mut names = match MessageStream::for_match_rule(rule, &conn, None).await {
        Ok(stream) => stream,
        Err(error) => {
            warn!(%error, "tray: cannot subscribe to NameOwnerChanged");
            return;
        }
    };

    let mut watches: Vec<(String, zbus::Proxy<'static>)> = Vec::new();
    loop {
        let mut select: SelectAll<BoxStream<'static, usize>> = SelectAll::new();
        for (index, (_, proxy)) in watches.iter().enumerate() {
            let icon_stream = proxy
                .receive_property_changed::<OwnedValue>("IconName")
                .await;
            let status_stream = proxy.receive_property_changed::<OwnedValue>("Status").await;
            select.push(
                futures_util::stream::select(icon_stream, status_stream)
                    .map(move |_| index)
                    .boxed(),
            );
        }
        tokio::select! {
            Some(message) = names.next() => {
                let Ok(message) = message else { continue };
                let Ok((name, _old, new)) = message
                    .body()
                    .deserialize::<(String, String, String)>()
                else {
                    continue;
                };
                if name.starts_with(HOST_PREFIX) {
                    *state.host_registered.lock().unwrap() = !new.is_empty();
                    continue;
                }
                if !name.starts_with("org.kde.StatusNotifierItem") {
                    continue;
                }
                let service = name;
                if new.is_empty() {
                    watches.retain(|(watched, _)| watched != &service);
                    {
                        let mut items = state.items.lock().unwrap();
                        if let Some(position) = items.iter().position(|item| item == &service) {
                            items.remove(position);
                        }
                    }
                    let iface_ref = conn
                        .object_server()
                        .interface::<_, StatusNotifierWatcher>(WATCHER_PATH)
                        .await;
                    if let Ok(iface_ref) = iface_ref {
                        let _ = StatusNotifierWatcherSignals::status_notifier_item_unregistered(
                            &iface_ref,
                            &service,
                        )
                        .await;
                    }
                    let _ = tx.send(TrayEvent::ItemRemoved(service));
                    continue;
                }
                if !state.items.lock().unwrap().contains(&service) {
                    continue;
                }
                if let Some(watch) = build_item_watch(&conn, &service).await {
                    watches.push(watch);
                }
                if let Ok(Some(item)) = item::fetch_item(&conn, &service, ITEM_DEFAULT_PATH).await {
                    let _ = tx.send(TrayEvent::ItemAdded(item));
                }
            }
            Some(index) = select.next() => {
                let Some((service, _)) = watches.get(index) else { continue };
                let service = service.clone();
                if let Ok(Some(item)) = item::fetch_item(&conn, &service, ITEM_DEFAULT_PATH).await {
                    let _ = tx.send(TrayEvent::ItemChanged(service, item.icon));
                }
            }
        }
    }
}

async fn build_item_watch(
    conn: &Connection,
    service: &str,
) -> Option<(String, zbus::Proxy<'static>)> {
    let proxy: zbus::Proxy<'static> = zbus::proxy::Builder::new(conn)
        .destination(service.to_string())
        .ok()?
        .path(ITEM_DEFAULT_PATH)
        .ok()?
        .interface(super::ITEM_INTERFACE)
        .ok()?
        .build()
        .await
        .ok()?;
    Some((service.to_string(), proxy))
}
