//! Native background renderer: one `Layer::Background` surface per monitor

mod render;

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Layer, LayerShell};
use relm4::prelude::*;
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_core::wallpaper::WallpaperState;

use crate::widgets::SharedSettings;

pub const WALLPAPER_CSS_SCSS: &str = include_str!("../../assets/css/wallpaper.scss");

/// Solid fallback colour when no wallpaper is configured
const FALLBACK_RGB: (u8, u8, u8) = (0x13, 0x13, 0x16);

#[derive(Debug)]
pub enum WallpaperMsg {
    SettingsChanged(Box<zex_core::Settings>),
    MonitorsChanged,
}

struct WallpaperWindow {
    root: gtk4::Window,
    picture: gtk4::Picture,
    fade_gen: Rc<std::cell::Cell<u64>>,
    path: Option<PathBuf>,
}

impl WallpaperWindow {
    fn new(monitor_idx: usize, monitor: &gdk::Monitor) -> Self {
        let picture = gtk4::Picture::new();
        picture.set_content_fit(gtk4::ContentFit::Cover);
        picture.set_css_classes(&["wallpaper"]);

        let root = gtk4::Window::new();
        root.init_layer_shell();
        root.set_layer(Layer::Background);
        root.set_monitor(Some(monitor));
        root.set_namespace(Some(&format!("zex-wallpaper-{monitor_idx}")));
        for edge in [
            gtk4_layer_shell::Edge::Top,
            gtk4_layer_shell::Edge::Bottom,
            gtk4_layer_shell::Edge::Left,
            gtk4_layer_shell::Edge::Right,
        ] {
            root.set_anchor(edge, true);
        }
        root.set_child(Some(&picture));

        let window = Self {
            root,
            picture,
            fade_gen: Rc::new(std::cell::Cell::new(0)),
            path: None,
        };
        window.root.present();
        window
    }

    /// Swap the texture when the resolved path changed
    fn apply(&mut self, path: &Option<PathBuf>) {
        if self.path == *path {
            return;
        }
        self.path = path.clone();
        let texture = match path {
            Some(path) => render::load_texture(path, None)
                .inspect_err(|err| tracing::warn!("{err:#}"))
                .unwrap_or_else(|_| render::fallback_texture(FALLBACK_RGB)),
            None => render::fallback_texture(FALLBACK_RGB),
        };
        self.picture.set_paintable(Some(&texture));
        self.fade_in();
    }

    /// 300 ms opacity fade after a swap; stale fades abort via the generation
    fn fade_in(&self) {
        let generation = self.fade_gen.get() + 1;
        self.fade_gen.set(generation);
        self.picture.set_opacity(0.0);
        let picture = self.picture.clone();
        let gen_cell = Rc::clone(&self.fade_gen);
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
}

pub struct Wallpaper {
    windows: HashMap<usize, WallpaperWindow>,
    settings: SharedSettings,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
    _provider: gtk4::CssProvider,
}

fn render_css(provider: &gtk4::CssProvider) -> anyhow::Result<()> {
    let css = grass::from_string(WALLPAPER_CSS_SCSS, &grass::Options::default())
        .map_err(|err| anyhow::anyhow!("compiling wallpaper.scss: {err}"))?;
    provider.load_from_string(&css);
    Ok(())
}

#[relm4::component(pub)]
impl SimpleComponent for Wallpaper {
    type Init = SettingsStore;
    type Input = WallpaperMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        store: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let settings = Rc::new(Mutex::new(store.get().clone()));
        let provider = gtk4::CssProvider::new();
        let mut model = Self {
            windows: HashMap::new(),
            settings,
            subscription: None,
            monitors_model: None,
            _provider: provider,
        };

        if let Some(display) = gdk::Display::default() {
            if let Err(err) = render_css(&model._provider) {
                tracing::warn!("{err:#}");
            }
            gtk4::style_context_add_provider_for_display(
                &display,
                &model._provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            let monitors_model = display.monitors();
            monitors_model.connect_items_changed({
                let sender = sender.clone();
                move |_, _, _, _| sender.input(WallpaperMsg::MonitorsChanged)
            });
            model.monitors_model = Some(monitors_model);
            model.sync_monitors(&display);
            model.refresh();
        } else {
            tracing::warn!("no display available; wallpaper stays hidden");
        }

        let bridge_tx = sender.input_sender().clone();
        let subscription = store.subscribe(move |snapshot| {
            let _ = bridge_tx.send(WallpaperMsg::SettingsChanged(Box::new(snapshot.clone())));
        });
        model.subscription = Some(subscription);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            WallpaperMsg::SettingsChanged(snapshot) => {
                *self.settings.lock().expect("settings mutex poisoned") = *snapshot;
                self.refresh();
            }
            WallpaperMsg::MonitorsChanged => {
                if let Some(display) = gdk::Display::default() {
                    self.sync_monitors(&display);
                    self.refresh();
                }
            }
        }
    }
}

impl Wallpaper {
    /// Re-read the wallpaper path from the snapshot and push it to every window
    fn refresh(&mut self) {
        let path = {
            let settings = self.settings.lock().expect("settings mutex poisoned");
            WallpaperState::from_settings(&settings).resolve()
        };
        for window in self.windows.values_mut() {
            window.apply(&path);
        }
    }

    fn sync_monitors(&mut self, display: &gdk::Display) {
        let monitors: Vec<gdk::Monitor> =
            display.monitors().iter().filter_map(Result::ok).collect();
        let present: std::collections::HashSet<usize> = (0..monitors.len()).collect();
        self.windows.retain(|idx, _| present.contains(idx));

        for (idx, monitor) in monitors.into_iter().enumerate() {
            if self.windows.contains_key(&idx) {
                continue;
            }
            self.windows
                .insert(idx, WallpaperWindow::new(idx, &monitor));
        }
    }
}
