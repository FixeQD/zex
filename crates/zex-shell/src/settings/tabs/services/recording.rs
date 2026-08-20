//! Services tab recording category

use gtk4::prelude::*;
use zex_core::Settings;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::{
    IndependentItem, category, independent_toggle_buttons, separator, settings_row, switch_row,
};

pub fn recording_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Recording");

    let snapshot = ctx.snapshot();
    let recorder = &snapshot.services.recorder;

    let notifications = independent_toggle_buttons(
        ctx,
        vec![
            IndependentItem {
                label: Some("Started"),
                icon: Some("media-playback-start-symbolic"),
                get: Box::new(|s: &Settings| s.services.recorder.start_notification),
                set: Box::new(|s: &mut Settings, active| {
                    s.services.recorder.start_notification = active
                }),
            },
            IndependentItem {
                label: Some("Stopped"),
                icon: Some("media-playback-stop-symbolic"),
                get: Box::new(|s: &Settings| s.services.recorder.stop_notification),
                set: Box::new(|s: &mut Settings, active| {
                    s.services.recorder.stop_notification = active
                }),
            },
        ],
    );
    let notifications_row = settings_row(
        Some("Notifications"),
        Some("When should the recorder send a notification."),
        false,
    );
    notifications_row.append(&notifications.container);
    container.append(&notifications_row);
    container.append(&separator());

    container.append(&switch_row(
        ctx,
        Some("Record Audio"),
        Some("Record the system's audio when recording."),
        recorder.record_audio,
        |s: &mut Settings, active| s.services.recorder.record_audio = active,
    ));

    container
}
