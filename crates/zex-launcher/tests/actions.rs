//! `.desktop` `Actions=` parsing

use std::fs;

use zex_launcher::apps::parse::parse_app_file;

fn write_desktop(dir: &tempfile::TempDir, body: &str) -> std::path::PathBuf {
    let path = dir.path().join("com.example.Test.desktop");
    fs::write(&path, body).unwrap();
    path
}

const WITH_ACTIONS: &str = r#"
[Desktop Entry]
Type=Application
Name=Test
Exec=test-bin
Icon=test-icon
Actions=new-window;other;

[Desktop Action new-window]
Name=New Window
Exec=test-bin --new

[Desktop Action other]
Name=Other
Exec=test-bin --other
"#;

#[test]
fn actions_are_parsed_with_name_and_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_desktop(&dir, WITH_ACTIONS);
    let app = parse_app_file(&path).expect("desktop entry parses");

    assert_eq!(app.actions.len(), 2);
    assert_eq!(app.actions[0].name, "New Window");
    assert_eq!(app.actions[0].command, "test-bin --new");
    assert_eq!(app.actions[1].name, "Other");
    assert_eq!(app.actions[1].command, "test-bin --other");
}

#[test]
fn actions_without_exec_or_name_are_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_desktop(
        &dir,
        r#"
[Desktop Entry]
Type=Application
Name=Test
Exec=test-bin
Actions=broken;

[Desktop Action broken]
Name=Broken
"#,
    );
    let app = parse_app_file(&path).expect("desktop entry parses");
    assert!(app.actions.is_empty());
}

#[test]
fn entry_without_actions_parses_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_desktop(
        &dir,
        r#"
[Desktop Entry]
Type=Application
Name=Test
Exec=test-bin
"#,
    );
    let app = parse_app_file(&path).expect("desktop entry parses");
    assert!(app.actions.is_empty());
}
