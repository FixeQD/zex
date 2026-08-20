use std::collections::HashMap;
use std::sync::Arc;

use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use super::engine::core::Core;

pub struct NotificationsServer {
    pub core: Arc<Core>,
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
