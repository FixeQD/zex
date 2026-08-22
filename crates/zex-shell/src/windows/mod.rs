pub mod bar;
pub mod launcher;
pub mod quickcenter;
pub mod osd;
pub mod powermenu;
pub mod lockscreen;
pub mod wallpaper;
pub mod corners;
pub mod notifications;
pub mod settings;

use iced::window::Id as IcedId;
use iced_exwlshell::reexport::{
    Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};
use zex_core::Settings;

use crate::app::{BaseWindowKind, WindowKind};

fn thickness_for_density(density: i8) -> i32 {
    match density {
        -1 => 35,
        -2 => 30,
        -3 => 25,
        0 => 40,
        _ => 40,
    }
}

fn anchor_from_side(side: &str) -> Anchor {
    match side {
        "top" => Anchor::Top | Anchor::Left | Anchor::Right,
        "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
        "left" => Anchor::Left | Anchor::Top | Anchor::Bottom,
        "right" => Anchor::Right | Anchor::Top | Anchor::Bottom,
        _ => Anchor::Bottom | Anchor::Left | Anchor::Right,
    }
}

fn anchor_from_vec(v: &[zex_core::settings::Anchor]) -> Anchor {
    let mut a = Anchor::empty();
    for e in v {
        a |= match e {
            zex_core::settings::Anchor::Top => Anchor::Top,
            zex_core::settings::Anchor::Right => Anchor::Right,
            zex_core::settings::Anchor::Bottom => Anchor::Bottom,
            zex_core::settings::Anchor::Left => Anchor::Left,
        };
    }
    if a.is_empty() {
        Anchor::Bottom | Anchor::Right
    } else {
        a
    }
}

/// Factory: translate a high-level WindowKind into low-level NewLayerShellSettings.
pub fn layer_settings_for(kind: &WindowKind, cfg: &Settings) -> NewLayerShellSettings {
    match kind {
        WindowKind::Bar { monitor, bar_id } => {
            let (side, density) = if *bar_id == 1 {
                (cfg.interface.bar2.side.as_str(), cfg.interface.bar2.density)
            } else {
                (cfg.interface.bar.side.as_str(), cfg.interface.bar.density)
            };
            NewLayerShellSettings {
                anchor: anchor_from_side(side),
                layer: Layer::Top,
                exclusive_zone: Some(thickness_for_density(density)),
                size: None,
                margin: None,
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::OutputName(format!("monitor-{}", monitor)),
                events_transparent: false,
                namespace: Some(if *bar_id == 0 {
                    format!("zex-bar-{}", monitor)
                } else {
                    format!("zex-bar2-{}", monitor)
                }),
            }
        }
        WindowKind::Wallpaper { monitor } => NewLayerShellSettings {
            anchor: Anchor::all(),
            layer: Layer::Background,
            exclusive_zone: None,
            size: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            output_option: OutputOption::OutputName(format!("monitor-{}", monitor)),
            events_transparent: true,
            namespace: Some(format!("zex-wallpaper-{}", monitor)),
        },
        WindowKind::Launcher => NewLayerShellSettings {
            anchor: Anchor::all(),
            layer: Layer::Overlay,
            exclusive_zone: None,
            size: Some((600, 500)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            output_option: OutputOption::LastOutput,
            events_transparent: false,
            namespace: Some("zex-launcher".into()),
        },
        WindowKind::QuickCenter => NewLayerShellSettings {
            anchor: Anchor::Right | Anchor::Top | Anchor::Bottom,
            layer: Layer::Overlay,
            exclusive_zone: None,
            size: Some((400, 600)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            output_option: OutputOption::LastOutput,
            events_transparent: false,
            namespace: Some("zex-quickcenter".into()),
        },
        WindowKind::Osd { .. } => {
            let anchor = anchor_from_vec(&cfg.services.osd.anchor);
            NewLayerShellSettings {
                anchor,
                layer: Layer::Overlay,
                exclusive_zone: None,
                size: Some((300, 80)),
                margin: Some((10, 10, 10, 10)),
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::LastOutput,
                events_transparent: false,
                namespace: Some("zex-osd".into()),
            }
        }
        WindowKind::PowerMenu => NewLayerShellSettings {
            anchor: Anchor::all(),
            layer: Layer::Overlay,
            exclusive_zone: None,
            size: Some((400, 300)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            output_option: OutputOption::LastOutput,
            events_transparent: false,
            namespace: Some("zex-powermenu".into()),
        },
        WindowKind::Notification { monitor } => {
            let anchor = anchor_from_vec(&cfg.interface.notifications.anchor);
            NewLayerShellSettings {
                anchor,
                layer: Layer::Overlay,
                exclusive_zone: None,
                size: None,
                margin: Some((10, 10, 10, 10)),
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::OutputName(format!("monitor-{}", monitor)),
                events_transparent: false,
                namespace: Some(format!("zex-notifications-{}", monitor)),
            }
        }
        WindowKind::Corner { name } => {
            let anchor = match name.as_str() {
                "top-left" => Anchor::Top | Anchor::Left,
                "top-right" => Anchor::Top | Anchor::Right,
                "bottom-left" => Anchor::Bottom | Anchor::Left,
                "bottom-right" => Anchor::Bottom | Anchor::Right,
                _ => Anchor::Top | Anchor::Left,
            };
            NewLayerShellSettings {
                anchor,
                layer: Layer::Top,
                exclusive_zone: None,
                size: Some((25, 25)),
                margin: None,
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::LastOutput,
                events_transparent: true,
                namespace: Some(format!("zex-corner-{}", name)),
            }
        }
    }
}

pub fn base_settings_for(kind: &BaseWindowKind) -> iced_exwlshell::actions::IcedXdgWindowSettings {
    match kind {
        BaseWindowKind::Settings => iced_exwlshell::actions::IcedXdgWindowSettings {
            size: None,
            client_side_decorations: true,
        },
    }
}

pub fn create_layer_shell(kind: WindowKind, cfg: &Settings, id: IcedId) -> crate::app::Message {
    let settings = layer_settings_for(&kind, cfg);
    crate::app::Message::NewLayerShell(settings, id)
}

pub fn create_base_window(kind: BaseWindowKind, id: IcedId) -> crate::app::Message {
    let settings = base_settings_for(&kind);
    crate::app::Message::NewBaseWindow(settings, id)
}

pub fn create_session_lock(_id: IcedId) -> crate::app::Message {
    crate::app::Message::DoLock
}

pub fn close_window_action(id: IcedId) -> crate::app::Message {
    crate::app::Message::RemoveWindow(id)
}
