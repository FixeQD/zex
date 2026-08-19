use std::time::Duration;

use super::super::model::types::{Notification, NotificationsConfig};

/// Commands executed on the daemon runtime
pub(crate) enum ClientCommand {
    Close(u32),
    CloseAll,
    InvokeAction(u32, String),
    Snapshot(flume::Sender<Vec<Notification>>),
    Dnd(flume::Sender<bool>),
    SetDnd(bool),
    ApplyConfig(NotificationsConfig),
}

/// Thread-safe facade over the daemon.
/// Sync reads answer through a reply channel; bus writes run on the daemon runtime.
#[derive(Clone)]
pub struct NotificationClient {
    pub(crate) commands: flume::Sender<ClientCommand>,
}

impl NotificationClient {
    pub fn close(&self, id: u32) {
        let _ = self.commands.send(ClientCommand::Close(id));
    }

    pub fn close_all(&self) {
        let _ = self.commands.send(ClientCommand::CloseAll);
    }

    pub fn invoke_action(&self, id: u32, key: &str) {
        let _ = self
            .commands
            .send(ClientCommand::InvokeAction(id, key.to_owned()));
    }

    pub fn set_dnd(&self, dnd: bool) {
        let _ = self.commands.send(ClientCommand::SetDnd(dnd));
    }

    pub fn apply_config(&self, config: NotificationsConfig) {
        let _ = self.commands.send(ClientCommand::ApplyConfig(config));
    }

    /// History snapshot, newest first
    pub fn notifications(&self) -> Vec<Notification> {
        let (reply, rx) = flume::unbounded();
        if self.commands.send(ClientCommand::Snapshot(reply)).is_err() {
            return Vec::new();
        }
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(notifications) => notifications.into_iter().rev().collect(),
            Err(err) => {
                tracing::warn!("notification snapshot reply unavailable: {err}");
                Vec::new()
            }
        }
    }

    pub fn dnd(&self) -> bool {
        let (reply, rx) = flume::unbounded();
        if self.commands.send(ClientCommand::Dnd(reply)).is_err() {
            return false;
        }
        rx.recv_timeout(Duration::from_secs(2)).unwrap_or(false)
    }
}