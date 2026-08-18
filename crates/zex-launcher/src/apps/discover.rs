//! Discovery of installed applications from XDG directories

use super::model::AppInfo;
use super::parse::parse_app_file;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Application directories in XDG precedence order: `$XDG_DATA_HOME/applications` first, then every `$XDG_DATA_DIRS` entry
/// Falling back to the standard system locations when the variable is unset
pub fn xdg_app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data_home) = dirs::data_local_dir() {
        dirs.push(data_home.join("applications"));
    }
    match std::env::var("XDG_DATA_DIRS") {
        Ok(xdg_dirs) => {
            for dir in xdg_dirs.split(':') {
                if !dir.is_empty() {
                    dirs.push(PathBuf::from(dir).join("applications"));
                }
            }
        }
        Err(_) => {
            dirs.push(PathBuf::from("/usr/local/share/applications"));
            dirs.push(PathBuf::from("/usr/share/applications"));
        }
    }
    dirs
}

pub fn collect_apps() -> Vec<AppInfo> {
    collect_from(&xdg_app_dirs())
}

pub fn collect_from(dirs: &[PathBuf]) -> Vec<AppInfo> {
    let mut apps: BTreeMap<String, AppInfo> = BTreeMap::new();
    for dir in dirs {
        walk(dir, &mut apps);
    }
    let mut result: Vec<AppInfo> = apps.into_values().collect();
    result.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    result
}

fn walk(dir: &Path, apps: &mut BTreeMap<String, AppInfo>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, apps);
        } else if path.extension().is_some_and(|ext| ext == "desktop") {
            if let Some(app) = parse_app_file(&path) {
                apps.entry(app.id.clone()).or_insert(app);
            }
        }
    }
}
