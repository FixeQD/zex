//! Workspace switcher: numbers, dots or per-workspace icons

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::glib::Propagation;
use gtk4::prelude::*;
use zex_core::app_icon::FALLBACK_ICON;
use zex_services::compositor::{WindowInfo, WorkspaceInfo};

/// 1-based inclusive range of the `amount`-sized page around `active`
pub fn page_window(active: i32, amount: usize) -> (i32, i32) {
    let amount = amount.max(1) as i32;
    let base = (active - 1).div_euclid(amount) * amount + 1;
    (base, base + amount - 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Numbers,
    Dots,
    Windows,
}

impl Style {
    pub fn from_settings(style: &str) -> Self {
        match style {
            "dots" => Style::Dots,
            "windows" => Style::Windows,
            _ => Style::Numbers,
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Style::Numbers => "numbers",
            Style::Dots => "dots",
            Style::Windows => "windows",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    style: Style,
    fixed: bool,
    amount: usize,
    /// raw ids of real workspaces, sorted
    real: Vec<i32>,
    active_raw: i32,
    /// (raw workspace id, window class) pairs
    windows: Vec<(i32, String)>,
    vertical: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspacesOptions {
    pub style: Style,
    pub fixed: bool,
    pub amount: usize,
    pub vertical: bool,
    pub display_offset: i32,
}

pub struct Workspaces {
    container: gtk4::Box,
    state: RefCell<State>,
    on_switch: Option<Rc<dyn Fn(i32)>>,
}

impl Workspaces {
    /// `on_switch` receives the raw workspace id to switch to
    /// `display_offset` maps raw ids to 1-based labels (0 for Hyprland, 1 for Niri)
    pub fn new(
        vertical: bool,
        display_offset: i32,
        on_switch: Option<Rc<dyn Fn(i32)>>,
    ) -> Rc<Self> {
        let container = gtk4::Box::new(
            if vertical {
                gtk4::Orientation::Vertical
            } else {
                gtk4::Orientation::Horizontal
            },
            5,
        );
        container.add_css_class("workspaces");
        let widget = Rc::new(Self {
            container: container.clone(),
            state: RefCell::new(State {
                style: Style::Numbers,
                fixed: false,
                amount: 5,
                real: Vec::new(),
                active_raw: 0,
                windows: Vec::new(),
                vertical,
            }),
            on_switch,
        });

        let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll({
            let widget = Rc::clone(&widget);
            move |_, _, delta_y| {
                let state = widget.state.borrow();
                let active_display = state.active_raw + display_offset;
                let target = (active_display + if delta_y > 0.0 { 1 } else { -1 }).clamp(1, 20);
                let switch_to = target - display_offset;
                if let Some(on_switch) = &widget.on_switch {
                    on_switch(switch_to);
                }
                Propagation::Proceed
            }
        });
        container.add_controller(scroll);
        widget
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.container.clone().upcast()
    }

    /// Rebuild buttons when anything switchable changed (called on compositor events)
    pub fn update(
        &self,
        workspaces: &[WorkspaceInfo],
        windows: &[WindowInfo],
        options: WorkspacesOptions,
    ) {
        let mut real: Vec<i32> = workspaces.iter().map(|ws| ws.id).collect();
        real.sort_unstable();
        real.dedup();
        let active_raw = workspaces
            .iter()
            .find(|ws| ws.active)
            .map_or(-1, |ws| ws.id);
        let window_classes: Vec<(i32, String)> = windows
            .iter()
            .map(|w| (w.workspace, w.class.clone()))
            .collect();
        let next = State {
            style: options.style,
            fixed: options.fixed,
            amount: options.amount.max(1),
            real,
            active_raw,
            windows: window_classes,
            vertical: options.vertical,
        };
        let mut state = self.state.borrow_mut();
        if *state == next {
            return;
        }
        *state = next;

        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }
        self.container.set_orientation(if state.vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        });
        self.container
            .set_spacing(if state.style == Style::Windows { 2 } else { 5 });
        self.container
            .set_css_classes(&["workspaces", state.style.css_class()]);

        let ids: Vec<i32> = if state.fixed {
            let (base, end) = page_window(
                state.active_raw.max(1) + options.display_offset,
                state.amount,
            );
            (base..=end)
                .map(|label| label - options.display_offset)
                .collect()
        } else {
            state.real.clone()
        };
        let rebuilt: Vec<gtk4::Button> = ids
            .iter()
            .map(|id| workspace_button(&state, *id, options.display_offset, &self.on_switch))
            .collect();
        for button in rebuilt {
            self.container.append(&button);
        }
    }
}

fn workspace_button(
    state: &State,
    raw_id: i32,
    display_offset: i32,
    on_switch: &Option<Rc<dyn Fn(i32)>>,
) -> gtk4::Button {
    let button = gtk4::Button::new();
    button.add_css_class("workspace");
    button.set_valign(gtk4::Align::Center);
    let exists = state.real.contains(&raw_id);
    if !exists {
        button.add_css_class("empty");
    }
    if state.active_raw == raw_id {
        button.add_css_class("active");
    }
    let label = format!("{}", raw_id + display_offset);
    button.set_tooltip_text(Some(&format!("workspace {label}")));

    let content = gtk4::Box::new(
        if state.vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        },
        2,
    );
    if state.style == Style::Windows {
        for icon in window_icons(state, raw_id) {
            content.append(&icon);
        }
    }
    let number = gtk4::Label::new(Some(&label));
    number.set_css_classes(&["workspace-btn-label"]);
    number.set_halign(gtk4::Align::Center);
    number.set_valign(gtk4::Align::Center);
    number.set_hexpand(true);
    number.set_vexpand(true);
    let label_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    label_container.set_width_request(28);
    label_container.set_height_request(28);
    label_container.append(&number);
    content.append(&label_container);

    button.set_child(Some(&content));
    if let Some(on_switch) = on_switch {
        let on_switch = Rc::clone(on_switch);
        button.connect_clicked(move |_| on_switch(raw_id));
    }
    button
}

/// One 14px icon per window of the workspace
fn window_icons(state: &State, raw_id: i32) -> Vec<gtk4::Image> {
    state
        .windows
        .iter()
        .filter(|(workspace, _)| *workspace == raw_id)
        .map(|(_, class)| {
            let name = if has_icon(class) {
                class
            } else {
                FALLBACK_ICON
            };
            let icon = gtk4::Image::from_icon_name(name);
            icon.set_pixel_size(14);
            icon
        })
        .collect()
}

fn has_icon(name: &str) -> bool {
    gdk::Display::default()
        .map(|display| gtk4::IconTheme::for_display(&display).has_icon(name))
        .unwrap_or(false)
}
