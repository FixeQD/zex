//! JSON persistence for [`Settings`] with atomic writes and a change subscription mechanism

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
};

use anyhow::{Context, Result};

use crate::settings::Settings;

const SETTINGS_FILE: &str = "settings.json";

/// Change listener kept alive by a [`Subscription`]
type Listener = dyn Fn(&Settings) + Send + Sync;

pub struct SettingsStore {
    path: PathBuf,
    settings: Settings,
    listeners: Arc<Mutex<Vec<Weak<Listener>>>>,
}

/// Removes its listener on drop
pub struct Subscription {
    guard: Arc<Listener>,
    listeners: Arc<Mutex<Vec<Weak<Listener>>>>,
}

impl SettingsStore {
    /// Default settings path: `$XDG_CONFIG_HOME/zex/settings.json`
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("zex")
            .join(SETTINGS_FILE)
    }

    /// Load settings from the default path, falling back to defaults when the file does not exist
    /// Missing keys inside a partial file keep their defaults
    pub fn load() -> Result<Self> {
        Self::load_from(Self::default_path())
    }

    /// Load settings from an explicit path
    /// used by tests and by thesettings service when the user points `ZEX_CONFIG_DIR` elsewhere
    pub fn load_from(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let settings = match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("parsing settings at {}", path.display()))?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(err) => {
                return Err(err).with_context(|| format!("reading settings at {}", path.display()));
            }
        };
        Ok(Self {
            path,
            settings,
            listeners: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Current settings snapshot
    pub fn get(&self) -> &Settings {
        &self.settings
    }

    /// Mutate settings, persist them atomically and notify subscribers
    pub fn update<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Settings),
    {
        f(&mut self.settings);
        self.save()?;
        self.notify();
        Ok(())
    }

    pub fn subscribe(&self, callback: impl Fn(&Settings) + Send + Sync + 'static) -> Subscription {
        let guard: Arc<Listener> = Arc::new(callback);
        self.listeners
            .lock()
            .expect("settings listener registry poisoned")
            .push(Arc::downgrade(&guard));
        Subscription {
            guard,
            listeners: Arc::clone(&self.listeners),
        }
    }

    fn notify(&self) {
        let mut listeners = self
            .listeners
            .lock()
            .expect("settings listener registry poisoned");
        listeners.retain(|listener| listener.strong_count() > 0);
        let live: Vec<Arc<Listener>> = listeners.iter().filter_map(Weak::upgrade).collect();
        drop(listeners);

        let settings = &self.settings;
        for callback in live {
            callback(settings);
        }
    }

    fn save(&self) -> Result<()> {
        let parent = self
            .path
            .parent()
            .context("settings path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("creating settings directory {}", parent.display()))?;

        let tmp = parent.join(format!(".{SETTINGS_FILE}.tmp"));
        let raw = serde_json::to_string_pretty(&self.settings).context("serializing settings")?;
        fs::write(&tmp, raw)
            .with_context(|| format!("writing temporary settings file {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing settings file {}", self.path.display()))?;
        Ok(())
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut listeners = self
            .listeners
            .lock()
            .expect("settings listener registry poisoned");
        listeners.retain(|listener| {
            !matches!(listener.upgrade(), Some(strong) if Arc::ptr_eq(&strong, &self.guard))
        });
    }
}

/// Convenience for reading the default config dir
pub fn config_dir() -> PathBuf {
    SettingsStore::default_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
