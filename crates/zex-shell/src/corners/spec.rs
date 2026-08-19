//! Corner layout: which corners exist on a monitor and their size.

use std::collections::HashSet;

use gtk4_layer_shell::{Edge, Layer};

use crate::bar::styles::{BarLike, Side, compact_rank};

/// Corner extent when the bar density is untouched
pub const DEFAULT_WARP: u32 = 25;

const SCREEN_CORNERS: [(Edge, Edge); 4] = [
    (Edge::Top, Edge::Left),
    (Edge::Top, Edge::Right),
    (Edge::Bottom, Edge::Left),
    (Edge::Bottom, Edge::Right),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CornerKind {
    Bar,
    Screen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerSpec {
    pub kind: CornerKind,
    pub edges: (Edge, Edge),
    pub size: u32,
    pub layer: Layer,
}

/// Warp extent matching the bar's density: 25/22.5/20/17.5 px
pub fn warp_size(rank: u8) -> u32 {
    let size: f32 = match rank {
        1 => 22.5,
        2 => 20.0,
        3 => 17.5,
        _ => 25.0,
    };
    (size.round() as u32).max(1)
}

fn side_edge(side: Side) -> Edge {
    match side {
        Side::Top => Edge::Top,
        Side::Bottom => Edge::Bottom,
        Side::Left => Edge::Left,
        Side::Right => Edge::Right,
    }
}

fn hugging_corners(bar: &dyn BarLike) -> [(Edge, Edge); 2] {
    let side = side_edge(Side::parse(bar.side()));
    if bar.vertical() {
        [(Edge::Top, side), (Edge::Bottom, side)]
    } else {
        [(side, Edge::Left), (side, Edge::Right)]
    }
}

/// Deduplicated corner set for one monitor from a settings snapshot
pub fn corner_specs(settings: &zex_core::Settings) -> Vec<CornerSpec> {
    let interface = &settings.interface;
    let bar = &interface.bar;
    let bar2 = &interface.bar2;
    let misc = &interface.misc;
    let mut specs = Vec::new();

    if misc.screen_corners != "disabled" {
        let layer = if misc.screen_corners == "always" {
            Layer::Overlay
        } else {
            Layer::Top
        };
        // Corners already snug against a floating bar mirror its density
        let bar_side = side_edge(Side::parse(&bar.side));
        let optimal = warp_size(compact_rank(bar.density()));
        for edges in SCREEN_CORNERS {
            let near_bar =
                bar.floating && !bar.centered && (edges.0 == bar_side || edges.1 == bar_side);
            specs.push(CornerSpec {
                kind: CornerKind::Screen,
                edges,
                size: if near_bar { optimal } else { DEFAULT_WARP },
                layer,
            });
        }
    }

    if misc.shell_corners {
        // Same edge pair from both bars creates one window
        let mut seen = HashSet::new();
        let bar_likes: [&dyn BarLike; 2] = [bar, bar2];
        for (idx, bar_like) in bar_likes.into_iter().enumerate() {
            if (idx == 1 && !bar2.enabled) || bar_like.floating() || bar_like.centered() {
                continue;
            }
            for edges in hugging_corners(bar_like) {
                if seen.insert(edges) {
                    specs.push(CornerSpec {
                        kind: CornerKind::Bar,
                        edges,
                        size: DEFAULT_WARP,
                        layer: Layer::Top,
                    });
                }
            }
        }
    }

    specs
}
