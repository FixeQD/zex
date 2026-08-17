//! Player discovery and property fetching.

use super::{
    MEDIA_PLAYER2_INTERFACE, MPRIS_PREFIX, MediaInfo, PLAYER_INTERFACE, PLAYER_OBJECT_PATH,
    PlaybackStatus,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::proxy::Builder;
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone, PartialEq)]
pub struct MprisPlayer {
    pub name: String,
    pub identity: String,
    pub info: MediaInfo,
}

impl MprisPlayer {
    pub fn bus_name(&self) -> String {
        format!("{MPRIS_PREFIX}{}", self.name)
    }
}

pub async fn bus_names(conn: &Connection) -> Result<Vec<String>> {
    let dbus = DBusProxy::new(conn).await?;
    let names: Vec<zbus::names::OwnedBusName> = dbus.list_names().await.context("list_names")?;
    Ok(names
        .into_iter()
        .map(|name| name.to_string())
        .filter(|name| name.starts_with(MPRIS_PREFIX))
        .collect())
}

pub async fn fetch(conn: &Connection, name: &str) -> Result<Option<MprisPlayer>> {
    let identity: zbus::Proxy<'_> = Builder::new(conn)
        .destination(name)?
        .path(PLAYER_OBJECT_PATH)?
        .interface(MEDIA_PLAYER2_INTERFACE)?
        .build()
        .await?;
    let player: zbus::Proxy<'_> = Builder::new(conn)
        .destination(name)?
        .path(PLAYER_OBJECT_PATH)?
        .interface(PLAYER_INTERFACE)?
        .build()
        .await?;

    let (identity, status, metadata): (String, String, HashMap<String, OwnedValue>) = match (
        identity.get_property("Identity").await,
        player.get_property("PlaybackStatus").await,
        player.get_property("Metadata").await,
    ) {
        (Ok(identity), Ok(status), Ok(metadata)) => (identity, status, metadata),
        // The player went away between listing and fetching
        _ => return Ok(None),
    };

    Ok(Some(MprisPlayer {
        name: name.trim_start_matches(MPRIS_PREFIX).to_string(),
        identity,
        info: MediaInfo::from_metadata(&metadata, PlaybackStatus::from(status.as_str())),
    }))
}

pub struct PlayerWatch {
    pub name: String,
    pub proxy: zbus::Proxy<'static>,
}

pub async fn build_watch(conn: &Connection, bus_name: &str) -> Option<PlayerWatch> {
    let proxy: zbus::Proxy<'static> = Builder::new(conn)
        .destination(bus_name.to_string())
        .ok()?
        .path(PLAYER_OBJECT_PATH)
        .ok()?
        .interface(PLAYER_INTERFACE)
        .ok()?
        .build()
        .await
        .ok()?;
    Some(PlayerWatch {
        name: bus_name.trim_start_matches(MPRIS_PREFIX).to_string(),
        proxy,
    })
}
