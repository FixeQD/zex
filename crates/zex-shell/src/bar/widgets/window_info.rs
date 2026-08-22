use iced::widget::{Row, Column, text};
use iced::Element;
use zex_services::compositor::WindowInfo;

use crate::app::Message;

const MAX_CHARS: usize = 52;

fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_CHARS {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(MAX_CHARS - 1).collect();
        t.push('…');
        t
    }
}

pub fn view<'a>(
    active: Option<&WindowInfo>,
    vertical: bool,
    _centered: bool,
    dense: bool,
) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let (title, app_id) = match active {
        Some(w) => (truncate(&w.title), w.class.clone()),
        None => ("Empty Workspace".into(), "Desktop".into()),
    };
    if vertical {
        return text(truncate(&app_id)).size(10).into();
    }
    let mut col = Column::new().spacing(0);
    col = col.push(text(title).size(12));
    if !dense {
        col = col.push(text(app_id).size(8));
    }
    Row::new().spacing(8).push(col).into()
}
