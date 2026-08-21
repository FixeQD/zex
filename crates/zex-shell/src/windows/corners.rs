//! corners window view (stub)
use iced::widget::text;
use crate::app::{Message, State};

pub fn corners_view<'a>(_state: &State, _theme: &iced::Theme) -> iced::Element<'a, Message> {
    text("corners - TODO").into()
}
