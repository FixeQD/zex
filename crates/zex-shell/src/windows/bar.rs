//! bar window view (stub)
use iced::widget::text;
use crate::app::{Message, State};

pub fn bar_view<'a>(_monitor: usize, _bar_id: u8, _state: &State, _theme: &iced::Theme) -> iced::Element<'a, Message> {
    text("bar - TODO").into()
}
