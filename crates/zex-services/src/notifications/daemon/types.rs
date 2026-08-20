#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

/// `urgency` hint values (0–2)
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
