use std::path::PathBuf;
use zex_launcher::Item;
use zex_launcher::apps::AppInfo;
use zex_launcher::engine::{Matcher, Section, organize};

fn app(id: &str, name: &str) -> Item {
    Item::App(AppInfo {
        id: id.into(),
        title: name.into(),
        command: id.into(),
        icon_name: None,
        icon_file: None,
        summary: None,
        tags: vec![],
        wants_terminal: false,
        source: PathBuf::from("/tmp/apps").join(format!("{id}.desktop")),
    })
}

fn titles(section: &Section) -> Vec<String> {
    section.items.iter().map(|item| item.title()).collect()
}

#[test]
fn browse_query_shows_grouped_catalog() {
    let matcher = Matcher::new();
    let catalog = vec![
        app("firefox", "Firefox"),
        app("mako", "mako"),
        Item::Window {
            title: "Editor".into(),
            app: "helix".into(),
        },
        Item::Menu(zex_launcher::Menu {
            title: "Power".into(),
            items: vec![],
        }),
        Item::Theme("adw-gtk3".into()),
    ];
    let sections = organize(catalog, "", &matcher);
    let section_titles: Vec<&str> = sections.iter().map(|s| s.title).collect();
    assert_eq!(
        section_titles,
        ["Applications", "Windows", "Themes", "Menus"]
    );
    assert_eq!(titles(&sections[0]), ["Firefox", "mako"]);
}

#[test]
fn command_arrow_creates_a_run_section() {
    let matcher = Matcher::new();
    let sections = organize(vec![app("ghostty", "Ghostty")], ">uptime", &matcher);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Run");
    assert_eq!(titles(&sections[0]), ["uptime"]);
}

#[test]
fn bang_trigger_creates_a_web_section() {
    let matcher = Matcher::new();
    let sections = organize(vec![], "!yt never gonna", &matcher);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Web");
    assert_eq!(titles(&sections[0]), ["yt: never gonna"]);
}

#[test]
fn math_query_creates_a_calculator_section() {
    let matcher = Matcher::new();
    let sections = organize(vec![], "2+2", &matcher);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Calculator");
    assert_eq!(titles(&sections[0]), ["2+2"]);
    assert_eq!(sections[0].items[0].subtitle(), Some("4".to_string()));
}

#[test]
fn sections_filter_internal_items() {
    let matcher = Matcher::new();
    let catalog = vec![app("firefox", "Firefox"), app("mako", "mako")];
    let sections = organize(catalog, "fox", &matcher);
    assert_eq!(sections.len(), 1);
    assert_eq!(titles(&sections[0]), ["Firefox"]);
}
