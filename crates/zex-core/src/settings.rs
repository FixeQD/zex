//! File layout (`~/.config/zex/settings.json`):
//!
//! ```json
//! {
//!   "appearance": { "wallcolors": { "wallpaper_path": "", ... } },
//!   "interface":  { "modules": { ... }, "bar": { ... }, "bar2": { ... }, ... },
//!   "services":   { "recorder": { ... }, "osd": { ... }, "lockscreen": { ... } }
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Root of the settings file.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub appearance: Appearance,
    pub interface: Interface,
    pub services: Services,
}

/// Wallpaper / colours group
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub wallcolors: WallpaperColors,
}

/// Wallpaper and material-colours options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WallpaperColors {
    pub quickselect_path: String,
    pub wallpaper_path: String,
    /// matugen scheme: `tonal_spot`, `content`, `expressive`, `neutral`, `monochrome`, `rainbow`, `fidelity`.
    pub color_scheme: String,
    pub dark_mode: bool,
    pub auto_dark: AutoDark,
}

impl Default for WallpaperColors {
    fn default() -> Self {
        Self {
            quickselect_path: String::new(),
            wallpaper_path: String::new(),
            color_scheme: "tonal_spot".to_string(),
            dark_mode: true,
            auto_dark: AutoDark::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AutoDark {
    pub enabled: bool,
    pub start_hour: u8,
    pub start_min: u8,
    pub end_hour: u8,
    pub end_min: u8,
}

impl Default for AutoDark {
    fn default() -> Self {
        Self {
            enabled: false,
            start_hour: 22,
            start_min: 0,
            end_hour: 6,
            end_min: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Interface {
    pub modules: Modules,
    pub bar: Bar,
    pub bar2: Bar2,
    pub notifications: Notifications,
    pub launcher: Launcher,
    pub misc: Misc,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Modules {
    /// Which bar segment each module sits in (0 = launcher/left, 1 = center, 2 = tray/right)
    pub location: Locations,
    pub visibility: Visibility,
    /// Which bar instance hosts a module when `bar2.enabled` (0 = bar, 1 = bar2)
    pub bar_id: BarId,
    pub options: ModuleOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Locations {
    pub launcher: u8,
    pub window_info: u8,
    pub media: u8,
    pub workspaces: u8,
    pub tasks: u8,
    pub recording_indicator: u8,
    pub systeminfotray: u8,
    pub clock: u8,
}

impl Default for Locations {
    fn default() -> Self {
        Self {
            launcher: 0,
            window_info: 0,
            media: 0,
            workspaces: 1,
            tasks: 1,
            recording_indicator: 2,
            systeminfotray: 2,
            clock: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Visibility {
    pub window_info: bool,
    pub media: bool,
    pub workspaces: bool,
    pub recording_indicator: bool,
    pub systeminfotray: bool,
    pub clock: bool,
    pub tasks: bool,
    pub launcher: bool,
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            window_info: true,
            media: true,
            workspaces: true,
            recording_indicator: true,
            systeminfotray: true,
            clock: true,
            tasks: false,
            launcher: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarId {
    pub launcher: u8,
    pub window_info: u8,
    pub media: u8,
    pub workspaces: u8,
    pub tasks: u8,
    pub recording_indicator: u8,
    pub systeminfotray: u8,
    pub clock: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModuleOptions {
    pub show_date: bool,
    pub day_month_swapped: bool,
    pub military_time: bool,
    /// OSD label shown by the recorder: `recording` or `paused`
    pub recording_indicator: String,
    /// `numbers` or `dots`.
    pub workspaces_style: String,
    pub fixed_workspaces_enabled: bool,
    pub fixed_workspaces_amount: u8,
}

impl Default for ModuleOptions {
    fn default() -> Self {
        Self {
            show_date: true,
            day_month_swapped: false,
            military_time: false,
            recording_indicator: "recording".to_string(),
            workspaces_style: "numbers".to_string(),
            fixed_workspaces_enabled: true,
            fixed_workspaces_amount: 5,
        }
    }
}

/// Primary bar
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bar {
    /// `top` or `bottom`
    pub side: String,
    pub vertical: bool,
    /// Negative = compact, 0 = default, positive = expanded
    pub density: i8,
    pub floating: bool,
    pub separation: bool,
    pub centered: bool,
    pub bar_background: bool,
    pub module_backgrounds: bool,
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            side: "bottom".to_string(),
            vertical: false,
            density: 0,
            floating: false,
            separation: false,
            centered: false,
            bar_background: true,
            module_backgrounds: true,
        }
    }
}

/// Secondary bar (top by default)
/// hosts the modules whose `bar_id` is 1
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bar2 {
    pub enabled: bool,
    pub side: String,
    pub vertical: bool,
    pub density: i8,
    pub floating: bool,
    pub separation: bool,
    pub centered: bool,
    pub bar_background: bool,
    pub module_backgrounds: bool,
}

impl Default for Bar2 {
    fn default() -> Self {
        Self {
            enabled: true,
            side: "top".to_string(),
            vertical: false,
            density: 0,
            floating: false,
            separation: false,
            centered: false,
            bar_background: true,
            module_backgrounds: true,
        }
    }
}

/// Screen edge a notification popup is anchored to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Notifications {
    /// Popup anchor corners, e.g. `["top", "right"]` for the top-right
    pub anchor: Vec<Anchor>,
    pub compact_popup: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            anchor: vec![Anchor::Top, Anchor::Right],
            compact_popup: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Launcher {
    /// `grid` or `list`
    pub layout: String,
    pub ai: Ai,
    pub clipboard: Clipboard,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            layout: "grid".to_string(),
            ai: Ai::default(),
            clipboard: Clipboard::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ai {
    /// Ollama HTTP endpoint
    pub endpoint: String,
    /// Model served by the endpoint
    pub model: String,
    /// Sampling temperature
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: "Qwythos-9B-v2:latest".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            system_prompt: "You are a concise, helpful assistant.".to_string(),
        }
    }
}

/// Clipboard history options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clipboard {
    pub history_limit: usize,
    pub keep_passwords: bool,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            history_limit: 500,
            keep_passwords: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Misc {
    /// Rounded shell corners on windows without a radius of their own
    pub shell_corners: bool,
    /// When screen corners are applied: `not_fullscreen` or `always`
    pub screen_corners: String,
}

impl Default for Misc {
    fn default() -> Self {
        Self {
            shell_corners: true,
            screen_corners: "not_fullscreen".to_string(),
        }
    }
}

/// Service options
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Services {
    pub recorder: Recorder,
    pub osd: Osd,
    pub lockscreen: Lockscreen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Recorder {
    pub start_notification: bool,
    pub stop_notification: bool,
    pub record_audio: bool,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            start_notification: true,
            stop_notification: true,
            record_audio: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Osd {
    pub anchor: Vec<Anchor>,
    pub vertical: bool,
}

impl Default for Osd {
    fn default() -> Self {
        Self {
            anchor: vec![Anchor::Bottom, Anchor::Right],
            vertical: false,
        }
    }
}

/// Lockscreen options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lockscreen {
    pub blur: bool,
    pub clock: bool,
}

impl Default for Lockscreen {
    fn default() -> Self {
        Self {
            blur: true,
            clock: true,
        }
    }
}
