use iced::widget::{Row, Column, button, text};
use iced::Element;
use zex_services::mpris::MprisPlayer;

use crate::app::Message;

fn truncate(s: &str) -> String {
    const MAX: usize = 52;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(MAX - 1).collect();
        t.push('…');
        t
    }
}

pub fn view<'a>(
    players: &[MprisPlayer],
    vertical: bool,
    _centered: bool,
    dense: bool,
) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    if players.is_empty() {
        return Row::new().into();
    }
    let mut row = Row::new().spacing(4);
    for (i, p) in players.iter().enumerate() {
        let show_labels = !vertical && i == players.len() - 1;
        let mut col = Column::new();
        if show_labels {
            col = col.push(text(truncate(&p.info.title)).size(12));
            if !dense {
                let artist = p.info.artist.join(", ");
                if !artist.is_empty() {
                    col = col.push(text(truncate(&artist)).size(8));
                }
            }
        }
        let mut item = Row::new()
            .spacing(4)
            .push(text("♫").size(if dense { 12 } else { 16 }))
            .push(col);
        let btn = button(item).on_press(Message::MediaPlayPause(p.name.clone()));
        row = row.push(btn);
    }
    row.into()
}
