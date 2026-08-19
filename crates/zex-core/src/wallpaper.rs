//! Wallpaper state: the current image path + monotonic version that renderers use to skip redundant work

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WallpaperState {
    pub path: PathBuf,
    pub version: u64,
}

impl WallpaperState {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            path: PathBuf::from(&settings.appearance.wallcolors.wallpaper_path),
            version: 0,
        }
    }

    pub fn update(&mut self, path: impl Into<PathBuf>) -> u64 {
        self.path = path.into();
        self.version += 1;
        self.version
    }

    pub fn resolve(&self) -> Option<PathBuf> {
        if self.path.as_os_str().is_empty() {
            return None;
        }
        let path = shellexpand_path(&self.path);
        path.exists().then_some(path)
    }
}

fn shellexpand_path(path: &Path) -> PathBuf {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let raw = path.as_os_str().as_bytes();
    if raw.starts_with(b"~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        let mut expanded = PathBuf::from(home);
        expanded.push(OsStr::from_bytes(&raw[2..]));
        return expanded;
    }
    path.to_path_buf()
}
