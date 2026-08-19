//! Active window: app icon, class and title, with empty-workspace fallbacks

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_core::app_icon::FALLBACK_ICON;
use zex_services::compositor::WindowInfo;

const EMPTY_WORKSPACE: &str = "Empty Workspace";
const DESKTOP: &str = "Desktop";
const MAX_TITLE_CHARS: usize = 52;

#[derive(Debug, Clone, PartialEq)]
struct State {
    title: String,
    app_id: String,
    vertical: bool,
    centered: bool,
    dense: bool,
}

pub struct WindowInfoWidget {
    container: gtk4::Box,
    icon: gtk4::Image,
    title: gtk4::Label,
    app_id: gtk4::Label,
    state: RefCell<State>,
}

impl WindowInfoWidget {
    pub fn new() -> Rc<Self> {
        let icon = gtk4::Image::from_icon_name(FALLBACK_ICON);
        icon.set_pixel_size(16);
        icon.set_css_classes(&["winfo-icon"]);

        let title = gtk4::Label::new(None);
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title.set_single_line_mode(true);
        title.set_css_classes(&["winfo-title"]);

        let app_id = gtk4::Label::new(None);
        app_id.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        app_id.set_single_line_mode(true);
        app_id.set_css_classes(&["winfo-app-id"]);

        let labels = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        labels.append(&title);
        labels.append(&app_id);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.set_css_classes(&["winfo"]);
        container.append(&icon);
        container.append(&labels);

        Rc::new(Self {
            container,
            icon,
            title,
            app_id,
            state: RefCell::new(State {
                title: EMPTY_WORKSPACE.into(),
                app_id: DESKTOP.into(),
                vertical: false,
                centered: false,
                dense: false,
            }),
        })
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    /// Refresh on compositor events; skips layout churn when nothing changed
    pub fn update(&self, active: Option<&WindowInfo>, vertical: bool, centered: bool, density: i8) {
        let title =
            truncate(&active.map_or_else(|| EMPTY_WORKSPACE.to_owned(), |w| w.title.clone()));
        let app_id = active.map_or_else(|| DESKTOP.to_owned(), |w| w.class.clone());
        if app_id.is_empty() {
            self.app_id.set_text(DESKTOP);
        } else {
            self.app_id.set_text(&app_id);
        }
        self.title.set_text(&title);
        if let Some(class) = active
            .as_ref()
            .map(|w| w.class.as_str())
            .filter(|c| !c.is_empty())
        {
            let name = if has_icon(class) {
                class.to_owned()
            } else {
                app_icon_fallback(&app_id)
            };
            self.icon.set_icon_name(Some(&name));
        } else {
            self.icon.set_icon_name(Some(FALLBACK_ICON));
        }

        let dense = density < 0;
        let next = State {
            title,
            app_id: app_id.clone(),
            vertical,
            centered,
            dense,
        };
        let mut state = self.state.borrow_mut();
        if *state == next {
            return;
        }
        *state = next;

        self.apply_layout(&state);
    }

    fn apply_layout(&self, state: &State) {
        self.container.remove_css_class("vertical");
        if state.vertical {
            self.container.add_css_class("vertical");
            self.container.set_halign(gtk4::Align::Center);
            self.container.set_hexpand(false);
            self.container.set_width_request(-1);
            self.title.set_visible(false);
            self.app_id.set_visible(false);
            self.container
                .set_tooltip_text(Some(&format!("{}\n{}", state.title, state.app_id)));
            return;
        }
        self.container.set_tooltip_text(Some(&state.app_id));
        let show_app_id = !state.dense;
        self.title.set_visible(true);
        self.app_id.set_visible(show_app_id);
        self.app_id.set_visible(show_app_id);
        self.app_id.set_hexpand(false);
        if state.centered {
            self.container.set_halign(gtk4::Align::Center);
            self.container.set_hexpand(false);
            self.container.set_width_request(150);
        } else {
            self.container.set_halign(gtk4::Align::Start);
            self.container.set_hexpand(true);
            self.container.set_width_request(-1);
        }
    }
}

/// Icon for the app-id, falling back to theme browsing only when the theme lacks it
fn app_icon_fallback(app_id: &str) -> String {
    if has_icon(app_id) {
        app_id.to_owned()
    } else {
        FALLBACK_ICON.to_owned()
    }
}

fn has_icon(name: &str) -> bool {
    gtk4::gdk::Display::default()
        .map(|display| gtk4::IconTheme::for_display(&display).has_icon(name))
        .unwrap_or(false)
}

pub fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_TITLE_CHARS {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(MAX_TITLE_CHARS).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_titles() {
        assert_eq!(truncate("hello"), "hello");
        assert_eq!(truncate(&"x".repeat(52)), "x".repeat(52));
    }

    #[test]
    fn truncate_marks_long_titles() {
        let long = "a".repeat(53);
        let cut = truncate(&long);
        assert_eq!(cut.chars().count(), 53);
        assert!(cut.ends_with('…'));
        assert!(!cut.ends_with('a'));
    }
}
