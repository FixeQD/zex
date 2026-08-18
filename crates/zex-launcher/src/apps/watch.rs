//! Live monitoring of XDG application directories.

use flume::{Receiver, TryRecvError};
use notify::event::{CreateKind, RemoveKind};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Installed(PathBuf),
    Removed(PathBuf),
    Touched(PathBuf),
    Rebuild(PathBuf),
}

/// Watches XDG application directories and reports `.desktop` changes
pub struct Watchdog {
    _watcher: RecommendedWatcher,
    rx: Receiver<Change>,
}

impl Watchdog {
    /// Watch all XDG application directories
    pub fn start() -> anyhow::Result<Self> {
        Self::start_on(&super::discover::xdg_app_dirs())
    }

    /// Watch a specific set of directories
    pub fn start_on(dirs: &[PathBuf]) -> anyhow::Result<Self> {
        let (tx, rx) = flume::unbounded();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    for change in translate(event) {
                        if tx.send(change).is_err() {
                            error!("monitor channel closed");
                            break;
                        }
                    }
                }
                Err(e) => warn!("file watcher error: {e}"),
            })?;
        for dir in dirs {
            if dir.exists() {
                match watcher.watch(dir, RecursiveMode::Recursive) {
                    Ok(()) => debug!("watching {:?}", dir),
                    Err(e) => warn!("failed to watch {:?}: {}", dir, e),
                }
            }
        }
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Drain pending changes without blocking
    pub fn pending(&self) -> Vec<Change> {
        let mut changes = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(change) => changes.push(change),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    error!("monitor channel disconnected");
                    break;
                }
            }
        }
        changes
    }

    /// Wait up to `timeout` for changes then drain whatever is pending
    pub fn next(&self, timeout: Duration) -> Vec<Change> {
        let mut changes = Vec::new();
        match self.rx.recv_timeout(timeout) {
            Ok(change) => {
                changes.push(change);
                changes.extend(self.pending());
            }
            Err(flume::RecvTimeoutError::Timeout) => {}
            Err(flume::RecvTimeoutError::Disconnected) => {
                error!("monitor channel disconnected");
            }
        }
        changes
    }

    /// Async wait for a single change
    pub async fn incoming(&self) -> Result<Change, flume::RecvError> {
        self.rx.recv_async().await
    }
}

/// Map a `notify` event to our change events
pub fn translate(event: Event) -> Vec<Change> {
    let mut changes = Vec::new();
    for path in event.paths {
        let is_desktop = path.extension().is_some_and(|ext| ext == "desktop");
        let change = match event.kind {
            EventKind::Create(_) if is_desktop => {
                debug!("desktop file created: {:?}", path);
                Some(Change::Installed(path))
            }
            EventKind::Remove(_) if is_desktop => {
                debug!("desktop file removed: {:?}", path);
                Some(Change::Removed(path))
            }
            EventKind::Modify(_) if is_desktop => {
                debug!("desktop file modified: {:?}", path);
                Some(Change::Touched(path))
            }
            EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder) => {
                debug!("folder changed: {:?}", path);
                Some(Change::Rebuild(path))
            }
            _ => None,
        };
        if let Some(change) = change {
            changes.push(change);
        }
    }
    changes
}
