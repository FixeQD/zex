//! The launcher overlay

mod icons;
mod keys;
mod nav;
mod rows;
pub mod window;

use std::rc::Rc;

use gtk4::prelude::*;
use relm4::prelude::*;
use window::LauncherWindow;
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_launcher::apps::{self, AppInfo, PinnedApps};
use zex_launcher::engine::{Matcher, organize};
use zex_launcher::items::{Item, dispatch};

pub use nav::grid_step;
pub use rows::{GRID_COLUMNS, ItemRow, PINNED_PER_ROW, item_row, pinned_button};

pub const LAUNCHER_CSS_SCSS: &str = include_str!("../../assets/css/launcher.scss");

#[derive(Debug)]
pub enum LauncherMsg {
    Toggle,
    QueryChanged(String),
    ClearQuery,
    ToggleLayout,
    Move { delta: i32, wrap: bool },
    ActivateAt(usize),
    Activate,
    Close,
    Back,
    Pin { id: String },
    LaunchApp { id: String },
    LaunchAction { command: String },
    AppsRescan,
    PinsChanged(Vec<String>),
    SettingsChanged(Box<zex_core::Settings>),
}

pub struct Launcher {
    store: SettingsStore,
    window: LauncherWindow,
    layout: String,
    apps: Vec<AppInfo>,
    catalog: Vec<Item>,
    pins: Rc<PinnedApps>,
    matcher: Matcher,
    rows: Vec<Item>,
    row_widgets: Vec<ItemRow>,
    selection: usize,
    query: String,
    visible: bool,
    subscription: Option<Subscription>,
    _pins_rx: flume::Receiver<Vec<String>>,
    _provider: Option<gtk4::CssProvider>,
    sender: ComponentSender<Launcher>,
}

impl Launcher {
    fn grid(&self) -> bool {
        self.layout == "grid"
    }

    fn rebuild_pins(&mut self) {
        let grid = &self.window.pinned;
        while let Some(child) = grid.first_child() {
            grid.remove(&child);
        }
        let ids = self.pins.pinned();
        if ids.is_empty() {
            grid.set_halign(gtk4::Align::Center);
            grid.attach(&self.window.pin_hint_icon, 0, 0, 1, 1);
            grid.attach(&self.window.pin_hint_label, 0, 1, 1, 1);
            return;
        }
        grid.set_halign(gtk4::Align::Start);
        let sender = self.sender.input_sender().clone();
        for (index, id) in ids.iter().enumerate() {
            let Some(app) = self.apps.iter().find(|app| app.id == *id) else {
                continue;
            };
            let button = pinned_button(
                app,
                Rc::clone(&self.pins),
                self.window.popover_open_cell(),
                sender.clone(),
            );
            grid.attach(
                &button,
                (index as i32) % PINNED_PER_ROW,
                (index as i32) / PINNED_PER_ROW,
                1,
                1,
            );
        }
    }

    fn clear_rows(&mut self) {
        while let Some(child) = self.window.list.first_child() {
            self.window.list.remove(&child);
        }
        while let Some(child) = self.window.flow.first_child() {
            self.window.flow.remove(&child);
        }
        self.row_widgets.clear();
        self.rows.clear();
    }

    fn rerender_results(&mut self, keep_selection: bool) {
        let has_query = !self.query.trim().is_empty();
        self.window.clear.button.set_visible(has_query);
        self.window.layout_toggle.button.set_visible(has_query);
        self.window.pinned_revealer.set_reveal_child(!has_query);
        self.window.results_revealer.set_reveal_child(has_query);

        self.clear_rows();
        if !has_query {
            self.selection = 0;
            return;
        }

        let grid = self.grid();
        self.window.list.set_visible(!grid);
        self.window.flow.set_visible(grid);

        let sections = organize(self.catalog.clone(), &self.query, &self.matcher);
        let rows: Vec<Item> = sections.into_iter().flat_map(|s| s.items).collect();
        if rows.is_empty() {
            self.selection = 0;
            return;
        }

        let sender = self.sender.input_sender().clone();
        let pins = Rc::clone(&self.pins);
        for (index, item) in rows.iter().enumerate() {
            let featured = index == 0;
            let row = item_row(
                item,
                index,
                grid && !featured,
                featured,
                pins.clone(),
                self.window.popover_open_cell(),
                sender.clone(),
            );
            if grid && !featured {
                self.window.flow.append(&row.container);
            } else {
                self.window.list.append(&row.container);
            }
            self.row_widgets.push(row);
        }
        self.rows = rows;

        let target = if keep_selection { self.selection } else { 0 };
        self.set_selection(target);
    }

    fn set_selection(&mut self, selection: usize) {
        let count = self.rows.len();
        self.selection = if count == 0 {
            0
        } else {
            selection.min(count - 1)
        };
        for (index, row) in self.row_widgets.iter().enumerate() {
            if index == self.selection {
                row.button.add_css_class("selected");
            } else {
                row.button.remove_css_class("selected");
            }
        }
        if let Some(row) = self.row_widgets.get(self.selection) {
            self.window.viewport.scroll_to(&row.container, None);
        }
    }

    fn move_selection(&mut self, delta: i32, wrap: bool) {
        if self.rows.is_empty() {
            return;
        }
        let count = self.rows.len();
        let step = if self.grid() && delta.abs() == 1 {
            GRID_COLUMNS as i32
        } else {
            delta
        };
        let next = if self.grid() && delta.abs() > 1 {
            grid_step(count, GRID_COLUMNS, self.selection, delta, wrap)
        } else if wrap {
            let target = (self.selection as i64 + i64::from(step)) % count as i64;
            target.rem_euclid(count as i64) as usize
        } else {
            (self.selection as i64 + i64::from(step)).clamp(0, count as i64 - 1) as usize
        };
        self.set_selection(next);
    }

    fn activate(&mut self) {
        let Some(item) = self.rows.get(self.selection).cloned() else {
            return;
        };
        self.activate_item(&item);
        self.hide();
    }

    fn activate_item(&self, item: &Item) {
        if let Err(err) = dispatch(item) {
            tracing::warn!("launcher dispatch failed: {err:#}");
        }
    }

    fn launch_by_id(&mut self, id: &str) {
        let Some(app) = self.apps.iter().find(|app| app.id == id) else {
            tracing::warn!("launcher: no application for id {id}");
            return;
        };
        self.activate_item(&Item::App(app.clone()));
        self.hide();
    }

    fn show(&mut self) {
        if self.visible {
            return;
        }
        self.visible = true;
        self.window.show();
        self.rebuild_pins();
        self.rerender_results(false);
    }

    fn hide(&mut self) {
        if !self.visible {
            return;
        }
        self.visible = false;
        self.window.hide();
    }

    fn sync_layout_icon(&self) {
        let icon = if self.grid() {
            "view-list-symbolic"
        } else {
            "view-grid-symbolic"
        };
        self.window.layout_toggle.set_icon(Some(icon));
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Launcher {
    type Init = (SettingsStore, Rc<crate::shared::ActionHandles>);
    type Input = LauncherMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        (store, actions): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let window = LauncherWindow::new(sender.input_sender().clone());

        let layout = store.get().interface.launcher.layout.clone();
        let mut model = Self {
            store,
            window,
            layout,
            apps: Vec::new(),
            catalog: Vec::new(),
            pins: Rc::new(PinnedApps::load(None)),
            matcher: Matcher::new(),
            rows: Vec::new(),
            row_widgets: Vec::new(),
            selection: 0,
            query: String::new(),
            visible: false,
            subscription: None,
            _pins_rx: flume::unbounded().1,
            _provider: None,
            sender: sender.clone(),
        };

        model.apps = match apps::load_apps(Some(&apps::default_store_path())) {
            Ok(catalog) => catalog,
            Err(err) => {
                tracing::warn!("launcher scan failed: {err:#}");
                Vec::new()
            }
        };
        model.catalog = model.apps.iter().cloned().map(Item::App).collect();

        if let Ok(watchdog) = apps::Watchdog::start() {
            let watchdog_sender = sender.clone();
            std::thread::spawn(move || {
                loop {
                    let changes = watchdog.next(std::time::Duration::from_secs(30));
                    if !changes.is_empty() {
                        watchdog_sender.input(LauncherMsg::AppsRescan);
                    }
                }
            });
        }

        model._pins_rx = model.pins.changes();
        let pins_rx = model._pins_rx.clone();
        let pins_sender = sender.clone();
        std::thread::spawn(move || {
            while let Ok(ids) = pins_rx.recv() {
                pins_sender.input(LauncherMsg::PinsChanged(ids));
            }
        });

        model._provider = Some(crate::shared::install_css_provider(LAUNCHER_CSS_SCSS));

        let subscription = crate::shared::subscribe_settings(
            &model.store,
            sender.input_sender().clone(),
            |snapshot| LauncherMsg::SettingsChanged(Box::new(snapshot.clone())),
        );
        model.subscription = Some(subscription);

        model.sync_layout_icon();
        model.rebuild_pins();

        let toggle = {
            let sender = sender.input_sender().clone();
            move || {
                let _ = sender.send(LauncherMsg::Toggle);
            }
        };
        actions.set_launcher(toggle);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            LauncherMsg::Toggle => {
                if self.visible {
                    self.hide();
                } else {
                    self.show();
                }
            }
            LauncherMsg::QueryChanged(text) => {
                self.query = text;
                self.rerender_results(false);
            }
            LauncherMsg::ClearQuery => {
                self.window.entry.set_text("");
                self.window.entry.grab_focus();
                self.query.clear();
                self.rerender_results(false);
            }
            LauncherMsg::ToggleLayout => {
                let next = if self.grid() { "list" } else { "grid" };
                if let Err(err) = self
                    .store
                    .update(|settings| settings.interface.launcher.layout = next.to_string())
                {
                    tracing::warn!("layout persistence failed: {err:#}");
                }
                self.layout = next.to_string();
                self.sync_layout_icon();
                self.rerender_results(true);
            }
            LauncherMsg::Move { delta, wrap } => self.move_selection(delta, wrap),
            LauncherMsg::ActivateAt(index) => {
                if index < self.rows.len() {
                    self.set_selection(index);
                    self.activate();
                }
            }
            LauncherMsg::Activate => self.activate(),
            LauncherMsg::Close | LauncherMsg::Back => self.hide(),
            LauncherMsg::Pin { id } => self.pins.toggle(&id),
            LauncherMsg::LaunchApp { id } => self.launch_by_id(&id),
            LauncherMsg::LaunchAction { command } => {
                self.activate_item(&Item::Command(command));
                self.hide();
            }
            LauncherMsg::AppsRescan => {
                self.apps = match apps::load_apps(Some(&apps::default_store_path())) {
                    Ok(catalog) => catalog,
                    Err(err) => {
                        tracing::warn!("launcher re-scan failed: {err:#}");
                        return;
                    }
                };
                self.catalog = self.apps.iter().cloned().map(Item::App).collect();
                self.rebuild_pins();
                if !self.query.trim().is_empty() {
                    self.rerender_results(true);
                }
            }
            LauncherMsg::PinsChanged(_ids) => self.rebuild_pins(),
            LauncherMsg::SettingsChanged(snapshot) => {
                self.layout = snapshot.interface.launcher.layout.clone();
                self.sync_layout_icon();
                if !self.query.trim().is_empty() {
                    self.rerender_results(true);
                }
            }
        }
    }
}
