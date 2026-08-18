//! Data model for a parsed `.desktop` file

use std::path::PathBuf;

/// A parsed XDG application file
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
    pub source: PathBuf,
}
