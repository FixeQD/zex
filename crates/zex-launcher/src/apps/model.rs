//! Data model for a parsed `.desktop` file

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A `[Desktop Action x]` entry declared in the `Actions=` field
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesktopAction {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppInfo {
    pub id: String,
    pub title: String,
    pub command: String,
    pub icon_name: Option<String>,
    pub icon_file: Option<PathBuf>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub wants_terminal: bool,
    pub actions: Vec<DesktopAction>,
    pub source: PathBuf,
}
