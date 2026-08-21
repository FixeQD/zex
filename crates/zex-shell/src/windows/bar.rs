use iced::widget::{Row, container, text};
use iced::Element;
use iced::Theme;

use crate::app::{Message, State};
use crate::bar::{
    layout::{Area, Layout, Module},
    styles,
    widgets::{media, window_info, workspaces},
};
use crate::bar::widgets::workspaces::Options as WsOpts;

pub fn view<'a>(monitor: usize, bar_id: u8, state: &'a State) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let cfg = &state.config.interface;
    let bar: &dyn styles::BarLike = if bar_id == 0 { &cfg.bar } else { &cfg.bar2 };
    let style = styles::compute(bar);
    let layout = Layout::new(&cfg.modules);

    let mut left: Vec<Element<'a, Message, Theme, iced_wgpu::Renderer>> = Vec::new();
    let mut center: Vec<Element<'a, Message, Theme, iced_wgpu::Renderer>> = Vec::new();
    let mut right: Vec<Element<'a, Message, Theme, iced_wgpu::Renderer>> = Vec::new();

    let ws_style =
        workspaces::Style::from_settings(&cfg.modules.options.workspaces_style);
    let ws_opts = WsOpts {
        style: ws_style,
        fixed: cfg.modules.options.fixed_workspaces_enabled,
        amount: cfg.modules.options.fixed_workspaces_amount as usize,
        vertical: bar.vertical(),
        display_offset: 0,
    };

    for p in layout.for_bar(bar_id) {
        if !p.visible {
            continue;
        }
        let w: Element<'a, Message, Theme, iced_wgpu::Renderer> = match p.module {
            Module::Launcher => text("≡").size(14).into(),
            Module::WindowInfo => {
                window_info::view(None, bar.vertical(), false, bar.density() < 0)
            }
            Module::Media => media::view(&[], bar.vertical(), false, bar.density() < 0),
            Module::Workspaces => workspaces::view(&[], &[], ws_opts),
            Module::Tasks => text("tasks").size(12).into(),
            Module::RecordingIndicator => text("●").size(12).into(),
            Module::SystemInfoTray => text("tray").size(12).into(),
            Module::Clock => {
                let t = chrono::Local::now().format("%H:%M").to_string();
                text(t).size(12).into()
            }
        };
        match p.area {
            Area::Left => left.push(w),
            Area::Center => center.push(w),
            Area::Right => right.push(w),
        }
    }

    let left_row = Row::with_children(left).spacing(6);
    let center_row = Row::with_children(center).spacing(6);
    let right_row = Row::with_children(right).spacing(6);

    let inner = Row::new()
        .push(left_row)
        .push(container(center_row).center_x(iced::Fill))
        .push(right_row)
        .spacing(8)
        .padding(if style.thickness == 40 { 6 } else { 4 })
        .align_y(iced::Alignment::Center);

    let bg = if bar.bar_background() {
        iced::Background::Color(state.theme.palette().background)
    } else {
        iced::Background::Color(iced::Color::TRANSPARENT)
    };

    container(inner)
        .style(move |_: &Theme| container::Style {
            background: Some(bg),
            ..Default::default()
        })
        .into()
}

pub fn title(monitor: usize, bar_id: u8) -> String {
    if bar_id == 0 {
        format!("zex-bar-{monitor}")
    } else {
        format!("zex-bar2-{monitor}")
    }
}
