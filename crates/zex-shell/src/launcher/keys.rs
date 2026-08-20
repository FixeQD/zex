//! Keyboard contract and focus handling for the launcher window

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use relm4::Sender;

use super::LauncherMsg;
use super::window::LauncherWindow;

pub fn wire(window: &LauncherWindow, sender: Sender<LauncherMsg>) {
    let entry = window.entry.clone();
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let key_sender = sender.clone();
    keys.connect_key_pressed(move |_, key, _keycode, state| {
        if state.contains(gdk::ModifierType::CONTROL_MASK)
            || state.contains(gdk::ModifierType::ALT_MASK)
        {
            return glib::Propagation::Proceed;
        }
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        match key {
            gdk::Key::Escape => {
                let _ = key_sender.send(LauncherMsg::Close);
                glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                let _ = key_sender.send(LauncherMsg::Activate);
                glib::Propagation::Stop
            }
            gdk::Key::Up | gdk::Key::Down => {
                let delta = if key == gdk::Key::Down { 1 } else { -1 };
                let _ = key_sender.send(LauncherMsg::Move { delta, wrap: false });
                glib::Propagation::Stop
            }
            gdk::Key::Tab => {
                let _ = key_sender.send(LauncherMsg::Move {
                    delta: if shift { -1 } else { 1 },
                    wrap: true,
                });
                glib::Propagation::Stop
            }
            gdk::Key::BackSpace if entry.text().is_empty() => {
                let _ = key_sender.send(LauncherMsg::Back);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    window.root.add_controller(keys);

    let sender = sender.clone();
    let guard = window.popover_open.clone();
    let root = window.root.clone();
    root.connect_notify_local(Some("focus-widget"), move |root, _| {
        let focused = <gtk4::Window as gtk4::prelude::RootExt>::focus(root);
        if root.is_visible() && focused.is_none() && !guard.get() {
            let _ = sender.send(LauncherMsg::Close);
        }
    });
}
