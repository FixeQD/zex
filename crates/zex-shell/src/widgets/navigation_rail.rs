//! Material 3 Navigation Rail widget

use iced::{Element, widget::{Column, Container, Text, button, Button, Space, row, Row}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RailItem {
    Home,
    Settings,
    Notifications,
    Apps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailConfig {
    pub selected: RailItem,
    pub extended: bool,
    pub show_labels: bool,
}

impl Default for RailConfig {
    fn default() -> Self {
        Self {
            selected: RailItem::Home,
            extended: false,
            show_labels: false,
        }
    }
}

fn rail_item_label(item: RailItem) -> &'static str {
    match item {
        RailItem::Home => "Home",
        RailItem::Settings => "Settings",
        RailItem::Notifications => "Notifications",
        RailItem::Apps => "Apps",
    }
}

fn rail_item_icon(item: RailItem) -> &'static str {
    match item {
        RailItem::Home => "home",
        RailItem::Settings => "settings",
        RailItem::Notifications => "notifications",
        RailItem::Apps => "apps",
    }
}

/// Create a Material 3 Navigation Rail
pub fn material_navigation_rail<'a, Message>(
    config: RailConfig,
    on_select: impl Fn(RailItem) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let items = [
        RailItem::Home,
        RailItem::Settings,
        RailItem::Notifications,
        RailItem::Apps,
    ];

    let mut column = Column::new().spacing(8).padding(12);

    for item in items {
        let is_selected = config.selected == item;
        
        let content: Element<'_, Message> = if config.extended && config.show_labels {
            Row::new()
                .spacing(12)
                .push(Text::new(rail_item_icon(item)).size(24))
                .push(Space::new())
                .push(Text::new(rail_item_label(item)).size(14))
                .into()
        } else {
            Text::new(rail_item_icon(item)).size(24).into()
        };
        
        let mut btn = Button::new(content)
            .on_press(on_select(item))
            .padding(12)
            .width(if config.extended { 200 } else { 48 });

        if is_selected {
            btn = btn.style(button::primary);
        } else {
            btn = btn.style(button::secondary);
        }

        column = column.push(btn);
    }

    Container::new(column)
        .style(|theme: &iced_core::Theme| iced_widget::container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced_core::Border {
                radius: 28.0.into(),
                width: 0.0,
                color: iced_core::Color::TRANSPARENT,
            },
            ..Default::default()
        })
        .into()
}