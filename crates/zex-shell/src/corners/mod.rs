//! Corner warps: small layer-shell masks for bar ends and screen corners.

mod spec;
mod window;

use std::rc::Rc;
use std::sync::Mutex;

use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use relm4::prelude::*;
use zex_core::SettingsStore;
use zex_core::store::Subscription;

use crate::shared;

pub use spec::{CornerKind, CornerSpec, DEFAULT_WARP, corner_specs, warp_size};

#[derive(Debug)]
pub enum CornersMsg {
    SettingsChanged(Box<zex_core::Settings>),
    MonitorsChanged,
}

pub struct Corners {
    windows: Vec<gtk4::Window>,
    settings: Rc<Mutex<zex_core::Settings>>,
    subscription: Option<Subscription>,
    monitors_model: Option<gio::ListModel>,
}

#[relm4::component(pub)]
impl SimpleComponent for Corners {
    type Init = SettingsStore;
    type Input = CornersMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        store: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let mut model = Self {
            windows: Vec::new(),
            settings: Rc::new(Mutex::new(store.get().clone())),
            subscription: None,
            monitors_model: None,
        };

        if let Some(display) = gdk::Display::default() {
            let monitors_model =
                shared::watch_monitors(&display, sender.input_sender().clone(), || {
                    CornersMsg::MonitorsChanged
                });
            model.monitors_model = Some(monitors_model);
            model.rebuild();
        } else {
            tracing::warn!("no display available; corner warps stay off");
        }

        let subscription = shared::subscribe_settings(&store, sender.input_sender().clone(), |s| {
            CornersMsg::SettingsChanged(Box::new(s.clone()))
        });
        model.subscription = Some(subscription);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            CornersMsg::SettingsChanged(snapshot) => {
                *self.settings.lock().expect("settings mutex poisoned") = *snapshot;
            }
            CornersMsg::MonitorsChanged => {}
        }
        self.rebuild();
    }
}

impl Corners {
    fn rebuild(&mut self) {
        for window in self.windows.drain(..) {
            window.destroy();
        }
        let settings = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        let specs = corner_specs(&settings);
        if specs.is_empty() {
            return;
        }
        let Some(display) = gdk::Display::default() else {
            return;
        };
        for (idx, monitor) in shared::monitors(&display).into_iter().enumerate() {
            for spec in &specs {
                self.windows
                    .push(window::CornerWindow::new(idx, &monitor, spec).root);
            }
        }
    }
}
