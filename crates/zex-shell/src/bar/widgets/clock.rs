use iced::widget::text;
use iced::Element;
use zex_core::Settings;

use crate::app::Message;

pub fn view<'a>(settings: &Settings) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let now = chrono::Local::now();
    let fmt = if settings.interface.modules.options.military_time {
        "%H:%M"
    } else {
        "%I:%M %p"
    };
    let mut s = now.format(fmt).to_string();
    if settings.interface.modules.options.show_date {
        let d = if settings.interface.modules.options.day_month_swapped {
            now.format(" %m-%d")
        } else {
            now.format(" %d-%m")
        }
        .to_string();
        s.push_str(&d);
    }
    text(s).size(12).into()
}
