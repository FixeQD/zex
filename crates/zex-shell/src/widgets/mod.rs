//! Bar module widgets

mod clock;
mod launcher_button;
mod notification;

pub use notification::NotificationWidget;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use gtk4::prelude::*;
use zex_core::Settings;
use zex_launcher::apps::{AppInfo, PinnedApps};
use zex_services::audio::VolumeControl;
use zex_services::compositor::{self, WindowInfo, WorkspaceInfo};
use zex_services::mpris::MprisPlayer;
use zex_services::tray::TrayItem;
use zex_services::upower::Battery;

use crate::bar::layout::Module;
use crate::bar::widgets::MprisControl;
use crate::bar::widgets::battery::BatteryWidget;
use crate::bar::widgets::media;
use crate::bar::widgets::systeminfotray::{SystemInfoTray, TrayControl};
use crate::bar::widgets::tasks::Tasks;
use crate::bar::widgets::window_info::WindowInfoWidget;
use crate::bar::widgets::workspaces::{Style, Workspaces, WorkspacesOptions};

pub use clock::{
    Clock, LocalTime, civil_from_days, format_date, format_month, format_time, local_time,
};

/// Snapshot shared by every bar window: the store callback writes it on every settings change
/// GTK thread reads it on refresh/tick
pub type SharedSettings = Rc<Mutex<Settings>>;

/// Shell-owned handles the dock and tray widgets operate through
pub struct DockDeps {
    pub on_quickcenter: Rc<dyn Fn()>,
    pub tray: TrayControl,
    pub volume: VolumeControl,
    pub apps: Vec<AppInfo>,
    pub pins: Rc<PinnedApps>,
}

pub struct Widgets {
    map: HashMap<Module, gtk4::Widget>,
    clock: Option<Clock>,
    workspaces: Option<Rc<Workspaces>>,
    window_info: Option<Rc<WindowInfoWidget>>,
    media: Option<Rc<media::Media>>,
    tasks: Option<Rc<Tasks>>,
    system_tray: Option<Rc<SystemInfoTray>>,
    settings: SharedSettings,
    /// Compositor handle for direct workspace switches; absent when no backend detected
    switcher: Option<Rc<dyn compositor::Compositor>>,
    /// Shared state refreshed on compositor and catalog events
    apps: RefCell<Vec<AppInfo>>,
    windows: RefCell<Vec<WindowInfo>>,
    active: RefCell<Option<WindowInfo>>,
}

impl Widgets {
    /// Build the registry from a shared settings snapshot
    /// The launcher button toggles the launcher overlay through the injected handler
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        settings: SharedSettings,
        on_launcher_clicked: impl Fn() + 'static,
        switcher: Option<Rc<dyn compositor::Compositor>>,
        media_control: MprisControl,
        display_offset: i32,
        deps: DockDeps,
    ) -> Self {
        let mut map = HashMap::new();
        let snapshot = settings.lock().expect("settings mutex poisoned").clone();
        let vertical = snapshot.interface.bar.vertical;
        let density = snapshot.interface.bar.density;

        let clock = Clock::new(settings.clone());
        map.insert(Module::Clock, clock.widget().upcast());
        let clock = Some(clock);

        let launcher = launcher_button::new(on_launcher_clicked);
        map.insert(Module::Launcher, launcher.upcast());

        let on_switch: Option<Rc<dyn Fn(i32)>> = switcher.as_ref().map(|compositor| {
            let compositor = Rc::clone(compositor);
            let closure: Rc<dyn Fn(i32)> = Rc::new(move |id| {
                if let Err(err) = compositor.switch_to_workspace(id) {
                    tracing::warn!("workspace switch failed: {err:#}");
                }
            });
            closure
        });
        let workspaces = Workspaces::new(vertical, display_offset, on_switch);
        map.insert(Module::Workspaces, workspaces.widget().upcast());
        let workspaces = Some(workspaces);

        let window_info = WindowInfoWidget::new();
        map.insert(Module::WindowInfo, window_info.widget().upcast());
        let window_info = Some(window_info);

        let media = media::Media::new(Rc::new(media_control));
        map.insert(Module::Media, media.widget().upcast());
        let media = Some(media);

        let on_focus: Option<Rc<dyn Fn(String)>> = switcher.clone().map(|compositor| {
            let closure: Rc<dyn Fn(String)> = Rc::new(move |address| {
                if let Err(err) = compositor.focus_window(&address) {
                    tracing::warn!("dock focus failed: {err:#}");
                }
            });
            closure
        });
        let tasks = Tasks::new(vertical, density, on_focus, Rc::clone(&deps.pins));
        map.insert(Module::Tasks, tasks.widget().upcast());
        let tasks = Some(tasks);

        let battery = BatteryWidget::new();
        let on_quickcenter = deps.on_quickcenter.clone();
        let system_tray = SystemInfoTray::new(
            vertical,
            move || on_quickcenter(),
            deps.tray.clone(),
            deps.volume.clone(),
            battery,
        );
        map.insert(Module::SystemInfoTray, system_tray.widget().upcast());
        let system_tray = Some(system_tray);

        Self {
            map,
            clock,
            workspaces,
            window_info,
            media,
            tasks,
            system_tray,
            settings,
            switcher,
            apps: RefCell::new(deps.apps.clone()),
            windows: RefCell::new(Vec::new()),
            active: RefCell::new(None),
        }
    }

    pub fn get(&self, module: Module) -> Option<gtk4::Widget> {
        self.map.get(&module).cloned()
    }

    /// Clock widget handle, for layout refreshes tied to bar settings
    pub fn clock(&self) -> Option<&Clock> {
        self.clock.as_ref()
    }

    /// Push compositor state; called on compositor events (GTK thread)
    pub fn on_compositor(
        &self,
        workspaces: &[WorkspaceInfo],
        windows: &[WindowInfo],
        active: Option<&WindowInfo>,
    ) {
        let snapshot = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        let style = Style::from_settings(&snapshot.interface.modules.options.workspaces_style);
        let fixed = snapshot.interface.modules.options.fixed_workspaces_enabled;
        let amount = snapshot.interface.modules.options.fixed_workspaces_amount as usize;
        let vertical = snapshot.interface.bar.vertical;
        if let Some(widget) = &self.workspaces {
            widget.update(
                workspaces,
                windows,
                WorkspacesOptions {
                    style,
                    fixed,
                    amount,
                    vertical,
                    display_offset: self.display_offset(),
                },
            );
        }
        if let Some(widget) = &self.window_info {
            let vertical = snapshot.interface.bar.vertical;
            let centered = self.is_centered(Module::WindowInfo);
            widget.update(active, vertical, centered, snapshot.interface.bar.density);
        }
        let active_owned = active.cloned();
        *self.windows.borrow_mut() = windows.to_vec();
        *self.active.borrow_mut() = active_owned;
        self.refresh_tasks(&snapshot);
    }

    /// Push MPRIS state; called on media events (GTK thread)
    pub fn on_media(&self, players: &[MprisPlayer]) {
        let snapshot = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        if let Some(widget) = &self.media {
            widget.update(
                players,
                snapshot.interface.bar.vertical,
                self.is_centered(Module::Media),
                snapshot.interface.bar.density,
            );
        }
    }

    /// Push the applications catalog; called after the launcher scan or a change event
    pub fn on_apps(&self, apps: &[AppInfo]) {
        *self.apps.borrow_mut() = apps.to_vec();
        let snapshot = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        self.refresh_tasks(&snapshot);
    }

    /// Rebuild the task dock after a pin change
    pub fn on_pins(&self) {
        let snapshot = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        self.refresh_tasks(&snapshot);
    }

    /// Push the quick settings toggle volume level
    pub fn on_volume(&self, volume: f32, muted: bool) {
        if let Some(tray) = &self.system_tray {
            tray.on_volume(volume, muted);
        }
    }

    /// Push battery snapshots from UPower
    pub fn on_batteries(&self, batteries: &[Battery]) {
        if let Some(tray) = &self.system_tray {
            tray.on_batteries(batteries);
        }
    }

    /// Push tray items; called on each dbusmenu host event (GTK thread)
    pub fn on_tray(&self, items: &[TrayItem]) {
        if let Some(tray) = &self.system_tray {
            tray.on_tray(items);
        }
    }

    /// Re-run the task dock against the last compositor snapshot
    fn refresh_tasks(&self, snapshot: &Settings) {
        if let Some(tasks) = &self.tasks {
            tasks.update(
                &self.apps.borrow(),
                &self.windows.borrow(),
                self.active.borrow().as_ref(),
                snapshot.interface.bar.vertical,
                &snapshot.interface.bar.side,
                snapshot.interface.bar.density,
            );
        }
    }
    fn is_centered(&self, module: Module) -> bool {
        use crate::bar::layout::{Area, PerModule};
        let snapshot = self.settings.lock().expect("settings mutex poisoned");
        Area::from_location(snapshot.interface.modules.location.value(module)) == Some(Area::Center)
    }

    /// Whether MPRIS currently has players; the bar window folds the media widget when not
    pub fn media_has_players(&self) -> bool {
        self.media.as_ref().is_some_and(|media| media.has_players())
    }

    /// Raw-id to 1-based display offset for this backend ("Niri" is 0-based)
    fn display_offset(&self) -> i32 {
        match self.switcher.as_ref().map(|c| c.name()) {
            Some("Niri") => 1,
            _ => 0,
        }
    }
}
