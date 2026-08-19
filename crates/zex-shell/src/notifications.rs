//! Notification daemon host shared by the popup overlay and the quick center.
//!
//! The `org.freedesktop.Notifications` service lives on a background tokio
//! runtime (like the lock service); its event stream fans out to every overlay
//! through a [`Fan`] and the GTK components act through the sync
//! [`NotificationClient`] facade.

use std::sync::{Arc, Mutex};

use flume::Receiver;
use zex_core::SettingsStore;
use zex_services::notifications::broadcast;
use zex_services::notifications::{
    Fan, NotificationClient, NotificationEvent, Notifications, NotificationsConfig,
};

/// Compiled daemon config for a settings snapshot
fn config_from(settings: &zex_core::Settings) -> NotificationsConfig {
    let notifications = &settings.services.notifications;
    NotificationsConfig {
        timeout_ms: notifications.timeout_ms,
        max_popups: notifications.max_popups,
        history_size: notifications.history_size,
        dnd: notifications.dnd,
    }
}

pub struct NotificationsHub {
    fan: Arc<broadcast::Fan<NotificationEvent>>,
    client: Mutex<Option<NotificationClient>>,
    config: Mutex<NotificationsConfig>,
}

impl NotificationsHub {
    /// Hub with the daemon still offline; consumers subscribe before [`Self::connect`]
    pub fn new(settings: &zex_core::Settings) -> Arc<Self> {
        Arc::new(Self {
            fan: Arc::new(Fan::new()),
            client: Mutex::new(None),
            config: Mutex::new(config_from(settings)),
        })
    }

    /// Subscribe to every daemon event; the subscription dies with the receiver
    pub fn subscribe(&self) -> Receiver<NotificationEvent> {
        self.fan.subscribe()
    }

    /// Client for sync reads and daemon commands; `None` until the bus is up
    pub fn client(&self) -> Option<NotificationClient> {
        self.client.lock().unwrap().clone()
    }

    /// Push the settings snapshot into the daemon when the config group moved
    pub fn on_settings(&self, settings: &zex_core::Settings) {
        let updated = config_from(settings);
        let mut config = self.config.lock().unwrap();
        if *config == updated {
            return;
        }
        *config = updated;
        if let Some(client) = self.client.lock().unwrap().as_ref() {
            client.apply_config(updated);
        }
    }

    /// Bring the daemon up on a background runtime.
    /// Events start flowing to subscribers once the bus name is owned.
    pub fn connect(self: &Arc<Self>, store: SettingsStore) {
        let hub = Arc::clone(self);
        std::thread::Builder::new()
            .name("zex-notifications-daemon".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        tracing::warn!("notifications runtime unavailable: {err}");
                        return;
                    }
                };
                runtime.block_on(async {
                    let Ok(connection) = zbus::Connection::session().await else {
                        tracing::warn!("session bus unavailable; notifications daemon stays off");
                        return;
                    };
                    match Notifications::connect(connection, config_from(store.get())).await {
                        Ok(service) => {
                            *hub.client.lock().unwrap() = Some(service.client());
                            let events = service.events().clone();
                            let fan = Arc::clone(&hub.fan);
                            std::thread::spawn(move || {
                                while let Ok(event) = events.recv() {
                                    fan.push(&event);
                                }
                            });
                            tracing::info!("notifications daemon on the session bus");
                            std::future::pending::<()>().await
                        }
                        Err(err) => {
                            tracing::warn!("notification daemon unavailable: {err:#}");
                        }
                    }
                });
            })
            .expect("failed to spawn notifications daemon thread");
    }
}