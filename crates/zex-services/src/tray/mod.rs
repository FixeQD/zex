//! StatusNotifier system tray: watcher service, item client and dbusmenu

mod item;
mod menu;
mod monitor;
mod watcher;

pub use item::{TrayIcon, TrayItem, TrayPixmap};
pub use menu::{MenuEntry, parse_layout};
pub use watcher::StatusNotifierWatcher;

use anyhow::{Context, Result};
use flume::Receiver;
use std::sync::Arc;
use tokio::task::JoinHandle;
use zbus::Connection;
use zbus::fdo::RequestNameFlags;
use zbus::zvariant::OwnedValue;

pub const WATCHER_DESTINATION: &str = "org.kde.StatusNotifierWatcher";
pub const WATCHER_PATH: &str = "/StatusNotifierWatcher";
pub const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";
pub const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
pub const ITEM_DEFAULT_PATH: &str = "/StatusNotifierItem";
pub const HOST_PREFIX: &str = "org.kde.StatusNotifierHost-";
pub const DBUSMENU_INTERFACE: &str = "com.canonical.dbusmenu";

#[derive(Debug, Clone, PartialEq)]
pub enum TrayEvent {
    ItemAdded(TrayItem),
    ItemRemoved(String),
    ItemChanged(String, TrayIcon),
}

pub struct SystemTray {
    conn: Connection,
    events: Receiver<TrayEvent>,
    _task: JoinHandle<()>,
}

impl SystemTray {
    pub async fn host(conn: Connection) -> Result<Self> {
        let (tx, rx) = flume::unbounded();
        let state = Arc::new(watcher::WatcherState::default());
        conn.object_server()
            .at(
                WATCHER_PATH,
                StatusNotifierWatcher::new(conn.clone(), state.clone(), tx.clone()),
            )
            .await
            .context("serve watcher")?;
        conn.request_name_with_flags(
            WATCHER_DESTINATION,
            RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement,
        )
        .await
        .context("request watcher name")?;
        conn.request_name_with_flags(
            format!("{HOST_PREFIX}{}", std::process::id()),
            RequestNameFlags::DoNotQueue.into(),
        )
        .await
        .context("request host name")?;

        let task = tokio::spawn(monitor::run(conn.clone(), tx, state));
        Ok(Self {
            conn,
            events: rx,
            _task: task,
        })
    }

    pub fn events(&self) -> &Receiver<TrayEvent> {
        &self.events
    }

    /// Fresh query of the currently registered items
    pub async fn items(&self) -> Result<Vec<TrayItem>> {
        let services = self
            .conn
            .object_server()
            .interface::<_, StatusNotifierWatcher>(WATCHER_PATH)
            .await?
            .get()
            .await
            .registered_status_notifier_items();
        let mut items = Vec::new();
        for service in services {
            if let Some(item) = item::fetch_item(&self.conn, &service, ITEM_DEFAULT_PATH).await? {
                items.push(item);
            }
        }
        Ok(items)
    }

    async fn item_proxy(&self, service: &str) -> Result<zbus::Proxy<'static>> {
        zbus::proxy::Builder::new(&self.conn)
            .destination(service.to_string())?
            .path(ITEM_DEFAULT_PATH)?
            .interface(ITEM_INTERFACE)?
            .build()
            .await
            .context("item proxy")
    }

    /// Left-click an item
    pub async fn activate(&self, service: &str, x: i32, y: i32) -> Result<()> {
        self.item_proxy(service)
            .await?
            .call_noreply("Activate", &(x, y))
            .await
            .context("Activate")
    }

    /// Ask the item to pop up its context menu
    pub async fn context_menu(&self, service: &str, x: i32, y: i32) -> Result<()> {
        self.item_proxy(service)
            .await?
            .call_noreply("ContextMenu", &(x, y))
            .await
            .context("ContextMenu")
    }

    /// Fetch the item's dbusmenu tree for a custom context menu
    pub async fn menu(&self, service: &str) -> Result<Vec<MenuEntry>> {
        let item = item::fetch_item(&self.conn, service, ITEM_DEFAULT_PATH)
            .await?
            .ok_or_else(|| anyhow::anyhow!("item {service} is gone"))?;
        let Some(menu_path) = item.menu else {
            return Ok(Vec::new());
        };
        let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(&self.conn)
            .destination(service)?
            .path(menu_path)?
            .interface(DBUSMENU_INTERFACE)?
            .build()
            .await
            .context("dbusmenu proxy")?;
        let (_revision, layout): (i32, OwnedValue) = proxy
            .call("GetLayout", &(0i32, -1i32, Vec::<String>::new()))
            .await?;
        Ok(parse_layout(&layout))
    }

    /// Trigger a dbusmenu item
    pub async fn menu_action(&self, service: &str, id: i32) -> Result<()> {
        let item = item::fetch_item(&self.conn, service, ITEM_DEFAULT_PATH)
            .await?
            .ok_or_else(|| anyhow::anyhow!("item {service} is gone"))?;
        let Some(menu_path) = item.menu else {
            return Ok(());
        };
        let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(&self.conn)
            .destination(service)?
            .path(menu_path)?
            .interface(DBUSMENU_INTERFACE)?
            .build()
            .await
            .context("dbusmenu proxy")?;
        let body = (id, "clicked", zbus::zvariant::Value::from(""), 0u32);
        proxy
            .call_noreply("Event", &body)
            .await
            .context("dbusmenu event")
    }
}
