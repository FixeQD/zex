//! Notification daemon: `org.freedesktop.Notifications` server with DND and history

mod history;
mod server;

pub use history::relative_age;

use anyhow::{Context, Result};
use flume::Receiver;
use std::sync::Arc;
use tokio::task::JoinHandle;
use zbus::Connection;
use zbus::names::WellKnownName;

pub const BUS_NAME: &str = "org.freedesktop.Notifications";
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

/// Notification urgency levels (hint `urgency`, 0–2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

impl Urgency {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::Critical,
            _ => Self::Normal,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// A single received notification
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<NotificationAction>,
    pub urgency: Urgency,
    pub timeout_ms: i64,
    pub time: i64,
    pub popup: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationEvent {
    Popup(Notification),
    Notified(Notification),
    Dismissed(u32),
    Closed(u32),
    DndChanged(bool),
    AgeTick,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationsConfig {
    pub timeout_ms: i64,
    pub max_popups: usize,
    pub history_size: usize,
    pub dnd: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            max_popups: 3,
            history_size: 100,
            dnd: false,
        }
    }
}

pub struct Notifications {
    events: Receiver<NotificationEvent>,
    core: Arc<server::Core>,
    _tasks: Vec<JoinHandle<()>>,
}

impl Notifications {
    pub async fn connect(conn: Connection, config: NotificationsConfig) -> Result<Self> {
        let (tx, rx) = flume::unbounded();
        let core = Arc::new(server::Core::new(conn.clone(), config, tx.clone()));

        let name = WellKnownName::try_from(BUS_NAME).context("invalid bus name")?;
        let reply = zbus::fdo::DBusProxy::new(&conn)
            .await?
            .request_name(name, zbus::fdo::RequestNameFlags::DoNotQueue.into())
            .await
            .context("request name")?;
        if !matches!(
            reply,
            zbus::fdo::RequestNameReply::PrimaryOwner | zbus::fdo::RequestNameReply::AlreadyOwner
        ) {
            return Err(anyhow::anyhow!(
                "another notification daemon owns {BUS_NAME}"
            ));
        }
        conn.object_server()
            .at(
                OBJECT_PATH,
                server::NotificationsServer { core: core.clone() },
            )
            .await
            .context("register Notifications interface")?;

        let timeout_core = core.clone();
        let timeout_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                timeout_core.dismiss_due();
            }
        });
        let age_tx = tx;
        let age_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let _ = age_tx.send(NotificationEvent::AgeTick);
            }
        });

        Ok(Self {
            events: rx,
            core,
            _tasks: vec![timeout_task, age_task],
        })
    }

    pub fn events(&self) -> &Receiver<NotificationEvent> {
        &self.events
    }

    pub fn dnd(&self) -> bool {
        self.core.dnd()
    }

    pub fn set_dnd(&self, dnd: bool) {
        self.core.set_dnd(dnd);
    }

    pub fn notifications(&self) -> Vec<Notification> {
        self.core.notifications()
    }

    pub fn popups(&self) -> Vec<u32> {
        self.core.popups()
    }

    pub async fn close(&self, id: u32) -> Result<()> {
        self.core.close(id).await.context("close notification")
    }

    pub async fn close_all(&self) -> Result<()> {
        let ids = self.core.notification_ids();
        for id in ids {
            self.core.close(id).await.context("close notification")?;
        }
        Ok(())
    }

    pub async fn invoke_action(&self, id: u32, key: &str) -> Result<()> {
        self.core
            .invoke_action(id, key)
            .await
            .context("invoke action")
    }
}
