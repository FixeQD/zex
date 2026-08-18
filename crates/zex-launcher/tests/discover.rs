use tempfile::TempDir;
use zex_launcher::apps::{collect_from, xdg_app_dirs};
use zex_launcher::testkit::{make_entry, put_desktop};

#[test]
fn scans_and_sorts_by_title() {
    let dir = TempDir::new().unwrap();
    make_entry(&dir.path(), "Zsh");
    make_entry(&dir.path(), "Alacritty");
    make_entry(&dir.path(), "mako");
    let apps = collect_from(&[dir.path().to_path_buf()]);
    let titles: Vec<&str> = apps.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["Alacritty", "mako", "Zsh"]);
}

#[test]
fn descends_into_subdirectories() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("kde");
    std::fs::create_dir_all(&sub).unwrap();
    put_desktop(
        &sub,
        "nested.desktop",
        "[Desktop Entry]\nType=Application\nName=Nested\nExec=true\n",
    );
    let apps = collect_from(&[dir.path().to_path_buf()]);
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].id, "nested");
}

#[test]
fn first_directory_wins_on_duplicate_id() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    put_desktop(
        &first.path(),
        "dupe.desktop",
        "[Desktop Entry]\nType=Application\nName=First\nExec=first\n",
    );
    put_desktop(
        &second.path(),
        "dupe.desktop",
        "[Desktop Entry]\nType=Application\nName=Second\nExec=second\n",
    );
    let apps = collect_from(&[first.path().to_path_buf(), second.path().to_path_buf()]);
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].title, "First");
}

#[test]
fn ignores_foreign_files() {
    let dir = TempDir::new().unwrap();
    put_desktop(
        &dir.path(),
        "real.desktop",
        "[Desktop Entry]\nType=Application\nName=Real\nExec=true\n",
    );
    std::fs::write(dir.path().join("readme.txt"), "not an app").unwrap();
    let apps = collect_from(&[dir.path().to_path_buf()]);
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].id, "real");
}

#[test]
fn missing_directory_is_tolerated() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("does-not-exist");
    assert!(collect_from(&[missing]).is_empty());
}

#[test]
fn xdg_dirs_are_not_empty() {
    assert!(!xdg_app_dirs().is_empty());
}