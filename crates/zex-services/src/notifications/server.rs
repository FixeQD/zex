//! The `org.freedesktop.Notifications` D-Bus service implementation

use super::history::History;
use super::{
    Notification, NotificationAction, NotificationEvent, NotificationsConfig, OBJECT_PATH, Urgency,
};
use flume::Sender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zbus::Connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

/// Shared daemon state
pub struct State {
    pub(crate) history: History,
    pub(crate) next_id: u32,
    pub(crate) dnd: bool,
    pub(crate) config: NotificationsConfig,
}

/// Engine shared by the DBus interface and the public service API
pub struct Core {
    pub(crate) conn: Connection,
    pub(crate) state: Arc<Mutex<State>>,
    pub(crate) tx: Sender<NotificationEvent>,
}

impl Core {
    pub fn new(
        conn: Connection,
        config: NotificationsConfig,
        tx: Sender<NotificationEvent>,
    ) -> Self {
        let state = Arc::new(Mutex::new(State {
            history: History::new(config.history_size),
            next_id: 1,
            dnd: config.dnd,
            config,
        }));
        Self { conn, state, tx }
    }

    /// Process a `Notify` call: validation, timeout resolution, popup limit, history insertion
    /// Returns the notification id (0 = rejected)
    pub async fn add(
        &self,
        app_name: String,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
        replaces_id: u32,
    ) -> zbus::fdo::Result<u32> {
        if summary.is_empty() && body.is_empty() {
            return Ok(0);
        }
        let actions = actions
            .chunks(2)
            .filter(|chunk| chunk.len() == 2)
            .map(|chunk| NotificationAction {
                key: chunk[0].clone(),
                label: chunk[1].clone(),
            })
            .collect::<Vec<_>>();
        let urgency = hints
            .get("urgency")
            .and_then(|value| value.downcast_ref::<u8>().ok())
            .map(Urgency::from_u8)
            .unwrap_or(Urgency::Normal);
        let resident = hints
            .get("resident")
            .and_then(|value| value.downcast_ref::<bool>().ok())
            .unwrap_or(false);
        let app_icon = if app_icon.is_empty() {
            hints
                .get("image-path")
                .and_then(|value| value.downcast_ref::<&str>().ok())
                .unwrap_or("")
                .to_string()
        } else {
            app_icon
        };
        let timeout_ms = if resident || expire_timeout == 0 {
            0
        } else if expire_timeout > 0 {
            i64::from(expire_timeout)
        } else {
            self.state.lock().unwrap().config.timeout_ms
        };

        let mut closed_signals: Vec<(u32, u32)> = Vec::new();
        let mut events = Vec::new();
        let id;
        {
            let mut state = self.state.lock().unwrap();
            if replaces_id != 0 {
                if state.history.remove(replaces_id).is_some() {
                    closed_signals.push((replaces_id, 2));
                    events.push(NotificationEvent::Closed(replaces_id));
                }
                id = replaces_id;
                state.next_id = state.next_id.max(replaces_id + 1);
            } else {
                id = state.next_id;
                state.next_id += 1;
            }
            let notification = Notification {
                id,
                app_name,
                app_icon,
                summary,
                body,
                actions,
                urgency,
                timeout_ms,
                time: now_secs(),
                popup: !state.dnd,
            };
            if notification.popup && state.config.max_popups > 0 {
                while state.history.popup_count() >= state.config.max_popups {
                    match state.history.oldest_popup() {
                        Some(oldest) => {
                            state.history.dismiss(oldest);
                            events.push(NotificationEvent::Dismissed(oldest));
                        }
                        None => break,
                    }
                }
            }
            if let Some(evicted) = state.history.push(notification.clone()) {
                if evicted.popup {
                    events.push(NotificationEvent::Dismissed(evicted.id));
                }
            }
            if notification.popup {
                events.push(NotificationEvent::Popup(notification.clone()));
            }
            events.push(NotificationEvent::Notified(notification));
        }
        for (id, reason) in closed_signals {
            self.emit_notification_closed(id, reason).await?;
        }
        for event in events {
            let _ = self.tx.send(event);
        }
        Ok(id)
    }

    /// Fully close a notification: remove it from the history, hide its popup and emit `NotificationClosed` (reason 2) on the bus
    pub async fn close(&self, id: u32) -> zbus::fdo::Result<()> {
        let removed = { self.state.lock().unwrap().history.remove(id).is_some() };
        if !removed {
            return Ok(());
        }
        self.emit_notification_closed(id, 2).await?;
        let _ = self.tx.send(NotificationEvent::Closed(id));
        Ok(())
    }

    /// Emit `ActionInvoked` for a notification action
    pub async fn invoke_action(&self, id: u32, key: &str) -> zbus::fdo::Result<()> {
        self.emit_action_invoked(id, key).await
    }

    pub fn set_dnd(&self, dnd: bool) {
        let mut state = self.state.lock().unwrap();
        if state.dnd == dnd {
            return;
        }
        state.dnd = dnd;
        let _ = self.tx.send(NotificationEvent::DndChanged(dnd));
    }

    pub fn dnd(&self) -> bool {
        self.state.lock().unwrap().dnd
    }

    pub fn notifications(&self) -> Vec<Notification> {
        self.state.lock().unwrap().history.iter().cloned().collect()
    }

    pub fn popups(&self) -> Vec<u32> {
        self.state
            .lock()
            .unwrap()
            .history
            .iter()
            .filter(|entry| entry.popup)
            .map(|entry| entry.id)
            .collect()
    }

    pub fn notification_ids(&self) -> Vec<u32> {
        self.state
            .lock()
            .unwrap()
            .history
            .iter()
            .map(|entry| entry.id)
            .collect()
    }

    /// Dismiss popups whose timeout elapsed
    /// Called periodically from the background task
    pub fn dismiss_due(&self) {
        let now = now_secs();
        let mut due = Vec::new();
        {
            let mut state = self.state.lock().unwrap();
            for entry in state.history.iter() {
                if entry.popup
                    && entry.timeout_ms > 0
                    && (now - entry.time) * 1000 >= entry.timeout_ms
                {
                    due.push(entry.id);
                }
            }
            for id in &due {
                state.history.dismiss(*id);
            }
        }
        for id in due {
            let _ = self.tx.send(NotificationEvent::Dismissed(id));
        }
    }

    async fn emit_notification_closed(&self, id: u32, reason: u32) -> zbus::fdo::Result<()> {
        let iface_ref = self
            .conn
            .object_server()
            .interface::<_, NotificationsServer>(OBJECT_PATH)
            .await?;
        NotificationsServerSignals::notification_closed(&iface_ref, id, reason)
            .await
            .map_err(zbus::fdo::Error::from)
    }

    async fn emit_action_invoked(&self, id: u32, key: &str) -> zbus::fdo::Result<()> {
        let iface_ref = self
            .conn
            .object_server()
            .interface::<_, NotificationsServer>(OBJECT_PATH)
            .await?;
        NotificationsServerSignals::action_invoked(&iface_ref, id, key.to_string())
            .await
            .map_err(zbus::fdo::Error::from)
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub struct NotificationsServer {
    pub(crate) core: Arc<Core>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationsServer {
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        self.core
            .add(
                app_name,
                app_icon,
                summary,
                body,
                actions,
                hints,
                expire_timeout,
                replaces_id,
            )
            .await
    }

    async fn close_notification(&self, id: u32) -> zbus::fdo::Result<()> {
        self.core.close(id).await
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "actions".to_string(),
            "body".to_string(),
            "icon-static".to_string(),
            "persistence".to_string(),
        ]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Zex Notifications".to_string(),
            "zex".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        )
    }

    #[zbus(signal)]
    async fn action_invoked(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn notification_closed(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
}
