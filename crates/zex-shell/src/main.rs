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

    let theme = zex_core::theme::theme_from_settings(&settings);

    let (service_handles, _event_rx) = services_bridge::spawn_all_services(settings.clone());
    let (_tx, rx) = flume::unbounded();
    let _state = app::State::new(settings.clone(), theme.clone(), service_handles, rx);

    // In a running Wayland session this would start the iced_exwlshell event loop.
    //! For `cargo check` / headless CI skip the actual run and just verify the wiring.

    // The hook that replaces every GTK `init_layer_shell()` call:
    let _on_new_shell = |info: iced_exwlshell::NewShellInfo| app::shell_info_to_message(info);

    if std::env::var("WAYLAND_DISPLAY").is_err() && std::env::var("DISPLAY").is_err() {
        println!(
            "zex-shell - State/Message/services + on_new_shell hook OK (headless, run skipped)"
        );
        return Ok(());
    }

    println!("zex-shell - would run iced_exwlshell::daemon().on_new_shell(...).run() here");
    Ok(())
}
