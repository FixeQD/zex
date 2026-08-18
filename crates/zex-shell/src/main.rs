use anyhow::Context;
use relm4::Component;
use zex_shell::bar::Bars;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    gtk4::init().context("initializing gtk")?;
    tracing::info!("zex shell starting");

    let store = zex_core::SettingsStore::load().context("loading settings")?;
    let _bars = Bars::builder().launch(store);

    let main_loop = gtk4::glib::MainLoop::new(None, false);
    main_loop.run();
    Ok(())
}
