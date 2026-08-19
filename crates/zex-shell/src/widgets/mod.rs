//! Bar module widgets

mod clock;
mod launcher_button;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use gtk4::prelude::*;
use zex_core::Settings;
use zex_services::compositor::{self, WindowInfo, WorkspaceInfo};
use zex_services::mpris::MprisPlayer;

use crate::bar::layout::Module;
use crate::bar::widgets::MprisControl;
use crate::bar::widgets::media;
use crate::bar::widgets::window_info::WindowInfoWidget;
use crate::bar::widgets::workspaces::{Style, Workspaces, WorkspacesOptions};

pub use clock::{
    Clock, LocalTime, civil_from_days, format_date, format_month, format_time, local_time,
};

/// Snapshot shared by every bar window: the store callback writes it on every settings change
/// GTK thread reads it on refresh/tick
pub type SharedSettings = Rc<Mutex<Settings>>;

pub struct Widgets {
    map: HashMap<Module, gtk4::Widget>,
    clock: Option<Clock>,
    workspaces: Option<Rc<Workspaces>>,
    window_info: Option<Rc<WindowInfoWidget>>,
    media: Option<Rc<media::Media>>,
    settings: SharedSettings,
    /// Compositor handle for direct workspace switches; absent when no backend detected
    switcher: Option<Rc<dyn compositor::Compositor>>,
}

impl Widgets {
    /// Build the registry from a shared settings snapshot
    /// The launcher button toggles the launcher overlay through the injected handler
    pub fn build(
        settings: SharedSettings,
        on_launcher_clicked: impl Fn() + 'static,
        switcher: Option<Rc<dyn compositor::Compositor>>,
        media_control: MprisControl,
        display_offset: i32,
    ) -> Self {
        let mut map = HashMap::new();
        let snapshot = settings.lock().expect("settings mutex poisoned").clone();
        let vertical = snapshot.interface.bar.vertical;

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

        Self {
            map,
            clock,
            workspaces,
            window_info,
            media,
            settings,
            switcher,
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
