//! Per-monitor lock window

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use zex_core::Settings;

use crate::wallpaper::FALLBACK_RGB;
use crate::wallpaper::render;
use crate::widgets::{format_date, format_time, local_time};

pub const LOCK_BLUR_SIGMA: f32 = 15.0;

pub struct LockWindow {
    pub root: gtk4::Window,
    picture: gtk4::Picture,
    clock: gtk4::Label,
    date: gtk4::Label,
    error: gtk4::Label,
    pub entry: gtk4::PasswordEntry,
    fade_gen: Rc<Cell<u64>>,
    path: Option<PathBuf>,
    blur: bool,
    pub user: String,
    pub busy: bool,
}

impl LockWindow {
    pub fn new(
        monitor_idx: usize,
        monitor: &gdk::Monitor,
        user: String,
        on_submit: impl Fn() + 'static,
        on_cancel: impl Fn() + 'static,
    ) -> Self {
        let picture = gtk4::Picture::new();
        picture.set_content_fit(gtk4::ContentFit::Cover);

        let clock = gtk4::Label::new(None);
        clock.set_css_classes(&["lockscreen-clock"]);
        let date = gtk4::Label::new(None);
        date.set_css_classes(&["lockscreen-date"]);
        let error = gtk4::Label::new(None);
        error.set_css_classes(&["lockscreen-error"]);
        error.set_visible(false);

        let entry = gtk4::PasswordEntry::new();
        entry.set_css_classes(&["lockscreen-entry"]);
        entry.set_show_peek_icon(false);
        entry.set_placeholder_text(Some("Password"));
        entry.connect_activate(move |_| on_submit());

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        content.set_css_classes(&["lockscreen-content"]);
        content.set_halign(gtk4::Align::Center);
        content.set_valign(gtk4::Align::Center);
        content.append(&clock);
        content.append(&date);
        content.append(&error);
        content.append(&entry);

        let layers = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        layers.set_css_classes(&["lockscreen-dim"]);
        layers.set_halign(gtk4::Align::Fill);
        layers.set_valign(gtk4::Align::Fill);
        layers.append(&content);

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&picture));
        overlay.add_overlay(&layers);

        let root = gtk4::Window::new();
        root.init_layer_shell();
        root.set_layer(Layer::Overlay);
        root.set_monitor(Some(monitor));
        root.set_namespace(Some(&format!("zex-lockscreen-{monitor_idx}")));
        root.set_keyboard_mode(KeyboardMode::Exclusive);
        for edge in [
            gtk4_layer_shell::Edge::Top,
            gtk4_layer_shell::Edge::Bottom,
            gtk4_layer_shell::Edge::Left,
            gtk4_layer_shell::Edge::Right,
        ] {
            root.set_anchor(edge, true);
        }
        root.set_child(Some(&overlay));

        let controller = gtk4::EventControllerKey::new();
        controller.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                on_cancel();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        root.add_controller(controller);

        let window = Self {
            root,
            picture,
            clock,
            date,
            error,
            entry,
            fade_gen: Rc::new(Cell::new(0)),
            path: None,
            blur: false,
            user,
            busy: false,
        };
        window.root.present();
        window
    }

    pub fn apply_wallpaper(&mut self, path: &Option<PathBuf>, blur: bool) {
        if self.path.as_ref() == path.as_ref() && self.blur == blur {
            return;
        }
        self.path = path.clone();
        self.blur = blur;
        let texture = match path {
            Some(path) => {
                let sigma = blur.then_some(LOCK_BLUR_SIGMA);
                render::load_texture(path, sigma)
                    .inspect_err(|err| tracing::warn!("{err:#}"))
                    .unwrap_or_else(|_| render::fallback_texture(FALLBACK_RGB))
            }
            None => render::fallback_texture(FALLBACK_RGB),
        };
        self.picture.set_paintable(Some(&texture));
        crate::shared::fade_in(&self.picture, &self.fade_gen);
    }

    pub fn refresh_clock(&mut self, settings: &Settings) {
        let now = local_time();
        let options = &settings.interface.modules.options;
        self.clock
            .set_label(&format_time(&now, options.military_time, false));
        let show_date = options.show_date;
        self.date
            .set_label(&format_date(&now, options.day_month_swapped, false));
        self.date.set_visible(show_date);
    }

    pub fn show_error(&mut self, text: &str) {
        self.error.set_label(text);
        self.error.set_visible(true);
    }

    pub fn clear_state(&mut self) {
        self.entry.set_text("");
        self.error.set_visible(false);
    }

    pub fn focus(&self) {
        self.entry.grab_focus();
    }
}
