//! Backlight control through the sysfs interface

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub const SYSFS_DIR: &str = "/sys/class/backlight";

#[derive(Debug, Clone)]
pub struct Backlight {
    device_dir: PathBuf,
}

impl Backlight {
    pub fn detect() -> Option<Self> {
        Self::detect_with_dir(Path::new(SYSFS_DIR))
    }

    /// The first backlight device under a custom directory (used for tests)
    pub fn detect_with_dir(dir: &Path) -> Option<Self> {
        let mut entries = std::fs::read_dir(dir)
            .ok()?
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        entries.sort_by_key(|entry| entry.file_name());
        entries
            .into_iter()
            .map(|entry| Self::from_dir(&entry.path()))
            .next()
    }

    /// Backlight for a specific device directory (used for tests).
    pub fn from_dir(dir: &Path) -> Self {
        Self {
            device_dir: dir.to_path_buf(),
        }
    }

    /// Name of the device directory (e.g. `intel_backlight`).
    pub fn device_name(&self) -> Option<&str> {
        self.device_dir.file_name().and_then(|name| name.to_str())
    }

    fn read_value(&self, file: &str) -> Result<u32> {
        let value = std::fs::read_to_string(self.device_dir.join(file))
            .with_context(|| format!("read {}", self.device_dir.join(file).display()))?;
        value
            .trim()
            .parse()
            .with_context(|| format!("parse {} from {}", file, self.device_dir.display()))
    }

    pub fn max_brightness(&self) -> Result<u32> {
        self.read_value("max_brightness")
    }

    pub fn brightness(&self) -> Result<u32> {
        self.read_value("brightness")
    }

    /// Current brightness as a fraction of the maximum (`0.0 ..= 1.0`)
    pub fn percent(&self) -> Result<f32> {
        let max = self.max_brightness()?;
        if max == 0 {
            return Ok(0.0);
        }
        Ok(self.brightness()? as f32 / max as f32)
    }

    /// Set the brightness to an absolute value
    pub fn set_brightness(&self, value: u32) -> Result<()> {
        let max = self.max_brightness()?;
        let value = value.min(max);
        std::fs::write(self.device_dir.join("brightness"), value.to_string())
            .with_context(|| format!("write {}", self.device_dir.join("brightness").display()))
    }

    /// Set the brightness as a fraction of the maximum, clamped to `0.0 ..= 1.0`
    pub fn set_percent(&self, percent: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&percent) {
            bail!("backlight percent {percent} out of range");
        }
        let max = self.max_brightness()?;
        self.set_brightness((percent * max as f32).round() as u32)
    }
}
