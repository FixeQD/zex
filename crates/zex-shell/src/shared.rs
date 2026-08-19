//! Shared layer-shell plumbing used by the bar and wallpaper components

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::gdk;
use gtk4::gdk::prelude::*;
use gtk4::gdk_pixbuf::{Colorspace, Pixbuf};
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use relm4::Sender;
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_core::theme::css;

/// Compile SCSS with grass and load it into `provider`
pub fn install_css(provider: &gtk4::CssProvider, scss: &str) -> anyhow::Result<()> {
    let css = css::compile(scss)?;
    css::load(provider, &css);
    Ok(())
}

/// Compile `scss`, register the provider for the default display and return it
pub fn install_css_provider(scss: &str) -> gtk4::CssProvider {
    let provider = gtk4::CssProvider::new();
    if let Err(err) = install_css(&provider, scss) {
        tracing::warn!("{err:#}");
    }
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    provider
}

/// Snapshot of the connected monitors in index order
pub fn monitors(display: &gdk::Display) -> Vec<gdk::Monitor> {
    display.monitors().iter().filter_map(Result::ok).collect()
}

/// Forward monitor add/remove to `sender`
/// Returns the model that must be kept alive for the signal connection to stay active
pub fn watch_monitors<M: Send + 'static>(
    display: &gdk::Display,
    sender: Sender<M>,
    msg: impl Fn() -> M + Send + 'static,
) -> gio::ListModel {
    let model = display.monitors();
    model.connect_items_changed(move |_, _, _, _| {
        let _ = sender.send(msg());
    });
    model
}

/// Bridge settings updates from the store's thread into the GTK loop
pub fn subscribe_settings<M: Send + 'static>(
    store: &SettingsStore,
    sender: Sender<M>,
    msg: impl Fn(&zex_core::Settings) -> M + Send + Sync + 'static,
) -> Subscription {
    store.subscribe(move |snapshot| {
        let _ = sender.send(msg(snapshot));
    })
}

/// Wrap a raw RGBA byte buffer as a texture
pub fn texture_from_rgba(width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<gdk::Texture> {
    let bytes = glib::Bytes::from(rgba);
    let pixbuf = Pixbuf::from_bytes(
        &bytes,
        Colorspace::Rgb,
        true,
        8,
        width as i32,
        height as i32,
        width as i32 * 4,
    );
    Ok(gdk::Texture::for_pixbuf(&pixbuf))
}

/// 300 ms opacity fade after a texture swap
pub fn fade_in(picture: &gtk4::Picture, fade_gen: &Rc<Cell<u64>>) {
    let generation = fade_gen.get() + 1;
    fade_gen.set(generation);
    picture.set_opacity(0.0);
    let picture = picture.clone();
    let gen_cell = Rc::clone(fade_gen);
    glib::timeout_add_local(Duration::from_millis(30), move || {
        if gen_cell.get() != generation {
            return glib::ControlFlow::Break;
        }
        let opacity = picture.opacity().min(1.0) + 0.1;
        picture.set_opacity(opacity);
        if opacity >= 1.0 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
