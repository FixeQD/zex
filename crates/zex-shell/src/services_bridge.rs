use flume::Receiver;
use zex_core::Settings;

use crate::app::{ServiceEvent, ServiceHandles};

pub fn spawn_all_services(_settings: Settings) -> (ServiceHandles, Receiver<ServiceEvent>) {
    let (event_tx, event_rx) = flume::unbounded();
    let (recorder_tx, _recorder_rx) = flume::unbounded();

    let _ = event_tx;

    let handles = ServiceHandles { recorder_tx };
    (handles, event_rx)
}

/// Subscription that forwards shell events into `Message::ShellEvent`.
/// Real implementation will listen on `iced_wayland_subscriber::shell` once the published crate exposes it; for now keep it as a no-op to stay buildable.
pub fn shell_subscription() -> iced::Subscription<()> {
    iced::Subscription::none()
}
