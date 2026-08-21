use iced::widget::{container, image};
use crate::app::{Message, State};

pub fn view<'a>(state: &'a State) -> iced::Element<'a, Message, iced::Theme, iced_wgpu::Renderer> {
    let path = state.config.appearance.wallcolors.wallpaper_path.clone();
    if path.is_empty() {
        return container(iced::widget::Space::new())
            .style(|t: &iced::Theme| container::Style {
                background: Some(t.palette().background.into()),
                ..Default::default()
            })
            .into();
    }
    let data = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return container(iced::widget::Space::new()).into(),
    };
    let img = match ::image::load_from_memory(&data) {
        Ok(i) => i,
        Err(_) => return container(iced::widget::Space::new()).into(),
    };
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let handle = image::Handle::from_rgba(w, h, rgba.into_raw());
    image(handle).into()
}
