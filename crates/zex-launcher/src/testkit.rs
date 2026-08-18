//! Shared test fixtures and factories for the launcher engine

use std::fs;
use std::path::Path;

/// Write a minimal, valid `.desktop` file and return its path
pub fn make_entry(dir: &Path, name: &str) -> std::path::PathBuf {
    let id = name.to_lowercase().replace(' ', "-");
    let path = dir.join(format!("{id}.desktop"));
    fs::write(
        &path,
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={name}\n\
             Exec={id} %u\n\
             Icon={id}\n\
             Categories=Network;WebBrowser;\n"
        ),
    )
    .unwrap();
    path
}

/// Write a `.desktop` file with arbitrary content and return its path
pub fn put_desktop(dir: &Path, filename: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(filename);
    fs::write(&path, body).unwrap();
    path
}
