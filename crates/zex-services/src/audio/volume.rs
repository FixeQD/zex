//! PipeWire volume state and control

pub use crate::audio::pod::{build_volume_pod, parse_volume_pod};

use crate::audio::monitor::run_volume_monitor;
use pipewire as pw;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct VolumeState {
    pub volume: f32,
    pub muted: bool,
}

impl Default for VolumeState {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
        }
    }
}

pub enum VolumeCommand {
    SetVolume(f32),
    ToggleMute,
}

#[derive(Clone)]
pub struct VolumeControl {
    sender: Arc<Mutex<Option<pw::channel::Sender<VolumeCommand>>>>,
}

impl VolumeControl {
    /// Set the sink volume, clamped to `[0.0, 1.5]`
    pub fn set_volume(&self, volume: f32) {
        let guard = self.sender.lock().unwrap_or_else(|e| e.into_inner());
        let Some(sender) = guard.as_ref() else {
            return;
        };
        let _ = sender.send(VolumeCommand::SetVolume(volume.clamp(0.0, 1.5)));
    }

    /// Toggle the sink mute state
    pub fn toggle_mute(&self) {
        let guard = self.sender.lock().unwrap_or_else(|e| e.into_inner());
        let Some(sender) = guard.as_ref() else {
            return;
        };
        let _ = sender.send(VolumeCommand::ToggleMute);
    }
}

/// Spawn the volume monitor on a background thread
pub fn spawn_volume_monitor(
    state: Arc<Mutex<VolumeState>>,
    ready_tx: oneshot::Sender<()>,
) -> VolumeControl {
    let sender = Arc::new(Mutex::new(None));
    let control = VolumeControl {
        sender: Arc::clone(&sender),
    };

    std::thread::Builder::new()
        .name("zex-pipewire-volume".to_string())
        .spawn(move || {
            let mut ready_tx = Some(ready_tx);
            loop {
                let (tx, rx) = pw::channel::channel::<VolumeCommand>();
                *sender.lock().unwrap() = Some(tx);
                if let Err(error) = run_volume_monitor(
                    Arc::clone(&state),
                    rx,
                    ready_tx.take().expect("ready channel already taken"),
                ) {
                    warn!("audio: volume monitor failed, retrying in 2s: {error}");
                }
                *sender.lock().unwrap() = None;
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        })
        .expect("failed to spawn volume monitor thread");

    control
}
