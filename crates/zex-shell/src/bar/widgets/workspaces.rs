use iced::widget::{Row, button, text};
use iced::Element;
use zex_services::compositor::{WindowInfo, WorkspaceInfo};

use crate::app::Message;

pub fn page_window(active: i32, amount: usize) -> (i32, i32) {
    let amount = amount.max(1) as i32;
    let base = (active - 1).div_euclid(amount) * amount + 1;
    (base, base + amount - 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Numbers,
    Dots,
    Windows,
}

impl Style {
    pub fn from_settings(s: &str) -> Self {
        match s {
            "dots" => Self::Dots,
            "windows" => Self::Windows,
            _ => Self::Numbers,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub style: Style,
    pub fixed: bool,
    pub amount: usize,
    pub vertical: bool,
    pub display_offset: i32,
}

pub fn view<'a>(
    workspaces: &[WorkspaceInfo],
    windows: &[WindowInfo],
    opts: Options,
) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let mut real: Vec<i32> = workspaces.iter().map(|w| w.id).collect();
    real.sort_unstable();
    real.dedup();
    let active = workspaces.iter().find(|w| w.active).map(|w| w.id).unwrap_or(-1);
    let (from, to) = if opts.fixed {
        page_window(active, opts.amount)
    } else if real.is_empty() {
        (1, 1)
    } else {
        (*real.first().unwrap(), *real.last().unwrap())
    };
    let mut row = Row::new().spacing(4);
    for id in from..=to {
        let is_active = id == active;
        let raw = id - opts.display_offset;
        let label = match opts.style {
            Style::Numbers => (id).to_string(),
            Style::Dots => "•".to_string(),
            Style::Windows => {
                let names: Vec<String> = windows
                    .iter()
                    .filter(|w| w.workspace == id)
                    .map(|w| w.class.clone())
                    .collect();
                if names.is_empty() {
                    "○".to_string()
                } else {
                    names.join(" ")
                }
            }
        };
        let mut btn = button(text(label).size(if opts.style == Style::Dots { 10 } else { 12 }))
            .on_press(Message::SwitchWorkspace(raw))
            .padding([4, 8]);
        if is_active {
            btn = btn.style(button::primary);
        }
        row = row.push(btn);
    }
    row.into()
}
