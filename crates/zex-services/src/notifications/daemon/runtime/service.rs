use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use flume::Receiver;
use tokio::task::JoinHandle;
use zbus::Connection;
use zbus::names::WellKnownName;

use super::super::OBJECT_PATH;
use super::super::client::NotificationClient;
use super::super::engine::core::Core;
use super::super::server::NotificationsServer;
use super::super::types::{Notification, NotificationEvent, NotificationsConfig};
use super::commands;

pub struct Notifications {
    events: Receiver<NotificationEvent>,
    core: Arc<Core>,
    client: NotificationClient,
    _tasks: Vec<JoinHandle<()>>,
}

impl Notifications {
    pub async fn connect(conn: Connection, config: NotificationsConfig) -> Result<Self> {
        let (tx, rx) = flume::unbounded();
        let core = Arc::new(Core::new(conn.clone(), config, tx.clone()));

        let name = WellKnownName::try_from(super::super::BUS_NAME).context("invalid bus name")?;
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
                "another notification daemon owns {}",
                super::super::BUS_NAME
            ));
        }
        conn.object_server()
            .at(OBJECT_PATH, NotificationsServer { core: core.clone() })
            .await
            .context("register Notifications interface")?;

        let timeout_core = core.clone();
        let timeout_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                timeout_core.dismiss_due();
            }
        });
        let age_tx = tx;
        let age_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let _ = age_tx.send(NotificationEvent::AgeTick);
            }
        });

        let (commands_tx, command_rx) = flume::unbounded();
        let command_core = core.clone();
        let command_task = commands::spawn(command_rx, command_core);

        Ok(Self {
            events: rx,
            core,
            client: NotificationClient {
                commands: commands_tx,
            },
            _tasks: vec![timeout_task, age_task, command_task],
        })
    }

    pub fn events(&self) -> &Receiver<NotificationEvent> {
        &self.events
    }

    pub fn client(&self) -> NotificationClient {
        self.client.clone()
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
        self.core.close_all().await.context("close notifications")
    }

    pub async fn invoke_action(&self, id: u32, key: &str) -> Result<()> {
        self.core
            .invoke_action(id, key)
            .await
            .context("invoke action")
    }
}
