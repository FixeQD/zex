//! Parsing of XDG `.desktop` files

use super::model::{AppInfo, DesktopAction};
use freedesktop_desktop_entry::DesktopEntry as FdEntry;
use std::path::Path;

pub fn parse_app_file(path: &Path) -> Option<AppInfo> {
    let fd_entry = FdEntry::from_path(path, None::<&[&str]>).ok()?;
    if fd_entry.type_() != Some("Application") || fd_entry.no_display() || fd_entry.hidden() {
        return None;
    }
    let locales: &[&str] = &[];
    let title = fd_entry.name(locales)?.to_string();
    let command = fd_entry.exec()?.to_string();

    Some(AppInfo {
        id: fd_entry.id().to_string(),
        title,
        command,
        icon_name: fd_entry.icon().map(ToOwned::to_owned),
        icon_file: None,
        summary: fd_entry.comment(locales).map(|c| c.into_owned()),
        tags: fd_entry
            .categories()
            .map(|cats| {
                cats.into_iter()
                    .filter(|tag| !tag.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        wants_terminal: fd_entry.terminal(),
        actions: fd_entry
            .actions()
            .unwrap_or_default()
            .into_iter()
            .filter(|id| !id.is_empty())
            .filter_map(|id| {
                let name = fd_entry.action_name(&id, locales)?.to_string();
                let command = fd_entry.action_exec(&id)?.trim().to_string();
                if command.is_empty() {
                    return None;
                }
                Some(DesktopAction { name, command })
            })
            .collect(),
        source: path.to_path_buf(),
    })
}
