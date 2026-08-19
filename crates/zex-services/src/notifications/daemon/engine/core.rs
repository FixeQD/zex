use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use flume::Sender;
use zbus::zvariant::OwnedValue;

use super::super::model::types::{
    Notification, NotificationAction, NotificationEvent, NotificationsConfig, Urgency,
};
use super::super::store::history::History;
use super::signals;

enum Reason {
    /// `NotificationClosed` reason 2: user or server closed the notification
    Closed = 2,
}

/// Engine shared by the D-Bus interface and the public service API
pub struct Core {
    conn: zbus::Connection,
    state: Arc<Mutex<State>>,
    tx: Sender<NotificationEvent>,
}

pub struct State {
    history: History,
    next_id: u32,
    dnd: bool,
    config: NotificationsConfig,
}

impl Core {
    pub fn new(
        conn: zbus::Connection,
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
        // resident or 0 = sticky; negative = daemon default
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
                    closed_signals.push((replaces_id, Reason::Closed as u32));
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
            // Enforce max_popups by evicting the oldest popup
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
            signals::notification_closed(&self.conn, id, reason).await?;
        }
        for event in events {
            let _ = self.tx.send(event);
        }
        Ok(id)
    }

    pub async fn close(&self, id: u32) -> zbus::fdo::Result<()> {
        let removed = { self.state.lock().unwrap().history.remove(id).is_some() };
        if !removed {
            return Ok(());
        }
        signals::notification_closed(&self.conn, id, Reason::Closed as u32).await?;
        let _ = self.tx.send(NotificationEvent::Closed(id));
        Ok(())
    }

    pub async fn close_all(&self) -> zbus::fdo::Result<()> {
        let ids = self.notification_ids();
        for id in ids {
            self.close(id).await?;
        }
        Ok(())
    }

    pub async fn invoke_action(&self, id: u32, key: &str) -> zbus::fdo::Result<()> {
        signals::action_invoked(&self.conn, id, key).await
    }

    pub fn set_dnd(&self, dnd: bool) {
        let mut state = self.state.lock().unwrap();
        if state.dnd == dnd {
            return;
        }
        state.dnd = dnd;
        let _ = self.tx.send(NotificationEvent::DndChanged(dnd));
    }

    /// Takes effect on the next notification; emits `DndChanged` when the flag moved
    pub fn apply_config(&self, config: NotificationsConfig) {
        let mut state = self.state.lock().unwrap();
        if state.config.dnd != config.dnd {
            let _ = self.tx.send(NotificationEvent::DndChanged(config.dnd));
            state.dnd = config.dnd;
        }
        state.config = config;
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

    /// Dismiss popups whose timeout elapsed; polled from the background task
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
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}