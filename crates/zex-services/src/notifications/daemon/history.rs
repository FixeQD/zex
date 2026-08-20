use std::collections::VecDeque;

use super::types::Notification;

pub struct History {
    entries: VecDeque<Notification>,
    cap: usize,
}

impl History {
    pub fn new(cap: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    pub fn push(&mut self, notification: Notification) -> Option<Notification> {
        self.entries.push_back(notification);
        if self.entries.len() > self.cap {
            self.entries.pop_front()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Notification> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    pub fn remove(&mut self, id: u32) -> Option<Notification> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        self.entries.remove(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.entries.iter()
    }

    /// Entries currently shown as popups
    pub fn popup_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.popup).count()
    }

    pub fn oldest_popup(&self) -> Option<u32> {
        self.entries
            .iter()
            .find(|entry| entry.popup)
            .map(|entry| entry.id)
    }

    /// Keep the entry in the history but stop showing it as a popup
    pub fn dismiss(&mut self, id: u32) {
        if let Some(notification) = self.get_mut(id) {
            notification.popup = false;
        }
    }
}

/// Relative age label ("now", "5m", "3h", "2d")
pub fn relative_age(now: i64, time: i64) -> String {
    let diff = (now - time).max(0);
    if diff < 60 {
        return "now".to_string();
    }
    if diff < 3600 {
        return format!("{}m", diff / 60);
    }
    if diff < 86400 {
        return format!("{}h", diff / 3600);
    }
    format!("{}d", diff / 86400)
}
