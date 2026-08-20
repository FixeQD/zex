//! Appearance tab quick-select category: folder chooser and wallpaper gallery.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;

use crate::settings::tabs::TabContext;
use crate::settings::widgets::category;

pub fn quick_select_category(ctx: &TabContext) -> gtk4::Box {
    let container = category("Quick Select");

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
    column.set_vexpand(true);
    column.set_valign(gtk4::Align::Fill);

    let folder_button = gtk4::Button::with_label("Select a Folder");
    folder_button.add_css_class("folder-chooser-button");
    column.append(&folder_button);

    let store = Rc::clone(&ctx.store);
    let window = ctx.window.clone();
    folder_button.connect_clicked(move |_| {
        let dialog = gtk4::FileDialog::builder()
            .title("Select Wallpapers Folder")
            .build();
        let store = Rc::clone(&store);
        dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
            let Ok(file) = result else {
                return;
            };
            let Some(path) = file.path() else {
                return;
            };
            let Ok(path) = path.into_os_string().into_string() else {
                return;
            };
            if let Err(err) = store.borrow_mut().update(|s| {
                s.appearance.wallcolors.quickselect_path = path;
            }) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
        });
    });

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_height_request(300);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    let gallery = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    gallery.add_css_class("wallpaper-gallery-container");
    build_gallery(ctx, &scroll);
    gallery.append(&scroll);
    column.append(&gallery);

    container.append(&column);
    container
}

fn build_gallery(ctx: &TabContext, scroll: &gtk4::ScrolledWindow) {
    if ctx.gallery.is_empty() {
        let label = gtk4::Label::new(Some("No wallpapers found in the directory."));
        label.add_css_class("message");
        label.set_halign(gtk4::Align::Center);
        label.set_margin_top(24);
        scroll.set_child(Some(&label));
        return;
    }

    let current = ctx.snapshot().appearance.wallcolors.wallpaper_path.clone();

    let grid = gtk4::Grid::new();
    grid.set_column_spacing(5);
    grid.set_row_spacing(5);
    grid.set_halign(gtk4::Align::Fill);
    grid.set_hexpand(true);

    let store = Rc::clone(&ctx.store);
    let registry: Rc<RefCell<Vec<(String, gtk4::Button)>>> = Rc::new(RefCell::new(Vec::new()));

    for (index, path) in ctx.gallery.iter().enumerate() {
        let selected = *path == current;
        let button = gtk4::Button::new();
        button.add_css_class("wallpaper-thumbnail");
        button.set_hexpand(true);
        button.set_halign(gtk4::Align::Fill);
        if selected {
            button.add_css_class("selected");
        }

        let picture = gtk4::Picture::new();
        picture.add_css_class("wallpaper-thumbnail-image");
        picture.set_content_fit(gtk4::ContentFit::Cover);
        picture.set_size_request(196, 100);
        picture.set_filename(Some(path));
        if selected {
            picture.add_css_class("selected");
        }
        button.set_child(Some(&picture));

        registry.borrow_mut().push((path.clone(), button.clone()));
        grid.attach(&button, (index % 4) as i32, (index / 4) as i32, 1, 1);
    }

    for (path, button) in registry.borrow().iter() {
        let path = path.clone();
        let registry = Rc::clone(&registry);
        let store = Rc::clone(&store);
        button.connect_clicked(move |_| {
            if let Err(err) = store.borrow_mut().update(|s| {
                s.appearance.wallcolors.wallpaper_path.clone_from(&path);
            }) {
                tracing::warn!("settings persistence failed: {err:#}");
            }
            sync_gallery_selection(&registry, &store);
        });
    }

    sync_gallery_selection(&registry, &store);
    scroll.set_child(Some(&grid));
}

fn sync_gallery_selection(
    registry: &Rc<RefCell<Vec<(String, gtk4::Button)>>>,
    store: &Rc<RefCell<zex_core::SettingsStore>>,
) {
    let binding = store.borrow();
    let current = &binding.get().appearance.wallcolors.wallpaper_path;
    for (path, button) in registry.borrow().iter() {
        let selected = path == current;
        if selected {
            button.add_css_class("selected");
            if let Some(child) = button.child() {
                child.add_css_class("selected");
            }
        } else {
            button.remove_css_class("selected");
            if let Some(child) = button.child() {
                child.remove_css_class("selected");
            }
        }
    }
}
