//! Widget registry integration tests.
//!
//! Widget construction needs a running GTK; when the environment is headless
//! (CI, no display) the tests skip so the pure engine tests still run.

use std::rc::Rc;
use std::sync::Mutex;

use gtk4::Align;
use gtk4::prelude::*;
use zex_core::Settings;
use zex_shell::bar::layout::Module;
use zex_shell::bar::widgets::MprisControl;
use zex_shell::widgets::{SharedSettings, Widgets};

fn gtk_available() -> bool {
    gtk4::init().is_ok()
}

fn build_widgets(settings: SharedSettings) -> Widgets {
    Widgets::build(
        settings,
        || {},
        None,
        MprisControl::new(flume::unbounded().0),
        0,
        zex_shell::widgets::DockDeps {
            on_quickcenter: Rc::new(|| {}),
            tray: zex_shell::bar::widgets::systeminfotray::TrayControl::new(flume::unbounded().0),
            volume: zex_services::audio::VolumeControl::default(),
            apps: Vec::new(),
            pins: Rc::new(zex_launcher::apps::PinnedApps::load(None)),
        },
    )
}

#[test]
fn registry_contains_implemented_modules_only() {
    if !gtk_available() {
        return;
    }
    let settings: SharedSettings = Rc::new(Mutex::new(Settings::default()));
    let widgets = build_widgets(settings);

    let implemented = [
        Module::Clock,
        Module::Launcher,
        Module::Workspaces,
        Module::WindowInfo,
        Module::Media,
        Module::Tasks,
        Module::SystemInfoTray,
    ];
    for module in implemented {
        assert!(widgets.get(module).is_some(), "{} missing", module.name());
    }
    for module in Module::ALL {
        if implemented.contains(&module) {
            continue;
        }
        assert!(
            widgets.get(module).is_none(),
            "{} not implemented yet",
            module.name()
        );
    }
}

#[test]
fn launcher_button_has_reference_classes_and_expansion() {
    if !gtk_available() {
        return;
    }
    let settings: SharedSettings = Rc::new(Mutex::new(Settings::default()));
    let widgets = build_widgets(settings);
    let button = widgets
        .get(Module::Launcher)
        .expect("launcher registered")
        .downcast::<gtk4::Button>()
        .expect("launcher is a button");

    assert_eq!(button.css_classes(), &["m3-icon", "launcher-button"]);
    assert_eq!(button.label(), Some("apps".into()));
    assert!(button.hexpands());
    assert!(button.vexpands());
    assert_eq!(button.halign(), Align::Fill);
    assert_eq!(button.valign(), Align::Fill);
}
