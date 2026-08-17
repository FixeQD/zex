//! Battery device model and UPower property fetching

use super::{DEVICE_INTERFACE, UPOWER_DESTINATION, UPOWER_INTERFACE, UPOWER_OBJECT_PATH};
use anyhow::{Context, Result};
use zbus::Connection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    Unknown,
    Charging,
    Discharging,
    Empty,
    FullyCharged,
    PendingCharge,
    PendingDischarge,
}

impl BatteryState {
    /// Map the numeric UPower `State` property to a typed state
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => Self::Charging,
            2 => Self::Discharging,
            3 => Self::Empty,
            4 => Self::FullyCharged,
            5 => Self::PendingCharge,
            6 => Self::PendingDischarge,
            _ => Self::Unknown,
        }
    }

    pub fn is_charging(self) -> bool {
        matches!(self, Self::Charging | Self::PendingCharge)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Battery {
    pub path: String,
    pub percent: f32,
    pub state: BatteryState,
    pub is_present: bool,
}

impl Battery {
    pub fn percent_u8(&self) -> u8 {
        self.percent.round().clamp(0.0, 100.0) as u8
    }

    pub fn charging(&self) -> bool {
        self.state.is_charging()
    }

    pub fn icon_name(&self) -> &'static str {
        if self.charging() {
            return "bolt";
        }
        match self.percent_u8() {
            100 => "battery_android_full",
            96..=99 => "battery_android_6",
            81..=95 => "battery_android_5",
            61..=80 => "battery_android_4",
            41..=60 => "battery_android_3",
            26..=40 => "battery_android_2",
            11..=25 => "battery_android_1",
            0..=10 => "battery_android_0",
            _ => "battery_android_question",
        }
    }
}

pub async fn paths(conn: &Connection) -> Result<Vec<String>> {
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(conn)
        .destination(UPOWER_DESTINATION)?
        .path(UPOWER_OBJECT_PATH)?
        .interface(UPOWER_INTERFACE)?
        .build()
        .await?;
    let paths: Vec<zbus::zvariant::OwnedObjectPath> = proxy
        .call("GetAllDevices", &())
        .await
        .context("GetAllDevices")?;
    Ok(paths.into_iter().map(|path| path.to_string()).collect())
}

pub async fn fetch(conn: &Connection, path: &str) -> Result<Option<Battery>> {
    let proxy: zbus::Proxy<'_> = match zbus::proxy::Builder::new(conn)
        .destination(UPOWER_DESTINATION)?
        .path(path)?
        .interface(DEVICE_INTERFACE)?
        .build()
        .await
    {
        Ok(proxy) => proxy,
        Err(_) => return Ok(None),
    };
    let percent: f64 = proxy.get_property("Percentage").await?;
    let state: u32 = proxy.get_property("State").await?;
    let is_present: bool = proxy.get_property("IsPresent").await?;
    Ok(Some(Battery {
        path: path.to_string(),
        percent: percent as f32,
        state: BatteryState::from_u32(state),
        is_present,
    }))
}
