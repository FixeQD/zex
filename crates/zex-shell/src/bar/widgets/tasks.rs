use iced::widget::{Row, button, text};
use iced::Element;
use zex_launcher::apps::AppInfo;
use zex_services::compositor::WindowInfo;

use crate::app::Message;

pub fn is_same_app(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    a.contains(&b) || b.contains(&a)
}

pub fn view<'a>(
    apps: &'a [AppInfo],
    windows: &'a [WindowInfo],
    active: Option<&'a WindowInfo>,
    _vertical: bool,
    _side: &str,
    density: i8,
) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let size = if matches!(density, 0 | 1) { 20 } else { 16 };
    let mut row = Row::new().spacing(4);
    for app in apps {
        let wins: Vec<&WindowInfo> = windows
            .iter()
            .filter(|w| is_same_app(&app.id, &w.class))
            .collect();
        let is_active = active.is_some_and(|a| is_same_app(&app.id, &a.class));
        let label = if wins.is_empty() {
            text(&app.id).size(size - 6)
        } else {
            let n = wins.len();
            text(format!("{} {}", app.id, n)).size(size - 6)
        };
        let mut btn = button(label).padding([4, 6]);
        if is_active {
            btn = btn.style(button::primary);
        }
        if wins.is_empty() {
            let id = app.id.clone();
            btn = btn.on_press(Message::FocusWindow(id));
        } else if let Some(w) = wins.first() {
            let addr = w.address.clone();
            btn = btn.on_press(Message::FocusWindow(addr));
        }
        row = row.push(btn);
    }
    row.into()
}
