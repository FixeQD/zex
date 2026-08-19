//! M3 buttons: type, size and shape variants plus connected groups.

use gtk4::glib;
use gtk4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3Type {
    Elevated,
    Filled,
    Tonal,
    Outlined,
    Text,
}

impl M3Type {
    pub fn class(self) -> &'static str {
        match self {
            M3Type::Elevated => "elevated",
            M3Type::Filled => "filled",
            M3Type::Tonal => "tonal",
            M3Type::Outlined => "outlined",
            M3Type::Text => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3Size {
    Xs,
    S,
    M,
    L,
    Xl,
}

impl M3Size {
    pub fn class(self) -> &'static str {
        match self {
            M3Size::Xs => "xs",
            M3Size::S => "s",
            M3Size::M => "m",
            M3Size::L => "l",
            M3Size::Xl => "xl",
        }
    }

    pub fn gap(self) -> i32 {
        match self {
            M3Size::Xs | M3Size::S | M3Size::M => 8,
            M3Size::L => 12,
            M3Size::Xl => 16,
        }
    }

    pub fn icon_px(self) -> i32 {
        match self {
            M3Size::Xs | M3Size::S => 20,
            M3Size::M => 24,
            M3Size::L => 32,
            M3Size::Xl => 40,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3Shape {
    Round,
    Square,
}

impl M3Shape {
    pub fn class(self) -> &'static str {
        match self {
            M3Shape::Round => "round",
            M3Shape::Square => "square",
        }
    }
}

/// One M3 button: a classed `gtk4::Button` with an icon/image and a label
pub struct M3Button {
    pub button: gtk4::Button,
    pub icon: gtk4::Image,
    pub label: gtk4::Label,
    pub container: gtk4::Box,
}

impl M3Button {
    /// Build a button with the given icon name and/or label text
    pub fn new(
        icon_name: Option<&str>,
        text: Option<&str>,
        kind: M3Type,
        size: M3Size,
        shape: M3Shape,
    ) -> Self {
        let button = gtk4::Button::new();
        button.add_css_class("m3-button");
        button.add_css_class(kind.class());
        button.add_css_class(size.class());
        button.add_css_class(shape.class());

        let icon = gtk4::Image::new();
        icon.add_css_class("m3-button-icon");
        icon.set_pixel_size(size.icon_px());
        match icon_name {
            Some(name) => icon.set_icon_name(Some(name)),
            None => icon.set_visible(false),
        }

        let label = gtk4::Label::new(None);
        label.add_css_class("m3-button-label");
        match text {
            Some(text) => label.set_label(text),
            None => label.set_visible(false),
        }

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, size.gap());
        container.set_halign(gtk4::Align::Center);
        container.set_valign(gtk4::Align::Center);
        container.append(&icon);
        container.append(&label);
        button.set_child(Some(&container));

        let button = Self {
            button,
            icon,
            label,
            container,
        };
        button.sync_icon_only();
        button
    }

    pub fn set_icon(&self, icon_name: Option<&str>) {
        match icon_name {
            Some(name) => {
                self.icon.set_icon_name(Some(name));
                self.icon.set_visible(true);
            }
            None => self.icon.set_visible(false),
        }
        self.sync_icon_only();
    }

    pub fn set_label(&self, text: Option<&str>) {
        match text {
            Some(text) => {
                self.label.set_label(text);
                self.label.set_visible(true);
            }
            None => self.label.set_visible(false),
        }
        self.sync_icon_only();
    }

    /// Stack the icon vertically above the label (navigation rail items)
    pub fn set_vertical(&self, vertical: bool) {
        let orientation = if vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        };
        self.container.set_orientation(orientation);
    }

    /// Toggle the segment state used inside connected groups
    pub fn set_active(&self, active: bool) {
        if active {
            self.button.add_css_class("active");
        } else {
            self.button.remove_css_class("active");
        }
    }

    pub fn connect_clicked<F: Fn(&gtk4::Button) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.button.connect_clicked(f)
    }

    fn sync_icon_only(&self) {
        let icon_only = self.icon.is_visible() && !self.label.is_visible();
        if icon_only {
            self.button.add_css_class("icon-only");
        } else {
            self.button.remove_css_class("icon-only");
        }
    }
}

/// A group of connected M3 buttons (e.g. a segmented control)
pub struct ConnectedButtonGroup {
    pub container: gtk4::Box,
}

impl ConnectedButtonGroup {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
        container.set_vexpand(false);
        container.add_css_class("connected-button-group");
        Self { container }
    }

    pub fn add(&self, button: &M3Button) {
        self.container.append(&button.button);
    }
}

impl Default for ConnectedButtonGroup {
    fn default() -> Self {
        Self::new()
    }
}
