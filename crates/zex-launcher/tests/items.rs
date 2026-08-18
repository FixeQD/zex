use std::path::PathBuf;
use zex_launcher::apps::AppInfo;
use zex_launcher::items::{Identify, Item, Launchable, Menu};

fn app(title: &str) -> AppInfo {
    AppInfo {
        id: title.to_lowercase(),
        title: title.into(),
        command: "true".into(),
        icon_name: None,
        icon_file: None,
        summary: None,
        tags: vec![],
        wants_terminal: false,
        source: PathBuf::from("/tmp/app.desktop"),
    }
}

#[test]
fn titles_cover_every_kind() {
    assert_eq!(Item::App(app("Firefox")).title(), "Firefox");
    assert_eq!(
        Item::Action { owner: "firefox".into(), label: "New Window".into(), command: "firefox".into() }.title(),
        "New Window"
    );
    assert_eq!(Item::Command("ls -la".into()).title(), "ls -la");
    assert_eq!(Item::File(PathBuf::from("/tmp/note.md")).title(), "/tmp/note.md");
    assert_eq!(
        Item::Window { title: "Terminal".into(), app: "ghostty".into() }.title(),
        "Terminal"
    );
    assert_eq!(Item::Web { provider: "gh".into(), query: "zex".into() }.title(), "gh: zex");
    assert_eq!(Item::Theme("adw-gtk3".into()).title(), "Theme: adw-gtk3");
    assert_eq!(Item::Ai("summarize".into()).title(), "Ask AI: summarize");
    assert_eq!(
        Item::Calc { expression: "2+2".into(), answer: "4".into() }.title(),
        "2+2"
    );
    assert_eq!(
        Item::Menu(Menu { title: "Power".into(), items: vec![] }).title(),
        "Power"
    );
}

#[test]
fn source_items_are_searchable_and_annotated() {
    let clip = Item::Clipboard(zex_launcher::clipboard::Entry::new(
        zex_launcher::clipboard::Content::Text("hello world".into()),
    ));
    assert_eq!(clip.title(), "hello world");
    assert_eq!(clip.subtitle(), Some("text".into()));

    let glyph = zex_launcher::emoji::catalog()[0].clone();
    let emoji = Item::Emoji(glyph.clone());
    assert!(emoji.title().contains(&glyph.mark));
    assert!(emoji.title().contains(&glyph.label));
    assert_eq!(emoji.subtitle(), Some(glyph.label));
}

#[test]
fn subtitles_cover_supported_kinds() {
    assert_eq!(Item::Calc { expression: "2+2".into(), answer: "4".into() }.subtitle(), Some("4".into()));
    assert_eq!(Item::Command("ls".into()).subtitle(), None);
    assert_eq!(
        Item::File(PathBuf::from("/tmp/note.md")).subtitle(),
        Some("/tmp".into())
    );
}

#[test]
fn trait_views_agree_with_inherent_methods() {
    let item = Item::Theme("adw-gtk3".into());
    assert_eq!(Identify::headline(&item), item.title());
    assert_eq!(Identify::footnote(&item), item.subtitle());
    assert_eq!(Identify::icon(&item), item.icon_path());
}

#[test]
fn launch_runs_commands() {
    let item = Item::Command("true".into());
    Launchable::launch(&item).unwrap();
}

#[test]
fn launch_rejects_empty_commands() {
    let item = Item::Command("   ".into());
    assert!(Launchable::launch(&item).is_err());
}

#[test]
fn menus_and_calculations_are_passive() {
    assert!(Launchable::launch(&Item::Menu(Menu { title: "Power".into(), items: vec![] })).is_ok());
    assert!(Launchable::launch(&Item::Calc { expression: "2+2".into(), answer: "4".into() }).is_ok());
}