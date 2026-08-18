//! Per-bar layer-shell window component

use std::rc::Rc;

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use relm4::prelude::*;

use super::layout::{Area, Layout, Module};
use super::styles::{self, BarStyle};
use crate::widgets::{SharedSettings, Widgets};

/// Constructor payload: which monitor/bar this window belongs to plus the shared settings snapshot and the monitor's widget registry
pub struct BarWindowInit {
    pub monitor_idx: usize,
    pub monitor: gdk::Monitor,
    pub bar_id: u8,
    pub settings: SharedSettings,
    pub widgets: Rc<Widgets>,
}

/// Nothing to act on yet besides full refreshes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarMsg {
    Refresh,
}

pub struct BarWindow {
    /// The layer-shell surface; cloned in `init` so messages can restyle it
    root: gtk4::Window,
    bar_id: u8,
    settings: SharedSettings,
    widgets: Rc<Widgets>,
    areas: [gtk4::Box; 3],
    style: BarStyle,
    visible: bool,
}

/// Layer-shell namespace contract with the compositor
fn namespace(monitor_idx: usize, bar_id: u8) -> String {
    match bar_id {
        0 => format!("zex-bar-{monitor_idx}"),
        _ => format!("zex-bar2-{monitor_idx}"),
    }
}

fn area_box(area: Area) -> gtk4::Box {
    let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    box_.set_css_classes(&[area.as_css_class()]);
    box_
}

impl BarWindow {
    /// Re-read the snapshot and rebuild model state
    /// Runs on the GTK thread
    fn refresh_model(&mut self) {
        let snapshot = self
            .settings
            .lock()
            .expect("settings mutex poisoned")
            .clone();
        let bar: &dyn styles::BarLike = if self.bar_id == 0 {
            &snapshot.interface.bar
        } else {
            &snapshot.interface.bar2
        };
        self.style = styles::compute(bar);

        let layout = Layout::new(&snapshot.interface.modules);
        self.visible = match self.bar_id {
            0 => layout.bar_in_use(0),
            _ => snapshot.interface.bar2.enabled && layout.bar_in_use(1),
        };

        for area in &self.areas {
            while let Some(child) = area.first_child() {
                area.remove(&child);
            }
            area.remove_css_class("empty");
        }

        for module in Module::ALL {
            let Some(placement) = layout.for_bar(self.bar_id).find(|p| p.module == module) else {
                continue;
            };
            let Some(widget) = self.widgets.get(module) else {
                continue;
            };
            self.areas[placement.area.index()].append(&widget);
            widget.set_visible(placement.visible);
        }

        for area in &self.areas {
            if area.first_child().is_none() {
                area.add_css_class("empty");
            }
        }
    }

    /// Apply the computed style to the window: classes, thickness, margins, anchors and visibility
    /// Idempotent, safe to re-apply on every refresh
    fn apply_window_state(root: &gtk4::Window, style: &BarStyle, visible: bool) {
        root.set_css_classes(&[]);
        for class in &style.css_classes {
            root.add_css_class(class);
        }
        if style.side.is_vertical() {
            root.set_width_request(style.thickness);
            root.set_height_request(-1);
        } else {
            root.set_width_request(-1);
            root.set_height_request(style.thickness);
        }
        for (edge, margin) in [
            (Edge::Top, style.margins[0]),
            (Edge::Left, style.margins[1]),
            (Edge::Right, style.margins[2]),
            (Edge::Bottom, style.margins[3]),
        ] {
            root.set_margin(edge, margin);
        }
        for (edge, anchor) in [
            (Edge::Top, style.anchors[0]),
            (Edge::Left, style.anchors[1]),
            (Edge::Right, style.anchors[2]),
            (Edge::Bottom, style.anchors[3]),
        ] {
            root.set_anchor(edge, anchor);
        }
        root.set_visible(visible);
    }
}

trait AreaIndex {
    fn index(self) -> usize;
}
impl AreaIndex for Area {
    fn index(self) -> usize {
        match self {
            Area::Left => 0,
            Area::Center => 1,
            Area::Right => 2,
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for BarWindow {
    type Init = BarWindowInit;
    type Input = BarMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Layer-shell setup must happen before the window is realized
        let widgets = view_output!();
        widgets.root.init_layer_shell();
        widgets.root.set_layer(Layer::Top);
        widgets.root.set_monitor(Some(&init.monitor));
        widgets
            .root
            .set_namespace(Some(&namespace(init.monitor_idx, init.bar_id)));
        widgets.root.auto_exclusive_zone_enable();

        let center = gtk4::CenterBox::new();
        center.set_css_classes(&["bar-widgets"]);
        let areas = Area::ALL.map(area_box);
        center.set_start_widget(Some(&areas[0]));
        center.set_center_widget(Some(&areas[1]));
        center.set_end_widget(Some(&areas[2]));
        widgets.root.set_child(Some(&center));

        let mut model = Self {
            root: widgets.root.clone(),
            bar_id: init.bar_id,
            settings: init.settings,
            widgets: init.widgets,
            areas,
            style: styles::compute(&zex_core::Settings::default().interface.bar),
            visible: false,
        };
        model.refresh_model();
        Self::apply_window_state(&widgets.root, &model.style, model.visible);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            BarMsg::Refresh => {
                self.refresh_model();
                if let Some(clock) = self.widgets.clock() {
                    clock.update_layout();
                }
                Self::apply_window_state(&self.root, &self.style, self.visible);
            }
        }
    }
}
