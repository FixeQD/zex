use anyhow::Result;
use zex_core::{Settings, SettingsStore};

mod app;
mod services_bridge;
mod widgets;
mod windows;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let settings = SettingsStore::load()
        .map(|s| s.get().clone())
        .unwrap_or_default();

    let _theme = zex_core::theme::theme_from_settings(&settings);

    // Skeleton: real iced_exwlshell bootstrap will be wired in Commit 4 (multi_window)
    // For Commit 3 we only ensure State/Message/Services compile and can be instantiated.
    let (service_handles, _event_rx) = services_bridge::spawn_all_services(settings.clone());
    let (_tx, rx) = flume::unbounded();
    let _state = app::State::new(settings, _theme, service_handles, rx);

    println!("zex-shell skeleton (Commit 3) — State/Message/services wiring OK");
    Ok(())
}
