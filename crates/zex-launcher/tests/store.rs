use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use zex_launcher::apps::{AppInfo, Store, collect_from, dir_mtimes, xdg_app_dirs};
use zex_launcher::testkit::make_entry;

fn sample(tmp: &TempDir) -> (Vec<AppInfo>, HashMap<PathBuf, SystemTime>) {
    let apps = tmp.path().join("applications");
    fs::create_dir_all(&apps).unwrap();
    make_entry(&apps, "Alacritty");
    make_entry(&apps, "mako");
    let mtimes = HashMap::from([(
        apps.clone(),
        fs::metadata(&apps).unwrap().modified().unwrap(),
    )]);
    let apps_list = collect_from(&[apps]);
    (apps_list, mtimes)
}

#[test]
fn round_trips_applications() {
    let tmp = TempDir::new().unwrap();
    let (apps, mtimes) = sample(&tmp);
    let path = tmp.path().join("index.db");
    Store::write(&path, &apps, &mtimes).unwrap();

    let store = Store::load(&path).unwrap().expect("index exists");
    assert!(store.fresh(&mtimes));
    let loaded = store.snapshot().unwrap();
    let titles: Vec<&str> = loaded.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["Alacritty", "mako"]);
    assert_eq!(loaded[0].source, apps[0].source);
    assert_eq!(loaded[0].tags, apps[0].tags);
}

#[test]
fn load_returns_none_for_missing_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("index.db");
    assert!(Store::load(&path).unwrap().is_none());
}

#[test]
fn invalidated_when_directory_mtime_changes() {
    let tmp = TempDir::new().unwrap();
    let (apps, mtimes) = sample(&tmp);
    let path = tmp.path().join("index.db");
    Store::write(&path, &apps, &mtimes).unwrap();

    let store = Store::load(&path).unwrap().unwrap();
    assert!(store.fresh(&mtimes));

    let apps_dir = tmp.path().join("applications");
    make_entry(&apps_dir, "NewApp");
    let new_mtimes = HashMap::from([(
        apps_dir.clone(),
        fs::metadata(&apps_dir).unwrap().modified().unwrap(),
    )]);
    assert!(!store.fresh(&new_mtimes));
}

#[test]
fn invalidated_when_source_file_changes() {
    let tmp = TempDir::new().unwrap();
    let (apps, mtimes) = sample(&tmp);
    let path = tmp.path().join("index.db");
    Store::write(&path, &apps, &mtimes).unwrap();

    let store = Store::load(&path).unwrap().unwrap();
    assert!(store.fresh(&mtimes));

    let source = apps[0].source.clone();
    let file = fs::File::options().write(true).open(&source).unwrap();
    file.set_modified(UNIX_EPOCH + Duration::from_secs(123_456_789))
        .unwrap();
    assert!(!store.fresh(&mtimes));
}

#[test]
fn rebuilds_after_schema_change() {
    let tmp = TempDir::new().unwrap();
    let (apps, mtimes) = sample(&tmp);
    let path = tmp.path().join("index.db");
    Store::write(&path, &apps, &mtimes).unwrap();

    let conn = Connection::open(&path).unwrap();
    conn.execute("UPDATE meta SET value = '999' WHERE key = 'version'", [])
        .unwrap();
    drop(conn);
    assert!(Store::load(&path).unwrap().is_none());
}

#[test]
fn reports_current_directory_mtimes() {
    let mtimes = dir_mtimes();
    assert!(!mtimes.is_empty());
    for dir in xdg_app_dirs() {
        if dir.exists() {
            assert!(mtimes.contains_key(&dir));
        }
    }
}
