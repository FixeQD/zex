use tempfile::TempDir;
use zex_launcher::load_apps;

#[test]
fn index_is_reused_across_loads() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("index.db");
    let first = load_apps(Some(&path)).unwrap();
    assert!(path.exists(), "index file was written");
    let second = load_apps(Some(&path)).unwrap();
    assert_eq!(first, second);
}