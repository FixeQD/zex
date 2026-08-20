//! Notification daemon: `org.freedesktop.Notifications` server with DND and history

pub mod broadcast;
mod daemon;

pub use broadcast::Fan;
pub use daemon::client::NotificationClient;
pub use daemon::history::relative_age;
pub use daemon::runtime::service::Notifications;
pub use daemon::types::{
    Notification, NotificationAction, NotificationEvent, NotificationsConfig, Urgency,
};
pub use daemon::{BUS_NAME, OBJECT_PATH};
