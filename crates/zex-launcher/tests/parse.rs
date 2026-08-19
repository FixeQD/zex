use tempfile::TempDir;
use zex_launcher::apps::parse::parse_app_file;
use zex_launcher::testkit::{make_entry, put_desktop};

#[test]
fn parses_a_valid_file() {
    let dir = TempDir::new().unwrap();
    let path = make_entry(&dir.path(), "Firefox");
    let app = parse_app_file(&path).expect("valid entry parses");
    assert_eq!(app.id, "firefox");
    assert_eq!(app.title, "Firefox");
    assert_eq!(app.command, "firefox %u");
    assert_eq!(app.icon_name.as_deref(), Some("firefox"));
    assert_eq!(app.tags, vec!["Network", "WebBrowser"]);
    assert!(!app.wants_terminal);
}

#[test]
fn rejects_menu_hidden_entries() {
    let dir = TempDir::new().unwrap();
    let no_display = put_desktop(
        &dir.path(),
        "invisible.desktop",
        "[Desktop Entry]\nType=Application\nName=Invisible\nExec=true\nNoDisplay=true\n",
    );
    let hidden = put_desktop(
        &dir.path(),
        "secret.desktop",
        "[Desktop Entry]\nType=Application\nName=Secret\nExec=true\nHidden=true\n",
    );
    assert!(parse_app_file(&no_display).is_none());
    assert!(parse_app_file(&hidden).is_none());
}

#[test]
fn rejects_non_application_types() {
    let dir = TempDir::new().unwrap();
    let link = put_desktop(
        &dir.path(),
        "pointer.desktop",
        "[Desktop Entry]\nType=Link\nName=Pointer\nExec=true\n",
    );
    assert!(parse_app_file(&link).is_none());
}

#[test]
fn rejects_missing_name_or_exec() {
    let dir = TempDir::new().unwrap();
    let no_name = put_desktop(
        &dir.path(),
        "noname.desktop",
        "[Desktop Entry]\nType=Application\nExec=true\n",
    );
    let no_exec = put_desktop(
        &dir.path(),
        "noexec.desktop",
        "[Desktop Entry]\nType=Application\nName=NoExec\n",
    );
    assert!(parse_app_file(&no_name).is_none());
    assert!(parse_app_file(&no_exec).is_none());
}

#[test]
fn rejects_garbage_files() {
    let dir = TempDir::new().unwrap();
    let garbage = put_desktop(&dir.path(), "garbage.desktop", "not a desktop file");
    assert!(parse_app_file(&garbage).is_none());
}
