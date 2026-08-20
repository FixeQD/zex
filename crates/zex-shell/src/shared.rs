//! Shared layer-shell plumbing used by the bar and wallpaper components

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk4::gdk;
use gtk4::gdk::prelude::*;
use gtk4::gdk_pixbuf::{Colorspace, Pixbuf};
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use relm4::Sender;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_core::theme::{Palette, css};

static THEME_CACHE: OnceLock<Mutex<String>> = OnceLock::new();

fn block_on<F: std::future::Future>(future: F) -> anyhow::Result<F::Output> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start theme runtime")?;
    Ok(runtime.block_on(future))
}

fn palette_for_settings(settings: &zex_core::Settings) -> (Palette, bool) {
    let wc = &settings.appearance.wallcolors;
    let dark = wc.dark_mode;
    let scheme = if zex_core::theme::matugen::SCHEMES.contains(&wc.color_scheme.as_str()) {
        wc.color_scheme.as_str()
    } else {
        "tonal_spot"
    };
    let path = if wc.wallpaper_path.is_empty() {
        None
    } else {
        Some(Path::new(wc.wallpaper_path.as_str()))
    };
    let palette = match path.filter(|p| p.is_file()) {
        Some(p) => match block_on(zex_core::theme::matugen::generate(p, scheme, dark)) {
            Ok(Ok(pal)) => pal,
            Ok(Err(e)) | Err(e) => {
                tracing::warn!("palette generation failed, using fallback: {e:#}");
                Palette::default()
            }
        },
        None => Palette::default(),
    };
    (palette, dark)
}

pub fn theme_scss_from_settings(settings: &zex_core::Settings) -> String {
    let (palette, dark) = palette_for_settings(settings);
    match css::theme_scss(&palette, dark) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("theme_scss failed: {e:#}");
            css::theme_scss(&Palette::default(), true).unwrap_or_default()
        }
    }
}

pub fn set_global_theme(scss: String) {
    let cache = THEME_CACHE.get_or_init(|| Mutex::new(String::new()));
    *cache.lock().expect("theme cache poisoned") = scss;
}

pub fn global_theme() -> Option<String> {
    THEME_CACHE
        .get()
        .map(|m| m.lock().expect("theme cache poisoned").clone())
}

pub fn current_theme_scss() -> String {
    if let Some(cached) = global_theme() {
        if !cached.is_empty() {
            return cached;
        }
    }
    let settings = SettingsStore::load()
        .map(|s| s.get().clone())
        .unwrap_or_default();
    theme_scss_from_settings(&settings)
}

fn with_theme(scss: &str) -> String {
    let theme = current_theme_scss();
    if theme.is_empty() {
        scss.to_string()
    } else {
        format!("{theme}\n{scss}")
    }
}

use anyhow::Context;

/// Compile SCSS with grass and load it into `provider`
pub fn install_css(provider: &gtk4::CssProvider, scss: &str) -> anyhow::Result<()> {
    let full = with_theme(scss);
    let css = css::compile(&full)?;
    css::load(provider, &css);
    Ok(())
}

pub fn install_css_without_theme(provider: &gtk4::CssProvider, scss: &str) -> anyhow::Result<()> {
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

pub fn install_global_css() -> gtk4::CssProvider {
    let scss = include_str!("../assets/css/global.scss");
    let provider = gtk4::CssProvider::new();
    let full = with_theme(scss);
    match css::compile(&full) {
        Ok(css) => css::load(&provider, &css),
        Err(e) => tracing::warn!("global css failed: {e:#}"),
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

pub fn apply_system_theming(settings: &zex_core::Settings) {
    let dark = settings.appearance.wallcolors.dark_mode;
    css::apply_system_color_scheme(dark);
    let theme = if dark { "adw-gtk3-dark" } else { "adw-gtk3" };
    let status = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "gtk-theme", theme])
        .status();
    if let Err(e) = status {
        tracing::warn!("could not set gtk-theme: {e}");
    }
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

/// Component-to-component action registry.
///
/// Widgets with no direct component handle (tray in the bar, the quick center
/// opening the power menu) trigger windows through here; every overlay registers
/// its toggle on launch and every consumer just calls it.
#[derive(Default)]
pub struct ActionHandles {
    launcher: RefCell<Option<Rc<dyn Fn()>>>,
    quickcenter: RefCell<Option<Rc<dyn Fn()>>>,
    powermenu: RefCell<Option<Rc<dyn Fn()>>>,
    settings: RefCell<Option<Rc<dyn Fn()>>>,
}

impl ActionHandles {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_launcher(&self, toggle: impl Fn() + 'static) {
        *self.launcher.borrow_mut() = Some(Rc::new(toggle));
    }

    pub fn set_quickcenter(&self, toggle: impl Fn() + 'static) {
        *self.quickcenter.borrow_mut() = Some(Rc::new(toggle));
    }

    pub fn set_powermenu(&self, toggle: impl Fn() + 'static) {
        *self.powermenu.borrow_mut() = Some(Rc::new(toggle));
    }

    pub fn set_settings(&self, open: impl Fn() + 'static) {
        *self.settings.borrow_mut() = Some(Rc::new(open));
    }

    pub fn toggle_launcher(&self) {
        if let Some(toggle) = self.launcher.borrow().as_ref() {
            toggle();
        }
    }

    pub fn toggle_quickcenter(&self) {
        if let Some(toggle) = self.quickcenter.borrow().as_ref() {
            toggle();
        }
    }

    pub fn toggle_powermenu(&self) {
        if let Some(toggle) = self.powermenu.borrow().as_ref() {
            toggle();
        }
    }

    pub fn open_settings(&self) {
        if let Some(open) = self.settings.borrow().as_ref() {
            open();
        }
    }
}
