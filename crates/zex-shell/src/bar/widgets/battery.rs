use iced::widget::{Row, text};
use iced::Element;
use zex_services::upower::Battery;

use crate::app::Message;

pub fn status_icon(charging: bool, percent: u8) -> &'static str {
    if charging {
        "bolt"
    } else {
        match percent {
            100 => "battery_android_full",
            96..=99 => "battery_android_6",
            81..=95 => "battery_android_5",
            61..=80 => "battery_android_4",
            41..=60 => "battery_android_3",
            26..=40 => "battery_android_2",
            11..=25 => "battery_android_1",
            0..=10 => "battery_android_0",
            _ => "battery_android_question",
        }
    }
}

pub fn view<'a>(batteries: &[Battery]) -> Element<'a, Message> {
    let Some(b) = batteries.iter().find(|b| b.is_present) else {
        return Row::new().into();
    };
    let pct = b.percent_u8();
    let charging = b.charging();
    Row::new()
        .spacing(4)
        .push(text(status_icon(charging, pct)).size(12))
        .push(text(format!("{pct}%")).size(12))
        .into()
}
