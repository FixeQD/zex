//! Pinned app ids: JSON file store plus change events.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// `~/.config/zex/pinned.json`
pub fn default_pins_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("zex")
        .join("pinned.json")
}

#[derive(Default, Serialize, Deserialize)]
struct PinnedFile {
    ids: Vec<String>,
}

/// Thread-safe pin registry; every mutation is persisted and broadcast
pub struct PinnedApps {
    state: Mutex<PinnedState>,
    path: PathBuf,
    tx: flume::Sender<Vec<String>>,
    rx: flume::Receiver<Vec<String>>,
}

struct PinnedState {
    ids: Vec<String>,
}

impl PinnedApps {
    pub fn load(path: Option<&Path>) -> Self {
        let path = path.map(Path::to_owned).unwrap_or_else(default_pins_path);
        let ids = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<PinnedFile>(&raw).ok())
            .map(|file| file.ids)
            .unwrap_or_default();
        let (tx, rx) = flume::unbounded();
        Self {
            state: Mutex::new(PinnedState { ids }),
            path,
            tx,
            rx,
        }
    }

    /// Current pinned app ids, in order
    pub fn pinned(&self) -> Vec<String> {
        self.state.lock().expect("pins mutex poisoned").ids.clone()
    }

    pub fn is_pinned(&self, id: &str) -> bool {
        self.state
            .lock()
            .expect("pins mutex poisoned")
            .ids
            .iter()
            .any(|pinned| pinned == id)
    }

    /// Pin when absent, unpin when present, then persist and broadcast
    pub fn toggle(&self, id: &str) {
        let mut state = self.state.lock().expect("pins mutex poisoned");
        if let Some(pos) = state.ids.iter().position(|pinned| pinned == id) {
            state.ids.remove(pos);
        } else {
            state.ids.push(id.to_owned());
        }
        let ids = state.ids.clone();
        drop(state);
        self.save();
        let _ = self.tx.send(ids);
    }

    pub fn changes(&self) -> flume::Receiver<Vec<String>> {
        self.rx.clone()
    }

    fn save(&self) {
        let file = PinnedFile { ids: self.pinned() };
        let raw = match serde_json::to_string_pretty(&file) {
            Ok(raw) => raw,
            Err(err) => {
                tracing::warn!("pins serialization failed: {err}");
                return;
            }
        };
        let result = (|| -> Result<()> {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).context("creating pins dir")?;
            }
            let tmp = self.path.with_extension("json.tmp");
            fs::write(&tmp, raw).context("writing pins file")?;
            fs::rename(&tmp, &self.path).context("moving pins file into place")?;
            Ok(())
        })();
        if let Err(err) = result {
            tracing::warn!("pins persistence failed: {err:#}");
        }
    }
}
