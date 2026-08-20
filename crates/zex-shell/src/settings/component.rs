//! Settings component behaviour: open/close, tab rebuilds, background jobs

use std::path::{Path, PathBuf};

use gtk4::prelude::*;
use zex_core::theme::ThemeManager;
use zex_core::{Settings as SettingsSnapshot, SettingsStore};

use super::Settings;
use super::SettingsMsg;
use super::preview::preview_css;
use super::tabs;

impl Settings {
    pub(crate) fn open(&mut self) {
        if self.window.is_visible() {
            self.window.present();
            return;
        }
        self.window.rail.select(&self.active);
        self.window
            .header_title
            .set_label(&super::tab_label(&self.active));
        self.rebuild_tab();
        self.regenerate_previews(false);
        self.rescan_gallery();
        self.window.present();
    }

    pub(crate) fn set_tab(&mut self, key: String) {
        if self.active == key {
            return;
        }
        self.active = key;
        self.window.rail.select(&self.active);
        self.window
            .header_title
            .set_label(&super::tab_label(&self.active));
        self.rebuild_tab();
    }

    /// Replace the content of the scroll area with the active tab, preserving
    /// the current vertical scroll position.
    pub(crate) fn rebuild_tab(&mut self) {
        if !self.window.is_visible() {
            return;
        }
        let ctx = tabs::TabContext {
            store: std::rc::Rc::clone(&self.store),
            sender: self.sender.clone(),
            previews: self.previews.clone(),
            gallery: self.gallery.clone(),
            window: self.window.root.clone(),
        };
        let widget = tabs::build_tab(&self.active, &ctx);
        let adjustment = self.window.scroll.vadjustment();
        let position = adjustment.value();
        self.window.scroll.set_child(Some(&widget));
        adjustment.set_value(position);
    }

    pub(crate) fn on_settings_changed(&mut self, snapshot: &SettingsSnapshot) {
        let wc = &snapshot.appearance.wallcolors;
        if wc.wallpaper_path != self.last_preview_path || self.previews.is_empty() {
            self.regenerate_previews(false);
        }
        if self.gallery_folder_from(snapshot) != self.last_folder || self.gallery.is_empty() {
            self.rescan_gallery();
        }
    }

    /// Spawn a background preview generator unless an identical one is fresh.
    pub(crate) fn regenerate_previews(&mut self, force: bool) {
        let path = {
            let store = self.store.borrow();
            store.get().appearance.wallcolors.wallpaper_path.clone()
        };
        if path.is_empty() {
            self.last_preview_path.clear();
            if !self.previews.is_empty() {
                self.previews.clear();
                self.refresh_theme_provider();
                self.rebuild_tab();
            }
            return;
        }
        if !force && self.last_preview_path == path && !self.previews.is_empty() {
            return;
        }
        self.last_preview_path = path.clone();
        let sender = self.sender.input_sender().clone();
        std::thread::spawn(move || {
            let manager = ThemeManager::new();
            let previews = manager.previews(Some(Path::new(&path)));
            let _ = sender.send(SettingsMsg::PreviewsReady(previews));
        });
    }

    /// The directory backing the wallpaper gallery, or empty when unset/missing.
    fn gallery_folder(&self) -> String {
        self.gallery_folder_from(&self.store.borrow().get().clone())
    }

    fn gallery_folder_from(&self, settings: &SettingsSnapshot) -> String {
        let wc = &settings.appearance.wallcolors;
        let raw = if wc.quickselect_path.is_empty() {
            "~/Pictures/Wallpapers".to_string()
        } else {
            wc.quickselect_path.clone()
        };
        let expanded = match raw.strip_prefix("~/") {
            Some(rest) => match std::env::var("HOME") {
                Ok(home) => format!("{home}/{rest}"),
                Err(_) => raw.clone(),
            },
            None => raw.clone(),
        };
        if Path::new(&expanded).is_dir() {
            expanded
        } else {
            String::new()
        }
    }

    /// Scan the gallery directory off the main thread.
    pub(crate) fn rescan_gallery(&mut self) {
        let folder = self.gallery_folder();
        if folder.is_empty() {
            self.last_folder.clear();
            if !self.gallery.is_empty() {
                self.gallery.clear();
                self.rebuild_tab();
            }
            return;
        }
        if self.last_folder == folder && !self.gallery.is_empty() {
            return;
        }
        self.last_folder = folder.clone();
        let sender = self.sender.input_sender().clone();
        std::thread::spawn(move || {
            let mut files: Vec<String> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&folder) {
                for entry in entries.flatten() {
                    let Ok(meta) = entry.metadata() else {
                        continue;
                    };
                    if !meta.is_file() {
                        continue;
                    }
                    let path: PathBuf = entry.path();
                    let ext = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(str::to_ascii_lowercase)
                        .unwrap_or_default();
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif")
                        && let Some(path) = path.to_str()
                    {
                        files.push(path.to_string());
                    }
                }
            }
            files.sort();
            let _ = sender.send(SettingsMsg::GalleryReady(files));
        });
    }

    /// Re-read the settings file and push the result back through the store.
    pub(crate) fn reload(&mut self) {
        let source = std::fs::read_to_string(SettingsStore::default_path());
        let loaded: SettingsSnapshot = match source
            .map_err(anyhow::Error::from)
            .and_then(|text| serde_json::from_str(&text).map_err(Into::into))
        {
            Ok(loaded) => loaded,
            Err(err) => {
                tracing::warn!("settings reload failed: {err:#}");
                return;
            }
        };
        if let Err(err) = self.store.borrow_mut().update(|settings| {
            *settings = loaded;
        }) {
            tracing::warn!("settings reload persist failed: {err:#}");
            return;
        }
        self.last_preview_path.clear();
        self.last_folder.clear();
        self.regenerate_previews(true);
        self.rescan_gallery();
        self.rebuild_tab();
    }

    /// Compile the preview swatch and theme-button colors into the CSS provider.
    pub(crate) fn refresh_theme_provider(&mut self) {
        let settings = self.store.borrow().get().clone();
        let css = preview_css(&self.previews, &settings);
        if let Err(err) = crate::shared::install_css_without_theme(&self._theme_provider, &css) {
            tracing::warn!("preview css failed: {err:#}");
        }
    }
}
