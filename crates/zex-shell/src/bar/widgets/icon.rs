//! Themed icon resolution shared by bar widgets

use zex_core::app_icon::FALLBACK_ICON;

pub fn has_icon(name: &str) -> bool {
    gtk4::gdk::Display::default()
        .map(|display| gtk4::IconTheme::for_display(&display).has_icon(name))
        .unwrap_or(false)
}

pub fn app_icon(app_id: &str) -> String {
    if has_icon(app_id) {
        app_id.to_owned()
    } else {
        FALLBACK_ICON.to_owned()
    }
}

pub fn window_icon(class: &str, app_id: &str) -> String {
    if !class.is_empty() && has_icon(class) {
        class.to_owned()
    } else {
        app_icon(app_id)
    }
}
