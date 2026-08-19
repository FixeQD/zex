use std::path::PathBuf;
use tempfile::TempDir;
use zex_launcher::apps::AppInfo;
use zex_launcher::engine::{Matcher, best_match, rank};
use zex_launcher::load_apps;

fn app(id: &str, name: &str) -> AppInfo {
    AppInfo {
        id: id.into(),
        title: name.into(),
        command: id.into(),
        icon_name: None,
        icon_file: None,
        summary: None,
        tags: vec![],
        wants_terminal: false,
        source: PathBuf::from("/tmp/apps").join(format!("{id}.desktop")),
    }
}

#[test]
fn empty_query_returns_everything() {
    let matcher = Matcher::new();
    let items = vec![
        zex_launcher::Item::App(app("firefox", "Firefox")),
        zex_launcher::Item::App(app("mako", "mako")),
    ];
    let scored = rank(items.iter(), "", &matcher);
    assert_eq!(scored.len(), 2);
}

#[test]
fn non_matching_query_returns_nothing() {
    let matcher = Matcher::new();
    let items = vec![zex_launcher::Item::App(app("firefox", "Firefox"))];
    let scored = rank(items.iter(), "zzzzz", &matcher);
    assert!(scored.is_empty());
}

#[test]
fn exact_match_ranks_first() {
    let matcher = Matcher::new();
    let items = vec![
        zex_launcher::Item::App(app("ghostty", "Ghostty")),
        zex_launcher::Item::App(app("gimp", "GIMP Image Editor")),
    ];
    let scored = rank(items.iter(), "gimp", &matcher);
    assert_eq!(scored.len(), 1);
    assert!(scored[0].score > 0);
    assert_eq!(scored[0].item.title(), "GIMP Image Editor");
}

#[test]
fn best_match_is_case_insensitive() {
    let items = vec![zex_launcher::Item::App(app("firefox", "Firefox"))];
    assert!(best_match(&items, "firefox").is_some());
    assert!(best_match(&items, "FIREFOX").is_some());
    assert!(best_match(&items, "fire").is_none());
}

#[test]
fn index_is_reused_across_loads() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("index.db");
    let first = load_apps(Some(&path)).unwrap();
    assert!(path.exists(), "index file was written");
    let second = load_apps(Some(&path)).unwrap();
    assert_eq!(first, second);
}
