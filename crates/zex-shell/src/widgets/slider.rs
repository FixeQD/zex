//! Material 3 Slider widget

use iced::{Element, widget::{slider, Slider}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderConfig {
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub show_value: bool,
    pub continuous: bool,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            min: 0,
            max: 100,
            step: 1,
            show_value: true,
            continuous: false,
        }
    }
}

/// Create a Material 3 slider
pub fn material_slider<'a, Message>(
    config: SliderConfig,
    value: i32,
    on_change: impl Fn(i32) -> Message + 'a,
    on_release: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut slider = Slider::new(config.min..=config.max, value, on_change)
        .step(config.step);

    if config.continuous {
        // continuous is not available in iced 0.14 slider
    }

    if let Some(on_release) = on_release {
        slider = slider.on_release(on_release);
    }

    slider.into()
}