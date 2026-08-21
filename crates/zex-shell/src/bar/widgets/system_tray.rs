use iced::widget::{Row, button, text};
use iced::Element;
use zex_services::tray::TrayItem;

use crate::app::Message;

pub fn wifi_icon(enabled: bool, connected: bool, strength: Option<u8>) -> Option<&'static str> {
    if !enabled {
        return None;
    }
    if !connected {
        return Some("signal_wifi_off");
    }
    Some(match strength {
        None => "signal_wifi_off",
        Some(75..) => "signal_wifi_4_bar",
        Some(50..) => "network_wifi_3_bar",
        Some(25..) => "network_wifi_2_bar",
        Some(1..) => "network_wifi_1_bar",
        Some(_) => "signal_wifi_0_bar",
    })
}

pub fn audio_icon(volume: f32, muted: bool) -> &'static str {
    if muted {
        "volume_off"
    } else if volume < 0.33 {
        "volume_mute"
    } else if volume < 0.67 {
        "volume_down"
    } else {
        "volume_up"
    }
}

pub fn view<'a>(
    items: &'a [TrayItem],
    volume: f32,
    muted: bool,
) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let mut row = Row::new().spacing(8);
    for it in items {
        row = row.push(button(text(&it.service).size(10)).on_press(Message::TrayActivate(it.service.clone())));
    }
    row = row.push(text(audio_icon(volume, muted)).size(12));
    row.into()
}
