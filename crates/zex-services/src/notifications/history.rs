//! Notification history ring buffer and relative age labels

use super::Notification;
use std::collections::VecDeque;

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

    pub fn get(&self, id: u32) -> Option<&Notification> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Notification> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    /// Remove an entry by id, returning it if it existed
    pub fn remove(&mut self, id: u32) -> Option<Notification> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        self.entries.remove(index)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entries currently shown as popups
    pub fn popup_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.popup).count()
    }

    /// Id of the oldest popup, if any
    pub fn oldest_popup(&self) -> Option<u32> {
        self.entries
            .iter()
            .find(|entry| entry.popup)
            .map(|entry| entry.id)
    }

    /// Mark the given entry as no longer shown as a popup
    pub fn dismiss(&mut self, id: u32) {
        if let Some(notification) = self.get_mut(id) {
            notification.popup = false;
        }
    }
}

/// Relative age label ("now", "5m", "3h", "2d") used by popups and the center
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
