//! OSD widget tree: the overlay window with an icon and non-interactive scale.

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use zex_services::audio::volume::VolumeState;

const VOLUME_ICON: &str = "audio-volume-high-symbolic";
const MUTED_ICON: &str = "audio-volume-muted-symbolic";
const BRIGHTNESS_ICON: &str = "display-brightness-symbolic";

pub struct OsdWidget {
    pub root: gtk4::Window,
    pub icon: gtk4::Image,
    pub scale: gtk4::Scale,
    pub box_: gtk4::Box,
}

impl OsdWidget {
    pub fn new() -> Self {
        let root = gtk4::Window::new();
        root.init_layer_shell();
        root.set_layer(Layer::Overlay);
        root.set_namespace(Some("zex-osd"));
        root.set_keyboard_mode(KeyboardMode::None);
        root.set_exclusive_zone(0);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            root.set_margin(edge, 20);
        }

        let icon = gtk4::Image::from_icon_name(VOLUME_ICON);
        icon.set_pixel_size(28);
        icon.set_css_classes(&["osd-icon"]);

        let adjustment = gtk4::Adjustment::new(0.0, 0.0, 1.0, 0.001, 0.1, 0.1);
        let scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&adjustment));
        scale.set_draw_value(false);
        scale.set_sensitive(false);
        scale.set_css_classes(&["osd-progress"]);

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        box_.set_halign(gtk4::Align::Center);
        box_.set_valign(gtk4::Align::Center);
        box_.set_css_classes(&["osd"]);
        box_.append(&icon);
        box_.append(&scale);

        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::Crossfade);
        revealer.set_transition_duration(250);
        revealer.set_halign(gtk4::Align::Center);
        revealer.set_valign(gtk4::Align::Center);
        revealer.set_child(Some(&box_));
        root.set_child(Some(&revealer));

        // Sync the crossfade with window visibility
        let revealer_sync = revealer.clone();
        root.connect_notify_local(Some("visible"), move |root, _| {
            revealer_sync.set_reveal_child(root.is_visible());
        });

        Self {
            root,
            icon,
            scale,
            box_,
        }
    }

    pub fn apply_orientation(&self, vertical: bool) {
        let orientation = if vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        };
        self.box_.set_orientation(orientation);
        self.scale.set_orientation(orientation);
        let (width, height) = if vertical { (20, 200) } else { (200, 20) };
        self.scale.set_size_request(width, height);
        if vertical {
            self.box_.add_css_class("vertical");
        } else {
            self.box_.remove_css_class("vertical");
        }
    }

    pub fn set_volume(&self, state: &VolumeState) {
        self.icon
            .set_icon_name(Some(if state.muted { MUTED_ICON } else { VOLUME_ICON }));
        self.scale
            .set_value(f64::from(state.volume.clamp(0.0, 1.0)));
    }

    pub fn set_backlight(&self, value: f32) {
        self.icon.set_icon_name(Some(BRIGHTNESS_ICON));
        self.scale.set_value(f64::from(value.clamp(0.0, 1.0)));
    }
}
