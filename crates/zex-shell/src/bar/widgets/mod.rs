//! Service-backed bar widgets

pub mod battery;
pub mod icon;
pub mod media;
pub mod popover;
pub mod systeminfotray;
pub mod tasks;
pub mod window_info;
pub mod workspaces;

use std::rc::Rc;

/// Routes player commands to the MPRIS runtime thread (never the GTK thread)
#[derive(Clone)]
pub struct MprisControl {
    tx: flume::Sender<String>,
}

pub type SharedMpris = Rc<MprisControl>;

impl MprisControl {
    pub fn new(tx: flume::Sender<String>) -> Self {
        Self { tx }
    }

    pub fn play_pause(&self, player: &str) {
        let _ = self.tx.send(player.to_owned());
    }
}
