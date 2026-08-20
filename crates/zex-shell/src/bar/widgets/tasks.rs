//! Task dock: pinned apps plus windows of the compositor session.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use zex_launcher::apps::{
    AppInfo, DEFAULT_TERMINAL_TEMPLATE, PinnedApps, spawn_command, spawn_entry,
};
use zex_services::compositor::WindowInfo;

use super::icon::{app_icon, has_icon};
use super::popover::{PopoverItem, show_popover};

pub fn is_same_app(id1: &str, id2: &str) -> bool {
    if id1.is_empty() || id2.is_empty() {
        return false;
    }
    let a = id1.to_lowercase();
    let b = id2.to_lowercase();
    a.contains(&b) || b.contains(&a)
}

/// Windows whose class matches the app id
pub fn app_windows<'a>(app: &AppInfo, windows: &'a [WindowInfo]) -> Vec<&'a WindowInfo> {
    windows
        .iter()
        .filter(|window| is_same_app(&app.id, &window.class))
        .collect()
}

/// Themed icon name for an app: its own icon when the theme knows it
fn app_icon_name(app: &AppInfo) -> String {
    let name = app.icon_name.as_deref().filter(|name| has_icon(name));
    match name {
        Some(name) => app_icon(name),
        None => app_icon(&app.id),
    }
}

/// Indicator position depends on the bar side, mirroring the reference css
fn indicator_pos_class(vertical: bool, side: &str) -> &'static str {
    if vertical {
        if side == "left" { "left" } else { "right" }
    } else if side == "top" {
        "top"
    } else {
        "bottom"
    }
}

/// Dock icon size: 20px on normal density, 16px otherwise
fn dock_icon_size(density: i8) -> i32 {
    if matches!(density, 0 | 1) { 20 } else { 16 }
}

fn launch(app: &AppInfo) {
    if let Err(err) = spawn_entry(app, DEFAULT_TERMINAL_TEMPLATE) {
        tracing::warn!("dock launch of {} failed: {err:#}", app.id);
    }
}

fn focus(address: &str, on_focus: &Rc<dyn Fn(String)>) {
    on_focus(address.to_owned());
}

struct State {
    apps: Vec<AppInfo>,
    windows: Vec<WindowInfo>,
    active: Option<String>,
    vertical: bool,
    side: String,
    dense: bool,
    icon_size: i32,
}

pub struct Tasks {
    container: gtk4::Box,
    pins: Rc<PinnedApps>,
    on_focus: Option<Rc<dyn Fn(String)>>,
    state: RefCell<State>,
}

impl Tasks {
    /// `on_focus` addresses windows by compositor address when a backend is bound
    pub fn new(
        vertical: bool,
        density: i8,
        on_focus: Option<Rc<dyn Fn(String)>>,
        pins: Rc<PinnedApps>,
    ) -> Rc<Self> {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.set_css_classes(&["tasks"]);
        let state = State {
            apps: Vec::new(),
            windows: Vec::new(),
            active: None,
            vertical,
            side: "bottom".to_owned(),
            dense: density < 0,
            icon_size: dock_icon_size(density),
        };
        Rc::new(Self {
            container,
            pins,
            on_focus,
            state: RefCell::new(state),
        })
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    /// Refresh on compositor, catalog and pins events; skips churn when unchanged
    pub fn update(
        &self,
        apps: &[AppInfo],
        windows: &[WindowInfo],
        active: Option<&WindowInfo>,
        vertical: bool,
        side: &str,
        density: i8,
    ) {
        let active_id = active.map(|window| window.class.clone());
        let dense = density < 0;
        let mut state = self.state.borrow_mut();
        if state.apps == apps
            && state.windows == windows
            && state.active == active_id
            && state.vertical == vertical
            && state.side.as_str() == side
            && state.dense == dense
        {
            return;
        }
        state.apps = apps.to_vec();
        state.windows = windows.to_vec();
        state.active = active_id;
        state.vertical = vertical;
        state.side = side.to_owned();
        state.dense = dense;
        state.icon_size = dock_icon_size(density);

        let snapshot = State {
            apps: state.apps.clone(),
            windows: state.windows.clone(),
            active: state.active.clone(),
            vertical: state.vertical,
            side: state.side.clone(),
            dense: state.dense,
            icon_size: state.icon_size,
        };
        drop(state);

        self.container.set_orientation(if snapshot.vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        });

        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        let pinned_ids: Vec<String> = self.pins.pinned();
        let pinned: Vec<&AppInfo> = apps
            .iter()
            .filter(|app| pinned_ids.iter().any(|id| id == &app.id))
            .collect();
        let open_classes: Vec<&str> = snapshot
            .windows
            .iter()
            .map(|window| window.class.as_str())
            .collect();

        for app in pinned {
            let is_open = !app_windows(app, &snapshot.windows).is_empty();
            let is_active = snapshot
                .active
                .as_ref()
                .is_some_and(|id| is_same_app(&app.id, id));
            self.append_app(app, is_open, is_active, &snapshot);
        }

        let unpinned_open: Vec<&AppInfo> = apps
            .iter()
            .filter(|app| !pinned_ids.iter().any(|id| id == &app.id))
            .filter(|app| open_classes.iter().any(|class| is_same_app(&app.id, class)))
            .collect();
        if !unpinned_open.is_empty() {
            let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
            separator.set_css_classes(&["dock-separator"]);
            self.container.append(&separator);
            for app in unpinned_open {
                let is_active = snapshot
                    .active
                    .as_ref()
                    .is_some_and(|id| is_same_app(&app.id, id));
                self.append_app(app, true, is_active, &snapshot);
            }
        }
    }

    fn append_app(&self, app: &AppInfo, is_open: bool, is_active: bool, snapshot: &State) {
        let icon = gtk4::Image::from_icon_name(&app_icon_name(app));
        icon.set_pixel_size(snapshot.icon_size);

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&icon));

        if is_open {
            let indicator = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            indicator.set_css_classes(&[
                "app-indicator",
                indicator_pos_class(snapshot.vertical, &snapshot.side),
            ]);
            overlay.add_overlay(&indicator);
        }

        let button = gtk4::Button::new();
        button.set_child(Some(&overlay));
        button.set_css_classes(&["app-button"]);
        if is_open {
            button.add_css_class("open-app");
        }
        if is_active {
            button.add_css_class("active-app");
        }

        let windows = snapshot.windows.clone();
        let on_focus = self.on_focus.clone();
        let pins = Rc::clone(&self.pins);

        let click_app = app.clone();
        let click_windows = windows.clone();
        let click_focus = on_focus.clone();
        button.connect_clicked(move |button| {
            if let Some(focus_handle) = &click_focus {
                let matches = app_windows(&click_app, &click_windows);
                match matches.len() {
                    0 => launch(&click_app),
                    1 => focus(&matches[0].address, focus_handle),
                    _ => show_window_list(button.upcast_ref(), matches, focus_handle),
                }
            } else {
                launch(&click_app);
            }
        });

        let menu_app = app.clone();
        let menu_windows = windows;
        let menu_pins = Rc::clone(&pins);
        let menu_focus = on_focus;
        let anchor = button.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gtk4::gdk::BUTTON_SECONDARY);
        gesture.connect_pressed(move |_gesture, _n, _x, _y| {
            show_app_menu(
                anchor.upcast_ref(),
                &menu_app,
                &menu_windows,
                is_open,
                Rc::clone(&menu_pins),
                menu_focus.clone(),
            );
        });
        button.add_controller(gesture);

        self.container.append(&button);
    }
}

/// List popover when the app has several windows
fn show_window_list(
    anchor: &gtk4::Widget,
    windows: Vec<&WindowInfo>,
    on_focus: &Rc<dyn Fn(String)>,
) {
    let items = windows
        .into_iter()
        .map(|window| {
            let address = window.address.clone();
            let title = window.title.clone();
            let on_focus = on_focus.clone();
            PopoverItem::Action(
                if title.is_empty() {
                    "Untitled Window".to_owned()
                } else {
                    title
                },
                Rc::new(move || focus(&address, &on_focus)),
            )
        })
        .collect();
    show_popover(anchor, items);
}

/// PPM menu: name, pin toggle, then per-open new-window and desktop actions
fn show_app_menu(
    anchor: &gtk4::Widget,
    app: &AppInfo,
    windows: &[WindowInfo],
    is_open: bool,
    pins: Rc<PinnedApps>,
    on_focus: Option<Rc<dyn Fn(String)>>,
) {
    let mut items = vec![
        PopoverItem::Label(app.title.clone()),
        PopoverItem::Separator,
    ];

    let pinned = pins.is_pinned(&app.id);
    let label = if pinned {
        "Unpin from Dock".to_owned()
    } else {
        "Pin to Dock".to_owned()
    };
    let id = app.id.clone();
    items.push(PopoverItem::Action(
        label,
        Rc::new(move || {
            pins.toggle(&id);
        }),
    ));

    if is_open {
        items.push(PopoverItem::Separator);
        let matches = app_windows(app, windows);
        if matches.len() == 1
            && let Some(focus_handle) = &on_focus
        {
            let address = matches[0].address.clone();
            let on_focus = focus_handle.clone();
            items.push(PopoverItem::Action(
                "Focus Window".to_owned(),
                Rc::new(move || {
                    focus(&address, &on_focus);
                }),
            ));
        }
        let app = app.clone();
        items.push(PopoverItem::Action(
            "New Window".to_owned(),
            Rc::new(move || {
                launch(&app);
            }),
        ));
    }

    for action in &app.actions {
        let command = action.command.clone();
        let name = action.name.clone();
        items.push(PopoverItem::Action(
            name,
            Rc::new(move || {
                if let Err(err) = spawn_command(&command) {
                    tracing::warn!("dock action failed: {err:#}");
                }
            }),
        ));
    }

    show_popover(anchor, items);
}
