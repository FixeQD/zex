//! Tray item model and D-Bus property fetching.

use super::ITEM_INTERFACE;
use anyhow::Result;
use zbus::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayPixmap {
    pub width: i32,
    pub height: i32,
    /// Raw RGBA bytes
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrayIcon {
    pub name: Option<String>,
    pub pixmap: Vec<TrayPixmap>,
}

impl TrayIcon {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.pixmap.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrayItem {
    pub service: String,
    pub id: String,
    pub title: String,
    pub status: String,
    pub icon: TrayIcon,
    pub menu: Option<String>,
}

pub async fn fetch_item(conn: &Connection, service: &str, path: &str) -> Result<Option<TrayItem>> {
    let proxy: zbus::Proxy<'_> = match zbus::proxy::Builder::new(conn)
        .destination(service)?
        .path(path)?
        .interface(ITEM_INTERFACE)?
        .build()
        .await
    {
        Ok(proxy) => proxy,
        Err(_) => return Ok(None),
    };
    let id = proxy.get_property::<String>("Id").await.unwrap_or_default();
    let title = proxy
        .get_property::<String>("Title")
        .await
        .unwrap_or_default();
    let status = proxy
        .get_property::<String>("Status")
        .await
        .unwrap_or_else(|_| "passive".into());
    let icon = fetch_icon(&proxy).await;
    let menu = proxy
        .get_property::<zbus::zvariant::OwnedObjectPath>("Menu")
        .await
        .ok()
        .map(|path| path.to_string());
    Ok(Some(TrayItem {
        service: service.to_string(),
        id,
        title,
        status,
        icon,
        menu,
    }))
}

async fn fetch_icon(proxy: &zbus::Proxy<'_>) -> TrayIcon {
    let name = proxy
        .get_property::<String>("IconName")
        .await
        .ok()
        .filter(|name| !name.is_empty());
    let pixmap = proxy
        .get_property::<Vec<(i32, i32, Vec<u8>)>>("IconPixmap")
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(width, height, data)| TrayPixmap {
            width,
            height,
            data,
        })
        .collect();
    TrayIcon { name, pixmap }
}
