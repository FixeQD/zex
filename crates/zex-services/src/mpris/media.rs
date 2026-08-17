//! Player media metadata model

use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Playing,
    Paused,
    Stopped,
}

impl From<&str> for PlaybackStatus {
    fn from(value: &str) -> Self {
        match value {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MediaInfo {
    pub title: String,
    pub artist: Vec<String>,
    pub album: String,
    pub art_url: Option<String>,
    pub length_us: Option<i64>,
    pub playback_status: PlaybackStatus,
}

impl MediaInfo {
    pub fn from_metadata(metadata: &HashMap<String, OwnedValue>, status: PlaybackStatus) -> Self {
        let title = metadata
            .get("xesam:title")
            .and_then(|value| value.downcast_ref::<&str>().ok())
            .unwrap_or_default()
            .to_string();
        let artist = metadata
            .get("xesam:artist")
            .and_then(|value| {
                Vec::<String>::try_from(zbus::zvariant::Value::from(value.clone())).ok()
            })
            .unwrap_or_default();
        let album = metadata
            .get("xesam:album")
            .and_then(|value| value.downcast_ref::<&str>().ok())
            .unwrap_or_default()
            .to_string();
        let art_url = metadata
            .get("mpris:artUrl")
            .and_then(|value| value.downcast_ref::<&str>().ok())
            .map(str::to_string);
        let length_us = metadata
            .get("mpris:length")
            .and_then(|value| value.downcast_ref::<&i64>().ok())
            .copied();
        Self {
            title,
            artist,
            album,
            art_url,
            length_us,
            playback_status: status,
        }
    }

    /// Compact display label: "Artist - Title", or just the title
    pub fn label(&self) -> String {
        if self.title.is_empty() {
            return String::new();
        }
        let artist = self.artist.join(", ");
        if artist.is_empty() {
            self.title.clone()
        } else {
            format!("{artist} - {}", self.title)
        }
    }
}
