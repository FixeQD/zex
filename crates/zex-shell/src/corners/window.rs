//! Corner window factory: one click-through layer-shell surface per spec.

use gtk4::cairo;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, LayerShell};

use super::spec::{CornerKind, CornerSpec};

/// `$surface` from `assets/css/bar.scss` (`#131316`)
const BAR_SURFACE_RGB: (f64, f64, f64) = (19.0 / 255.0, 19.0 / 255.0, 22.0 / 255.0);
const SCREEN_MASK_RGB: (f64, f64, f64) = (0.0, 0.0, 0.0);

pub struct CornerWindow {
    pub root: gtk4::Window,
}

impl CornerWindow {
    pub fn new(monitor_idx: usize, monitor: &gdk::Monitor, spec: &CornerSpec) -> Self {
        let rgb = match spec.kind {
            CornerKind::Bar => BAR_SURFACE_RGB,
            CornerKind::Screen => SCREEN_MASK_RGB,
        };
        let size = spec.size as i32;

        let root = gtk4::Window::new();
        root.init_layer_shell();
        root.set_layer(spec.layer);
        root.set_monitor(Some(monitor));
        root.set_namespace(Some(&namespace(monitor_idx, spec)));
        root.set_size_request(size, size);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            root.set_anchor(edge, edge == spec.edges.0 || edge == spec.edges.1);
        }
        root.set_keyboard_mode(KeyboardMode::None);
        root.set_exclusive_zone(0);

        let drawing = gtk4::DrawingArea::new();
        drawing.set_size_request(size, size);
        drawing.set_css_classes(&["corner"]);
        drawing.set_draw_func(disc_draw(rgb, spec.edges));
        root.set_child(Some(&drawing));

        // Empty input region keeps the warp click-through
        root.connect_realize(|root| {
            let Some(surface) = root.surface() else {
                return;
            };
            surface.set_input_region(Some(&cairo::Region::create()));
        });

        root.present();

        Self { root }
    }
}

fn disc_draw(
    rgb: (f64, f64, f64),
    edges: (Edge, Edge),
) -> impl FnMut(&gtk4::DrawingArea, &cairo::Context, i32, i32) + 'static {
    move |_drawing, cr, width, height| {
        let (cx, cy, start, end) = corner_arc(edges, f64::from(width), f64::from(height));
        cr.set_source_rgba(rgb.0, rgb.1, rgb.2, 1.0);
        cr.arc(cx, cy, f64::from(width).max(f64::from(height)), start, end);
        let _ = cr.fill();
    }
}

fn corner_arc(edges: (Edge, Edge), width: f64, height: f64) -> (f64, f64, f64, f64) {
    let left = edges.0 == Edge::Left || edges.1 == Edge::Left;
    let top = edges.0 == Edge::Top || edges.1 == Edge::Top;
    let (cx, cy) = (
        if left { 0.0 } else { width },
        if top { 0.0 } else { height },
    );
    if left && top {
        (
            cx,
            cy,
            std::f64::consts::PI,
            3.0 * std::f64::consts::PI / 2.0,
        )
    } else if left {
        (cx, cy, std::f64::consts::PI / 2.0, std::f64::consts::PI)
    } else if top {
        (
            cx,
            cy,
            3.0 * std::f64::consts::PI / 2.0,
            2.0 * std::f64::consts::PI,
        )
    } else {
        (cx, cy, 0.0, std::f64::consts::PI / 2.0)
    }
}

fn namespace(monitor_idx: usize, spec: &CornerSpec) -> String {
    let e1 = edge_name(spec.edges.0);
    let e2 = edge_name(spec.edges.1);
    match spec.kind {
        CornerKind::Bar => format!("zex-corner-{monitor_idx}-{e1}-{e2}"),
        CornerKind::Screen => format!("zex-screen-corner-{monitor_idx}-{e1}-{e2}"),
    }
}

fn edge_name(edge: Edge) -> &'static str {
    match edge {
        Edge::Top => "top",
        Edge::Bottom => "bottom",
        Edge::Left => "left",
        Edge::Right => "right",
        other => {
            tracing::warn!("unknown corner edge {other:?}");
            "x"
        }
    }
}
