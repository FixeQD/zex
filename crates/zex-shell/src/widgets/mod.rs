//! Bar module widgets

mod clock;
mod launcher_button;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use gtk4::prelude::*;
use zex_core::Settings;

use crate::bar::layout::Module;

pub use clock::{
    Clock, LocalTime, civil_from_days, format_date, format_month, format_time, local_time,
};

/// Snapshot shared by every bar window: the store callback writes it on every settings change
/// GTK thread reads it on refresh/tick
pub type SharedSettings = Rc<Mutex<Settings>>;

pub struct Widgets {
    map: HashMap<Module, gtk4::Widget>,
    clock: Option<Clock>,
}

impl Widgets {
    /// Build the registry from a shared settings snapshot
    /// The launcher button toggles the launcher overlay through the injected handler
    pub fn build(settings: SharedSettings, on_launcher_clicked: impl Fn() + 'static) -> Self {
        let mut map = HashMap::new();

        let clock = Clock::new(settings.clone());
        map.insert(Module::Clock, clock.widget().upcast());
        let clock = Some(clock);

        let launcher = launcher_button::new(on_launcher_clicked);
        map.insert(Module::Launcher, launcher.upcast());

        Self { map, clock }
    }

    pub fn get(&self, module: Module) -> Option<gtk4::Widget> {
        self.map.get(&module).cloned()
    }

    /// Clock widget handle, for layout refreshes tied to bar settings
    pub fn clock(&self) -> Option<&Clock> {
        self.clock.as_ref()
    }
}
