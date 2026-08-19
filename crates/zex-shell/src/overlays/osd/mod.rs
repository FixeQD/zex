//! OSD: transient feedback for volume and brightness changes.

mod events;
mod widget;

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};
use relm4::prelude::*;
use zex_core::SettingsStore;
use zex_core::settings::Anchor;
use zex_core::store::Subscription;
use zex_services::audio::volume::VolumeState;

use crate::shared;

use self::widget::OsdWidget;

/// How long the OSD stays visible after the last trigger
pub const AUTO_HIDE_SECONDS: u64 = 2;

/// Shared OSD suppression flag: the quick-center sliders raise it while they drag
static SUPPRESS_OSD: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Flag shared by every OSD consumer; quick-center raises it around slider drags
pub fn suppress_osd_flag() -> Arc<AtomicBool> {
    SUPPRESS_OSD
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

#[derive(Debug)]
pub enum OsdMsg {
    SettingsChanged(Box<zex_core::Settings>),
    Volume(VolumeState),
    Backlight(f32),
}

pub struct Osd {
    widget: OsdWidget,
    /// Raised while quick-center sliders drag so their feedback does not pop the OSD
    suppress: Arc<AtomicBool>,
    /// Pending auto-hide source, restarted on every trigger
    hide_timer: RefCell<Option<glib::SourceId>>,
    subscription: Option<Subscription>,
}

impl Osd {
    fn suppressed(&self) -> bool {
        self.suppress.load(Ordering::Relaxed)
    }

    fn show_osd(&self) {
        if self.suppressed() {
            return;
        }
        if let Some(source) = self.hide_timer.borrow_mut().take() {
            source.remove();
        }
        self.widget.root.set_visible(true);
        let root = self.widget.root.clone();
        *self.hide_timer.borrow_mut() = Some(glib::timeout_add_local(
            std::time::Duration::from_secs(AUTO_HIDE_SECONDS),
            move || {
                root.set_visible(false);
                glib::ControlFlow::Break
            },
        ));
    }

    fn apply_layout(&self, settings: &zex_core::Settings) {
        let anchors = &settings.services.osd.anchor;
        for (edge, anchor) in [
            (Edge::Top, Anchor::Top),
            (Edge::Bottom, Anchor::Bottom),
            (Edge::Left, Anchor::Left),
            (Edge::Right, Anchor::Right),
        ] {
            self.widget.root.set_anchor(edge, anchors.contains(&anchor));
        }
        // Single-edge anchors force the layout along that edge
        let vertical = match anchors.as_slice() {
            [Anchor::Top] | [Anchor::Bottom] => false,
            [Anchor::Left] | [Anchor::Right] => true,
            _ => settings.services.osd.vertical,
        };
        self.widget.apply_orientation(vertical);
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Osd {
    type Init = SettingsStore;
    type Input = OsdMsg;
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
            widget: OsdWidget::new(),
            suppress: suppress_osd_flag(),
            hide_timer: RefCell::new(None),
            subscription: None,
        };
        model.subscription = Some(shared::subscribe_settings(
            &store,
            sender.input_sender().clone(),
            |s| OsdMsg::SettingsChanged(Box::new(s.clone())),
        ));
        model.apply_layout(store.get());

        events::spawn_volume_events(sender.clone());
        events::spawn_backlight_events(sender.clone());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            OsdMsg::SettingsChanged(snapshot) => {
                self.apply_layout(&snapshot);
            }
            OsdMsg::Volume(state) => {
                if self.suppressed() {
                    return;
                }
                self.widget.set_volume(&state);
                self.show_osd();
            }
            OsdMsg::Backlight(value) => {
                if self.suppressed() {
                    return;
                }
                self.widget.set_backlight(value);
                self.show_osd();
            }
        }
    }
}
