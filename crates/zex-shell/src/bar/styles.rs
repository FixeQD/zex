//! Bar style computation

use zex_core::settings::Bar;
use zex_core::settings::Bar2;

/// Screen edge a bar can sit on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub fn parse(side: &str) -> Side {
        match side {
            "top" => Side::Top,
            "bottom" => Side::Bottom,
            "left" => Side::Left,
            "right" => Side::Right,
            other => {
                tracing::warn!("unknown bar side \"{other}\", falling back to bottom");
                Side::Bottom
            }
        }
    }

    pub const fn as_css_class(self) -> &'static str {
        match self {
            Side::Top => "top",
            Side::Bottom => "bottom",
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    pub const fn is_vertical(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }

    /// Index into the `[top, left, right, bottom]` margin/anchor arrays
    pub const fn index(self) -> usize {
        match self {
            Side::Top => 0,
            Side::Left => 1,
            Side::Right => 2,
            Side::Bottom => 3,
        }
    }

    /// Opposite edge, used to zero the margin that would double the gap
    pub const fn opposite(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Shared settings surface for both bar instances
pub trait BarLike {
    fn side(&self) -> &str;
    fn vertical(&self) -> bool;
    fn density(&self) -> i8;
    fn floating(&self) -> bool;
    fn separation(&self) -> bool;
    fn centered(&self) -> bool;
    fn bar_background(&self) -> bool;
    fn module_backgrounds(&self) -> bool;
}

impl BarLike for Bar {
    fn side(&self) -> &str {
        &self.side
    }
    fn vertical(&self) -> bool {
        self.vertical
    }
    fn density(&self) -> i8 {
        self.density
    }
    fn floating(&self) -> bool {
        self.floating
    }
    fn separation(&self) -> bool {
        self.separation
    }
    fn centered(&self) -> bool {
        self.centered
    }
    fn bar_background(&self) -> bool {
        self.bar_background
    }
    fn module_backgrounds(&self) -> bool {
        self.module_backgrounds
    }
}

impl BarLike for Bar2 {
    fn side(&self) -> &str {
        &self.side
    }
    fn vertical(&self) -> bool {
        self.vertical
    }
    fn density(&self) -> i8 {
        self.density
    }
    fn floating(&self) -> bool {
        self.floating
    }
    fn separation(&self) -> bool {
        self.separation
    }
    fn centered(&self) -> bool {
        self.centered
    }
    fn bar_background(&self) -> bool {
        self.bar_background
    }
    fn module_backgrounds(&self) -> bool {
        self.module_backgrounds
    }
}

/// Schema density (`0` normal, negative compact, positive expanded)
pub const fn compact_rank(density: i8) -> u8 {
    let compact = -density;
    if compact <= 0 {
        0
    } else if compact >= 3 {
        3
    } else {
        compact as u8
    }
}

/// Bar thickness
pub const fn thickness_for(rank: u8) -> i32 {
    match rank {
        1 => 35,
        2 => 30,
        3 => 25,
        _ => 40,
    }
}

/// Full style state of a bar window
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarStyle {
    pub side: Side,
    pub css_classes: Vec<&'static str>,
    pub thickness: i32,
    pub margins: [i32; 4],
    pub anchors: [bool; 4],
}

/// Compute the style of one bar instance from its settings
pub fn compute(bar: &dyn BarLike) -> BarStyle {
    let side = Side::parse(bar.side());
    let rank = compact_rank(bar.density());

    let mut css_classes = Vec::with_capacity(8);
    if bar.floating() {
        css_classes.push("floating");
    } else {
        css_classes.push("hug");
        if bar.centered() {
            css_classes.push("round");
        } else {
            css_classes.push("extrapadding");
        }
    }
    if bar.separation() {
        css_classes.push("separated");
    } else {
        css_classes.push("full");
    }
    match rank {
        1 => css_classes.push("compact"),
        2 => css_classes.push("compact-plus"),
        3 => css_classes.push("ultracompact"),
        _ => {}
    }
    css_classes.push(if bar.vertical() {
        "vertical"
    } else {
        "horizontal"
    });
    if bar.module_backgrounds() {
        css_classes.push("module-backgrounds");
    }
    if bar.bar_background() {
        css_classes.push("bar-background");
    }
    css_classes.push(side.as_css_class());

    // Anchored to the full side, or centered
    let mut anchors = [false; 4];
    if bar.centered() {
        anchors[side.index()] = true;
    } else if bar.vertical() {
        anchors[Side::Top.index()] = true;
        anchors[Side::Bottom.index()] = true;
        anchors[side.index()] = true;
    } else {
        anchors[Side::Left.index()] = true;
        anchors[Side::Right.index()] = true;
        anchors[side.index()] = true;
    }

    // Floating bars are lifted off the screen edge by a fixed margin
    let mut margins = [0; 4];
    if bar.floating() {
        margins = [5; 4];
        margins[side.opposite().index()] = 0;
    }

    BarStyle {
        side,
        css_classes,
        thickness: thickness_for(rank),
        margins,
        anchors,
    }
}
