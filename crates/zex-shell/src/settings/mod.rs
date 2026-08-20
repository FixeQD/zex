//! Settings window component

mod component;
mod preview;
pub mod tabs;
mod widgets;
mod window;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use relm4::prelude::*;
use window::SettingsWindow;
use zex_core::SettingsStore;
use zex_core::store::Subscription;
use zex_core::theme::matugen::Preview;

#[doc(hidden)]
pub use preview::preview_css;
pub use window::tab_label;

pub const SETTINGS_CSS_SCSS: &str = include_str!("../../assets/css/settings.scss");

#[derive(Debug)]
pub enum SettingsMsg {
    Open,
    TabSelected(String),
    SettingsChanged(Box<zex_core::Settings>),
    PreviewsReady(Vec<Preview>),
    GalleryReady(Vec<String>),
    Reload,
}

pub struct Settings {
    store: Rc<RefCell<SettingsStore>>,
    window: SettingsWindow,
    active: String,
    previews: Vec<Preview>,
    gallery: Vec<String>,
    last_preview_path: String,
    last_folder: String,
    subscription: Option<Subscription>,
    _provider: Option<gtk4::CssProvider>,
    _theme_provider: gtk4::CssProvider,
    sender: ComponentSender<Self>,
}

#[relm4::component(pub)]
impl SimpleComponent for Settings {
    type Init = (SettingsStore, Rc<crate::shared::ActionHandles>);
    type Input = SettingsMsg;
    type Output = ();

    view! {
        root = gtk4::Window {
            set_visible: false,
        }
    }

    fn init(
        (store, actions): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let window = SettingsWindow::new(sender.input_sender().clone());

        let store = Rc::new(RefCell::new(store));

        let mut model = Self {
            store: Rc::clone(&store),
            window,
            active: "quick".to_string(),
            previews: Vec::new(),
            gallery: Vec::new(),
            last_preview_path: String::new(),
            last_folder: String::new(),
            subscription: None,
            _provider: Some(crate::shared::install_css_provider(SETTINGS_CSS_SCSS)),
            _theme_provider: crate::shared::install_css_provider(""),
            sender: sender.clone(),
        };

        let subscription = {
            let inner = store.borrow();
            crate::shared::subscribe_settings(&inner, sender.input_sender().clone(), |snapshot| {
                SettingsMsg::SettingsChanged(Box::new(snapshot.clone()))
            })
        };
        model.subscription = Some(subscription);

        model.refresh_theme_provider();

        let open = {
            let sender = sender.input_sender().clone();
            move || {
                let _ = sender.send(SettingsMsg::Open);
            }
        };
        actions.set_settings(open);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::Open => self.open(),
            SettingsMsg::TabSelected(key) => self.set_tab(key),
            SettingsMsg::SettingsChanged(snapshot) => {
                self.on_settings_changed(&snapshot);
            }
            SettingsMsg::PreviewsReady(previews) => {
                self.previews = previews;
                self.refresh_theme_provider();
                self.rebuild_tab();
            }
            SettingsMsg::GalleryReady(gallery) => {
                self.gallery = gallery;
                self.rebuild_tab();
            }
            SettingsMsg::Reload => self.reload(),
        }
    }
}
