use anyhow::Context;
use gtk4::prelude::*;
use relm4::Component;
use zex_shell::bar::Bars;
use zex_shell::corners::Corners;
use zex_shell::launcher::Launcher;
use zex_shell::lockscreen::Lockscreen;
use zex_shell::overlays::osd::Osd;
use zex_shell::shared::ActionHandles;
use zex_shell::wallpaper::Wallpaper;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    gtk4::init().context("initializing gtk")?;
    tracing::info!("zex shell starting");

    let actions = ActionHandles::new();

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _bars = Bars::builder().launch((store, actions.clone()));

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _corners = Corners::builder().launch(store);

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _osd = Osd::builder().launch(store);

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _wallpaper = Wallpaper::builder().launch(store);

    let (lock_tx, lock_rx) = flume::unbounded();
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                tracing::warn!("lock runtime unavailable: {err}");
                return;
            }
        };
        runtime.block_on(async {
            let Ok(connection) = zbus::Connection::session().await else {
                tracing::warn!("session bus unavailable; lock stays off");
                return;
            };
            match zex_services::lockscreen::Lockscreen::connect(connection, lock_tx).await {
                Ok(_service) => std::future::pending::<()>().await,
                Err(err) => tracing::warn!("lock service unavailable: {err:#}"),
            }
        });
    });

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _lockscreen = Lockscreen::builder().launch((store, lock_rx));

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _launcher = Launcher::builder().launch((store, actions.clone()));

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _settings = zex_shell::settings::Settings::builder().launch((store, actions.clone()));

    let _m3_provider = zex_shell::m3::install_css();
    if std::env::var_os("ZEX_M3_SHOWCASE").is_some() {
        tracing::info!("ZEX_M3_SHOWCASE set; opening m3 showcase window");
        let showcase = zex_shell::m3::showcase::window();
        showcase.present();
    }

    let main_loop = gtk4::glib::MainLoop::new(None, false);
    main_loop.run();
    Ok(())
}
