//! Application engine: parsing, discovery, indexing, monitoring and launch.

mod discover;
pub mod model;
pub mod parse;
mod pinned;
mod run;
mod session;
mod store;
mod watch;

pub use discover::{collect_apps, collect_from, xdg_app_dirs};
pub use model::{AppInfo, DesktopAction};
pub use pinned::{PinnedApps, default_pins_path};
pub use run::{DEFAULT_TERMINAL_TEMPLATE, spawn_command, spawn_entry, strip_field_codes};
pub use session::session_env;
pub use store::{Store, default_store_path, dir_mtimes};
pub use watch::{Change, Watchdog, translate};

use crate::icons::find_icon_file;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, warn};

/// Load all applications, preferring a valid index over a fresh scan.
///
/// When `store_path` is `Some`, the index is validated against the current
/// XDG directories and file modification times; a stale or missing index is
/// rebuilt from a fresh scan. Icon files are resolved after loading.
pub fn load_apps(store_path: Option<&Path>) -> Result<Vec<AppInfo>> {
    let mtimes = dir_mtimes();
    let apps = match store_path {
        Some(path) => match Store::load(path) {
            Ok(Some(store)) if store.fresh(&mtimes) => match store.snapshot() {
                Ok(apps) => {
                    debug!("index hit ({} applications)", apps.len());
                    apps
                }
                Err(e) => {
                    warn!("corrupt index, rescanning: {e}");
                    rebuild(path, &mtimes)
                }
            },
            Ok(_) => rebuild(path, &mtimes),
            Err(e) => {
                warn!("index unavailable, rescanning: {e}");
                collect_apps()
            }
        },
        None => collect_apps(),
    };
    Ok(apps.into_iter().map(fill_icon).collect())
}

fn rebuild(path: &Path, mtimes: &HashMap<PathBuf, SystemTime>) -> Vec<AppInfo> {
    let apps = collect_apps();
    match Store::write(path, &apps, mtimes) {
        Ok(()) => debug!("index saved ({} applications)", apps.len()),
        Err(e) => warn!("failed to write index: {e}"),
    }
    apps
}

fn fill_icon(mut app: AppInfo) -> AppInfo {
    if app.icon_file.is_none() {
        if let Some(name) = &app.icon_name {
            app.icon_file = find_icon_file(name);
        }
    }
    app
}
