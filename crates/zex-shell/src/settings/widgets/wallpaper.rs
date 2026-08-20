//! Wallpaper preview overlay with an implicit file picker

use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;

use crate::settings::tabs::TabContext;

/// A clickable wallpaper preview with an implicit file picker
pub fn wallpaper_overlay(ctx: &TabContext) -> gtk4::Overlay {
    let snapshot = ctx.snapshot();
    let snapshot = &snapshot.appearance.wallcolors;

    let picture = gtk4::Picture::new();
    picture.add_css_class("wallpaper-preview");
    picture.set_content_fit(gtk4::ContentFit::Cover);
    picture.set_size_request(560, 300);
    if !snapshot.wallpaper_path.is_empty() {
        picture.set_filename(Some(&snapshot.wallpaper_path));
    }

    let base =
        basename(&snapshot.wallpaper_path).unwrap_or_else(|| "Click to set wallpaper".to_string());
    let file_label = gtk4::Label::new(Some(&base));
    file_label.add_css_class("wallpaper-filename-label");
    file_label.set_halign(gtk4::Align::Start);
    file_label.set_valign(gtk4::Align::End);
    file_label.set_margin_start(10);
    file_label.set_margin_bottom(10);

    let picker = gtk4::Button::new();
    picker.add_css_class("wallpaper-button-overlay");
    picker.set_has_frame(false);
    picker.set_halign(gtk4::Align::Fill);
    picker.set_valign(gtk4::Align::Fill);
    picker.set_hexpand(true);
    picker.set_vexpand(true);

    let store = Rc::clone(&ctx.store);
    let window = ctx.window.clone();
    let overlay_picture = picture.clone();
    let overlay_label = file_label.clone();
    picker.connect_clicked(move |_| {
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("Images (PNG, JPG, WebP, GIF)"));
        for mime in ["image/jpeg", "image/png", "image/webp", "image/gif"] {
            filter.add_mime_type(mime);
        }
        let filters = gio::ListStore::new::<gtk4::FileFilter>();
        filters.append(&filter);
        let dialog = gtk4::FileDialog::builder()
            .title("Select Wallpaper")
            .filters(&filters)
            .build();
        let store = Rc::clone(&store);
        let picture = picture.clone();
        let file_label = file_label.clone();
        dialog.open(Some(&window), None::<&gio::Cancellable>, move |result| {
            let Ok(file) = result else {
                return;
            };
            let Some(path) = file.path() else {
                return;
            };
            let Ok(path) = path.into_os_string().into_string() else {
                return;
            };
            let base = basename(&path).unwrap_or_else(|| "Click to set wallpaper".to_string());
            if let Err(err) = store.borrow_mut().update(|s| {
                s.appearance.wallcolors.wallpaper_path = path.clone();
            }) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
            picture.set_filename(Some(&path));
            file_label.set_label(&base);
        });
    });

    let overlay = gtk4::Overlay::new();
    overlay.add_css_class("wallpaper-overlay");
    overlay.set_halign(gtk4::Align::Fill);
    overlay.set_hexpand(true);
    overlay.set_child(Some(&overlay_picture));
    overlay.add_overlay(&picker);
    overlay.add_overlay(&overlay_label);
    overlay
}

fn basename(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}
