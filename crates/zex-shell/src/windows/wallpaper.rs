//! wallpaper window view (stub)
use iced::widget::text;
use crate::app::{Message, State};

pub fn wallpaper_view<'a>(_state: &State, _theme: &iced::Theme) -> iced::Element<'a, Message> {
    text("wallpaper - TODO").into()
}
