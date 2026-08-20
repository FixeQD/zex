//! Native BlueZ integration used by Settings.

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const BLUEZ: &str = "org.bluez";
const OBJECT_MANAGER: &str = "org.freedesktop.DBus.ObjectManager";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub path: OwnedObjectPath,
    pub address: String,
    pub alias: String,
    pub icon: String,
    pub connected: bool,
    pub paired: bool,
    pub rssi: Option<i16>,
    pub class: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothSnapshot {
    pub available: bool,
    pub powered: bool,
    pub devices: Vec<BluetoothDevice>,
}

pub fn sort_devices(devices: &mut [BluetoothDevice]) {
    devices.sort_by(|a, b| a.alias.to_lowercase().cmp(&b.alias.to_lowercase()));
}

fn value<T: TryFrom<OwnedValue>>(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<T> {
    properties
        .get(key)
        .cloned()
        .and_then(|value| value.try_into().ok())
}

async fn managed(
    conn: &Connection,
) -> Result<HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>> {
    let proxy = zbus::Proxy::new(conn, BLUEZ, "/", OBJECT_MANAGER).await?;
    Ok(proxy.call("GetManagedObjects", &()).await?)
}

pub async fn snapshot(conn: &Connection) -> Result<BluetoothSnapshot> {
    let objects = managed(conn).await?;
    let mut adapters = Vec::new();
    let mut devices = Vec::new();
    for (path, interfaces) in objects {
        if let Some(props) = interfaces.get(ADAPTER) {
            adapters.push((
                path.clone(),
                value::<bool>(props, "Powered").unwrap_or(false),
            ));
        }
        if let Some(props) = interfaces.get(DEVICE) {
            devices.push(BluetoothDevice {
                path,
                address: value(props, "Address").unwrap_or_default(),
                alias: value(props, "Alias")
                    .unwrap_or_else(|| value(props, "Name").unwrap_or_default()),
                icon: value(props, "Icon").unwrap_or_else(|| "bluetooth_connected".into()),
                connected: value(props, "Connected").unwrap_or(false),
                paired: value(props, "Paired").unwrap_or(false),
                rssi: value(props, "RSSI"),
                class: value(props, "Class"),
            });
        }
    }
    let powered = adapters.iter().any(|(_, powered)| *powered);
    devices.sort_by(|a, b| a.alias.to_lowercase().cmp(&b.alias.to_lowercase()));
    Ok(BluetoothSnapshot {
        available: !adapters.is_empty(),
        powered,
        devices,
    })
}

async fn adapter(conn: &Connection) -> Result<OwnedObjectPath> {
    managed(conn)
        .await?
        .into_iter()
        .find_map(|(path, interfaces)| interfaces.contains_key(ADAPTER).then_some(path))
        .context("no Bluetooth adapter found")
}

pub async fn set_powered(conn: &Connection, powered: bool) -> Result<()> {
    let path = adapter(conn).await?;
    let proxy = zbus::Proxy::new(conn, BLUEZ, path.as_str(), ADAPTER).await?;
    proxy.set_property("Powered", powered).await?;
    Ok(())
}

pub async fn start_discovery(conn: &Connection) -> Result<()> {
    let path = adapter(conn).await?;
    {
        let proxy = zbus::Proxy::new(conn, BLUEZ, path.as_str(), ADAPTER).await?;
        let _: () = proxy.call("StartDiscovery", &()).await?;
    }
    Ok(())
}

pub async fn stop_discovery(conn: &Connection) -> Result<()> {
    let path = adapter(conn).await?;
    {
        let proxy = zbus::Proxy::new(conn, BLUEZ, path.as_str(), ADAPTER).await?;
        let _: () = proxy.call("StopDiscovery", &()).await?;
    }
    Ok(())
}

pub async fn connect(conn: &Connection, path: &OwnedObjectPath) -> Result<()> {
    {
        let proxy = zbus::Proxy::new(conn, BLUEZ, path.as_str(), DEVICE).await?;
        let _: () = proxy.call("Connect", &()).await?;
    }
    Ok(())
}

pub async fn disconnect(conn: &Connection, path: &OwnedObjectPath) -> Result<()> {
    {
        let proxy = zbus::Proxy::new(conn, BLUEZ, path.as_str(), DEVICE).await?;
        let _: () = proxy.call("Disconnect", &()).await?;
    }
    Ok(())
}

pub async fn pair(conn: &Connection, path: &OwnedObjectPath) -> Result<()> {
    {
        let proxy = zbus::Proxy::new(conn, BLUEZ, path.as_str(), DEVICE).await?;
        let _: () = proxy.call("Pair", &()).await?;
    }
    Ok(())
}

pub async fn session() -> Result<Connection> {
    Connection::system()
        .await
        .map_err(|err| anyhow!(err))
        .context("connecting to BlueZ system bus")
}
