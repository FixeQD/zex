//! Material 3 Button widget

use iced::{
    Element,
    widget::{Button, Text, button},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonType {
    Filled,
    Elevated,
    Tonal,
    Outlined,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonShape {
    Rounded,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonConfig {
    pub button_type: ButtonType,
    pub size: ButtonSize,
    pub shape: ButtonShape,
    pub leading_icon: Option<&'static str>,
    pub trailing_icon: Option<&'static str>,
    pub disabled: bool,
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            button_type: ButtonType::Filled,
            size: ButtonSize::Md,
            shape: ButtonShape::Rounded,
            leading_icon: None,
            trailing_icon: None,
            disabled: false,
        }
    }
}

/// Create a Material 3 button
pub fn material_button<'a, Message>(
    config: ButtonConfig,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut btn = Button::new(Text::new(label)).on_press_maybe(on_press);

    // Apply styling based on config
    btn = match config.button_type {
        ButtonType::Filled => btn.style(button::primary),
        ButtonType::Elevated => btn.style(button::secondary),
        ButtonType::Tonal => btn.style(button::secondary),
        ButtonType::Outlined => btn.style(button::secondary),
        ButtonType::Text => btn.style(button::secondary),
    };

    // Apply size
    let (padding_x, padding_y, text_size) = match config.size {
        ButtonSize::Xs => (12, 4, 12),
        ButtonSize::Sm => (16, 8, 13),
        ButtonSize::Md => (24, 10, 14),
        ButtonSize::Lg => (32, 12, 15),
        ButtonSize::Xl => (40, 14, 16),
    };

    btn = btn.padding([padding_y, padding_x]);
    // Text size would be set via theme or widget style

    if config.disabled {
        btn = btn.style(button::secondary); // disabled style
    }

    btn.into()
}
