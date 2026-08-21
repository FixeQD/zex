use anyhow::{Context, Result};
use flume::Sender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zex_shell::bar::BarsMsg;
use zex_shell::launcher::LauncherMsg;
use zex_shell::overlays::osd::OsdMsg;
use zex_shell::overlays::quickcenter::QuickCenterMsg;
use zex_shell::overlays::popup::PopupsMsg;
use zex_shell::overlays::powermenu::PowermenuMsg;
use zex_shell::wallpaper::WallpaperMsg;
use zex_shell::corners::CornersMsg;
use zex_shell::lockscreen::LockscreenMsg;
use zex_shell::settings::SettingsMsg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    Bar { monitor: usize, bar_id: u8 },
    Launcher,
    QuickCenter,
    Osd,
    Powermenu,
    Wallpaper { monitor: usize },
    Corner { name: String },
    Notification { monitor: usize },
    Lockscreen { monitor: usize },
    Settings,
    Popups,
}

#[derive(Debug, Clone)]
pub enum WindowAction {
    Open(WindowKind),
    Toggle(WindowKind),
    Close(WindowKind),
    CloseAll,
    Refresh,
}

pub struct WindowManager {
    windows: Arc<Mutex<HashMap<WindowKind, Sender<WindowCmd>>>>,
    actions: Arc<Mutex<HashMap<WindowKind, Box<dyn Fn() + Send + Sync>>>>,
    bars_tx: Option<flume::Sender<BarsMsg>>,
    launcher_tx: Option<flume::Sender<LauncherMsg>>,
    quickcenter_tx: Option<flume::Sender<QuickCenterMsg>>,
    osd_tx: Option<flume::Sender<OsdMsg>>,
    powermenu_tx: Option<flume::Sender<PowermenuMsg>>,
    wallpaper_tx: Option<flume::Sender<WallpaperMsg>>,
    corners_tx: Option<flume::Sender<CornersMsg>>,
    lockscreen_tx: Option<flume::Sender<LockscreenMsg>>,
    settings_tx: Option<flume::Sender<SettingsMsg>>,
    popups_tx: Option<flume::Sender<PopupsMsg>>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Arc::new(Mutex::new(HashMap::new())),
            actions: Arc::new(Mutex::new(HashMap::new())),
            bars_tx: None,
            launcher_tx: None,
            quickcenter_tx: None,
            osd_tx: None,
            powermenu_tx: None,
            wallpaper_tx: None,
            corners_tx: None,
            lockscreen_tx: None,
            settings_tx: None,
            popups_tx: None,
        }
    }

    pub fn set_bars_sender(&mut self, tx: flume::Sender<BarsMsg>) {
        self.bars_tx = Some(tx);
    }

    pub fn set_launcher_sender(&mut self, tx: flume::Sender<LauncherMsg>) {
        self.launcher_tx = Some(tx);
    }

    pub fn set_quickcenter_sender(&mut self, tx: flume::Sender<QuickCenterMsg>) {
        self.quickcenter_tx = Some(tx);
    }

    pub fn set_osd_sender(&mut self, tx: flume::Sender<OsdMsg>) {
        self.osd_tx = Some(tx);
    }

    pub fn set_powermenu_sender(&mut self, tx: flume::Sender<PowermenuMsg>) {
        self.powermenu_tx = Some(tx);
    }

    pub fn set_wallpaper_sender(&mut self, tx: flume::Sender<WallpaperMsg>) {
        self.wallpaper_tx = Some(tx);
    }

    pub fn set_corners_sender(&mut self, tx: flume::Sender<CornersMsg>) {
        self.corners_tx = Some(tx);
    }

    pub fn set_lockscreen_sender(&mut self, tx: flume::Sender<LockscreenMsg>) {
        self.lockscreen_tx = Some(tx);
    }

    pub fn set_settings_sender(&mut self, tx: flume::Sender<SettingsMsg>) {
        self.settings_tx = Some(tx);
    }

    pub fn set_popups_sender(&mut self, tx: flume::Sender<PopupsMsg>) {
        self.popups_tx = Some(tx);
    }

    pub fn register_action(&self, kind: WindowKind, action: impl Fn() + Send + Sync + 'static) {
        self.actions.lock().unwrap().insert(kind, Box::new(action));
    }

    pub fn unregister_action(&self, kind: &WindowKind) {
        self.actions.lock().unwrap().remove(kind);
    }

    pub fn open(&self, kind: WindowKind) -> Result<()> {
        if let Some(action) = self.actions.lock().unwrap().get(&kind) {
            action();
            Ok(())
        } else {
            Err(anyhow::anyhow!("no action registered for {:?}", kind))
        }
    }

    pub fn toggle(&self, kind: WindowKind) -> Result<()> {
        self.open(kind)
    }

    pub fn close(&self, kind: WindowKind) -> Result<()> {
        match kind {
            WindowKind::Launcher => {
                if let Some(tx) = &self.launcher_tx {
                    tx.send(LauncherMsg::Close).ok();
                }
            }
            WindowKind::QuickCenter => {
                if let Some(tx) = &self.quickcenter_tx {
                    tx.send(QuickCenterMsg::Close).ok();
                }
            }
            WindowKind::Osd => {
                if let Some(tx) = &self.osd_tx {
                    tx.send(OsdMsg::Close).ok();
                }
            }
            WindowKind::Powermenu => {
                if let Some(tx) = &self.powermenu_tx {
                    tx.send(PowermenuMsg::Close).ok();
                }
            }
            WindowKind::Settings => {
                if let Some(tx) = &self.settings_tx {
                    tx.send(SettingsMsg::Close).ok();
                }
            }
            WindowKind::Popups => {
                if let Some(tx) = &self.popups_tx {
                    tx.send(PopupsMsg::Close).ok();
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn refresh(&self) {
        if let Some(tx) = &self.bars_tx {
            tx.send(BarsMsg::Refresh).ok();
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}