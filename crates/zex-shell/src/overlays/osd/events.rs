//! OSD event sources: PipeWire volume events and the sysfs backlight poll.

use std::sync::{Arc, Mutex};

use relm4::prelude::*;
use zex_services::audio::volume::{VolumeState, spawn_volume_monitor};
use zex_services::backlight::Backlight;

use super::Osd;
use super::OsdMsg;

/// Sysfs poll rate: no event sink exists, so changes are sampled
const BACKLIGHT_POLL: std::time::Duration = std::time::Duration::from_millis(300);

pub(super) fn spawn_volume_events(out: ComponentSender<Osd>) {
    let (volume_events, volume_event_rx) = flume::unbounded();
    spawn_volume_monitor(
        Arc::new(Mutex::new(VolumeState::default())),
        tokio::sync::oneshot::channel().0,
        volume_events,
    );
    std::thread::spawn(move || {
        while let Ok(state) = volume_event_rx.recv() {
            out.input(OsdMsg::Volume(state));
        }
    });
}

pub(super) fn spawn_backlight_events(out: ComponentSender<Osd>) {
    std::thread::spawn(move || {
        let Some(backlight) = Backlight::detect() else {
            tracing::warn!("no backlight device detected; brightness OSD stays off");
            return;
        };
        let mut last: Option<f32> = None;
        loop {
            let value = backlight
                .percent()
                .ok()
                .map(|percent| (percent / 100.0).clamp(0.0, 1.0));
            if value != last {
                last = value;
                if let Some(value) = value {
                    out.input(OsdMsg::Backlight(value));
                }
            }
            std::thread::sleep(BACKLIGHT_POLL);
        }
    });
}
