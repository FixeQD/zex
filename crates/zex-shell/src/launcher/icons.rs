//! App icon widgets

use std::path::Path;

use gtk4::gdk;
use gtk4::prelude::*;
use zex_launcher::apps::AppInfo;
use zex_launcher::items::Item;

pub const ICON_SIZE: i32 = 32;
pub const FEATURED_ICON_SIZE: i32 = 40;
pub const GRID_ICON_SIZE: i32 = 40;

fn load_icon_texture(path: &Path, size: u32) -> anyhow::Result<gdk::Texture> {
    let image = image::open(path)
        .map_err(|err| anyhow::anyhow!("icon decode failed for {}: {err}", path.display()))?
        .resize_to_fill(size, size, image::imageops::FilterType::Triangle)
        .to_rgba8();
    crate::shared::texture_from_rgba(size, size, image.as_raw())
}

fn fallback_icon_name(item: &Item) -> &'static str {
    match item {
        Item::Command(_) => "utilities-terminal-symbolic",
        Item::Web { .. } => "web-browser-symbolic",
        Item::Calc { .. } => "accessories-calculator-symbolic",
        Item::Clipboard(_) => "edit-paste-symbolic",
        Item::Emoji(_) => "smile-symbolic",
        Item::Theme(_) => "emblem-photos-symbolic",
        Item::Ai(_) => "avatar-default-symbolic",
        Item::Menu(_) => "go-next-symbolic",
        Item::Window { .. } => "window-new-symbolic",
        Item::Action { .. } => "system-run-symbolic",
        Item::File(_) => "text-x-generic-symbolic",
        Item::App(_) => "application-x-executable-symbolic",
    }
}

pub fn icon_widget(app: &AppInfo, size: i32, css_class: &str) -> gtk4::Widget {
    if let Some(texture) = app
        .icon_file
        .as_ref()
        .and_then(|path| load_icon_texture(path, size as u32).ok())
    {
        let picture = gtk4::Picture::new();
        picture.set_paintable(Some(&texture));
        picture.set_width_request(size);
        picture.set_height_request(size);
        picture.set_css_classes(&[css_class]);
        picture.set_content_fit(gtk4::ContentFit::Contain);
        return picture.upcast();
    }
    if let Some(name) = app.icon_name.as_ref() {
        let image = gtk4::Image::from_icon_name(name);
        image.set_pixel_size(size);
        image.set_css_classes(&[css_class]);
        return image.upcast();
    }
    let placeholder = gtk4::Label::new(Some("?"));
    placeholder.set_css_classes(&[css_class, "launcher-icon-placeholder"]);
    placeholder.set_width_request(size);
    placeholder.set_height_request(size);
    placeholder.set_halign(gtk4::Align::Center);
    placeholder.set_valign(gtk4::Align::Center);
    placeholder.upcast()
}

pub fn fallback_icon(item: &Item, size: i32, css_class: &str) -> gtk4::Widget {
    let image = gtk4::Image::from_icon_name(fallback_icon_name(item));
    image.set_pixel_size(size);
    image.set_css_classes(&[css_class]);
    image.upcast()
}
