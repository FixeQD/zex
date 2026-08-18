//! Module layout engine

use zex_core::settings::{BarId, Locations, Modules, Visibility};

/// Every module the bar can host, in the canonical reference order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    Launcher,
    WindowInfo,
    Media,
    Workspaces,
    Tasks,
    RecordingIndicator,
    SystemInfoTray,
    Clock,
}

impl Module {
    pub const ALL: [Module; 8] = [
        Module::Launcher,
        Module::WindowInfo,
        Module::Media,
        Module::Workspaces,
        Module::Tasks,
        Module::RecordingIndicator,
        Module::SystemInfoTray,
        Module::Clock,
    ];

    /// Settings key / registry name of the module
    pub const fn name(self) -> &'static str {
        match self {
            Module::Launcher => "launcher",
            Module::WindowInfo => "window_info",
            Module::Media => "media",
            Module::Workspaces => "workspaces",
            Module::Tasks => "tasks",
            Module::RecordingIndicator => "recording_indicator",
            Module::SystemInfoTray => "systeminfotray",
            Module::Clock => "clock",
        }
    }
}

/// One of the three bar segments: left (0), center (1), right (2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Left,
    Center,
    Right,
}

impl Area {
    pub const ALL: [Area; 3] = [Area::Left, Area::Center, Area::Right];

    pub fn from_location(location: u8) -> Option<Area> {
        match location {
            0 => Some(Area::Left),
            1 => Some(Area::Center),
            2 => Some(Area::Right),
            _ => None,
        }
    }

    /// CSS class of the segment box
    pub const fn as_css_class(self) -> &'static str {
        match self {
            Area::Left => "left-widgets",
            Area::Center => "center-widgets",
            Area::Right => "right-widgets",
        }
    }
}

/// Per-module accessors for the three `interface.modules` sub-groups
pub trait PerModule<T> {
    fn value(&self, module: Module) -> T;
}

impl PerModule<u8> for Locations {
    fn value(&self, module: Module) -> u8 {
        match module {
            Module::Launcher => self.launcher,
            Module::WindowInfo => self.window_info,
            Module::Media => self.media,
            Module::Workspaces => self.workspaces,
            Module::Tasks => self.tasks,
            Module::RecordingIndicator => self.recording_indicator,
            Module::SystemInfoTray => self.systeminfotray,
            Module::Clock => self.clock,
        }
    }
}

impl PerModule<bool> for Visibility {
    fn value(&self, module: Module) -> bool {
        match module {
            Module::Launcher => self.launcher,
            Module::WindowInfo => self.window_info,
            Module::Media => self.media,
            Module::Workspaces => self.workspaces,
            Module::Tasks => self.tasks,
            Module::RecordingIndicator => self.recording_indicator,
            Module::SystemInfoTray => self.systeminfotray,
            Module::Clock => self.clock,
        }
    }
}

impl PerModule<u8> for BarId {
    fn value(&self, module: Module) -> u8 {
        match module {
            Module::Launcher => self.launcher,
            Module::WindowInfo => self.window_info,
            Module::Media => self.media,
            Module::Workspaces => self.workspaces,
            Module::Tasks => self.tasks,
            Module::RecordingIndicator => self.recording_indicator,
            Module::SystemInfoTray => self.systeminfotray,
            Module::Clock => self.clock,
        }
    }
}

/// Where a single module lives in the shell: segment + bar instance + visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub module: Module,
    pub area: Area,
    pub bar_id: u8,
    pub visible: bool,
}

/// The full bar arrangement derived from the settings snapshot
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    placements: Vec<Placement>,
}

impl Layout {
    /// Derive the layout from `interface.modules`
    pub fn new(modules: &Modules) -> Self {
        let mut placements = Vec::with_capacity(Module::ALL.len());
        for module in Module::ALL {
            let location = modules.location.value(module);
            let Some(area) = Area::from_location(location) else {
                tracing::warn!(
                    "module {}: invalid location {location}, skipping",
                    module.name()
                );
                continue;
            };
            let bar_id = modules.bar_id.value(module);
            if bar_id > 1 {
                tracing::warn!(
                    "module {}: invalid bar id {bar_id}, skipping",
                    module.name()
                );
                continue;
            }
            placements.push(Placement {
                module,
                area,
                bar_id,
                visible: modules.visibility.value(module),
            });
        }
        Self { placements }
    }

    /// All placements, in canonical module order
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// Placements hosted by one bar instance, in module order
    pub fn for_bar(&self, bar_id: u8) -> impl Iterator<Item = Placement> + '_ {
        self.placements
            .iter()
            .copied()
            .filter(move |p| p.bar_id == bar_id)
    }

    /// Whether a bar instance hosts at least one module
    pub fn bar_in_use(&self, bar_id: u8) -> bool {
        self.placements.iter().any(|p| p.bar_id == bar_id)
    }

    /// True when no module is placed anywhere
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }
}
