//! UPower battery monitoring over the session bus

mod device;
mod monitor;

pub use device::{Battery, BatteryState};

use anyhow::Result;
use flume::Receiver;
use tokio::task::JoinHandle;
use zbus::Connection;

pub const UPOWER_DESTINATION: &str = "org.freedesktop.UPower";
pub const UPOWER_OBJECT_PATH: &str = "/org/freedesktop/UPower";
pub const UPOWER_INTERFACE: &str = "org.freedesktop.UPower";
pub const DEVICE_INTERFACE: &str = "org.freedesktop.UPower.Device";
const DEVICES_PATH_PREFIX: &str = "/org/freedesktop/UPower/devices";

#[derive(Debug, Clone, PartialEq)]
pub enum UpowerEvent {
    BatteryChanged(Battery),
    DeviceAdded(String),
    DeviceRemoved(String),
}

pub struct Upower {
    conn: Connection,
    events: Receiver<UpowerEvent>,
    _task: JoinHandle<()>,
}

impl Upower {
    pub async fn connect(conn: Connection) -> Result<Self> {
        let (tx, rx) = flume::unbounded();
        let task = tokio::spawn(monitor::run(conn.clone(), tx));
        Ok(Self {
            conn,
            events: rx,
            _task: task,
        })
    }

    pub fn events(&self) -> &Receiver<UpowerEvent> {
        &self.events
    }

    pub async fn batteries(&self) -> Result<Vec<Battery>> {
        let paths = device::paths(&self.conn).await?;
        let mut batteries = Vec::new();
        for path in paths {
            if let Some(battery) = device::fetch(&self.conn, &path).await? {
                batteries.push(battery);
            }
        }
        Ok(batteries)
    }
}
