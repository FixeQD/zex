//! Background monitor: player lifecycle and property changes.

use super::{MPRIS_PREFIX, MprisEvent, player};
use flume::Sender;
use futures_util::stream::{SelectAll, StreamExt};
use std::collections::HashMap;
use tracing::warn;
use zbus::match_rule::MatchRule;
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MessageStream};

async fn name_watcher(conn: Connection, tx: Sender<(String, bool)>) {
    let rule = MatchRule::builder()
        .sender("org.freedesktop.DBus")
        .unwrap()
        .member("NameOwnerChanged")
        .unwrap()
        .interface("org.freedesktop.DBus")
        .unwrap()
        .arg0ns(MPRIS_PREFIX.trim_end_matches('.'))
        .unwrap()
        .msg_type(Type::Signal)
        .build();
    let mut names = match MessageStream::for_match_rule(rule, &conn, None).await {
        Ok(stream) => stream,
        Err(error) => {
            warn!(%error, "mpris: cannot subscribe to NameOwnerChanged");
            return;
        }
    };
    while let Some(message) = names.next().await {
        let Ok(message) = message else { continue };
        let Ok((name, _old, new)) = message.body().deserialize::<(String, String, String)>() else {
            continue;
        };
        let _ = tx.send((name, !new.is_empty()));
    }
}

pub async fn run(conn: Connection, tx: Sender<MprisEvent>) {
    let (names_tx, names_rx) = flume::unbounded();
    tokio::spawn(name_watcher(conn.clone(), names_tx));

    for bus_name in player::bus_names(&conn).await.unwrap_or_default() {
        if let Ok(Some(player)) = player::fetch(&conn, &bus_name).await {
            let _ = tx.send(MprisEvent::PlayerAdded(player));
        }
    }

    let mut watches: Vec<player::PlayerWatch> = Vec::new();
    loop {
        let mut select = SelectAll::new();
        for (index, watch) in watches.iter().enumerate() {
            let status_stream = watch
                .proxy
                .receive_property_changed::<OwnedValue>("PlaybackStatus")
                .await;
            let metadata_stream = watch
                .proxy
                .receive_property_changed::<OwnedValue>("Metadata")
                .await;
            select.push(
                futures_util::stream::select(status_stream, metadata_stream)
                    .map(move |_| index)
                    .boxed(),
            );
        }
        tokio::select! {
            Ok((bus_name, appeared)) = names_rx.recv_async() => {
                let name = bus_name.trim_start_matches(MPRIS_PREFIX).to_string();
                if appeared {
                    if let Ok(Some(player)) = player::fetch(&conn, &bus_name).await {
                        if let Some(watch) = player::build_watch(&conn, &bus_name).await {
                            watches.push(watch);
                        }
                        let _ = tx.send(MprisEvent::PlayerAdded(player));
                    }
                } else {
                    watches.retain(|watch| watch.name != name);
                    let _ = tx.send(MprisEvent::PlayerRemoved(name));
                }
            }
            Some(index) = select.next() => {
                let Some(watch) = watches.get(index) else { continue };
                let (status, metadata) = match (
                    watch.proxy.get_property::<String>("PlaybackStatus").await,
                    watch.proxy.get_property::<HashMap<String, OwnedValue>>("Metadata").await,
                ) {
                    (Ok(status), Ok(metadata)) => (status, metadata),
                    _ => continue,
                };
                let info = super::MediaInfo::from_metadata(&metadata, super::PlaybackStatus::from(status.as_str()));
                let _ = tx.send(MprisEvent::PlayerChanged(watch.name.clone(), info));
            }
            else => {
                break;
            }
        }
    }
}
