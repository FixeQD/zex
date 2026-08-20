//! Native NetworkManager integration used by Settings.

mod agent;
mod types;

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, Value};

pub use agent::SecretAgent;
pub use types::{AccessPoint, EthernetStatus, NetworkSnapshot, is_secure, sort_access_points};

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const NM_IFACE: &str = "org.freedesktop.NetworkManager";
const DEVICE_IFACE: &str = "org.freedesktop.NetworkManager.Device";
const WIFI_IFACE: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const AP_IFACE: &str = "org.freedesktop.NetworkManager.AccessPoint";
const AGENT_PATH: &str = "/org/freedesktop/NetworkManager/SecretAgent/zex";

async fn proxy<'a>(conn: &'a Connection, path: &'a str, iface: &'a str) -> Result<zbus::Proxy<'a>> {
    Ok(zbus::Proxy::new(conn, NM, path, iface).await?)
}

async fn devices(conn: &Connection) -> Result<Vec<OwnedObjectPath>> {
    let p = proxy(conn, NM_PATH, NM_IFACE).await?;
    Ok(p.call("GetDevices", &()).await?)
}

async fn device_type(conn: &Connection, path: &OwnedObjectPath) -> Result<u32> {
    let p = proxy(conn, path.as_str(), DEVICE_IFACE).await?;
    Ok(p.get_property("DeviceType").await?)
}

async fn device_interface(conn: &Connection, path: &OwnedObjectPath) -> Result<String> {
    let p = proxy(conn, path.as_str(), DEVICE_IFACE).await?;
    Ok(p.get_property("Interface").await?)
}

async fn wifi_device(conn: &Connection) -> Result<OwnedObjectPath> {
    for path in devices(conn).await? {
        if device_type(conn, &path).await.unwrap_or(0) == 2 {
            return Ok(path);
        }
    }
    Err(anyhow!("no Wi-Fi device found"))
}

async fn access_points(conn: &Connection, wifi: &OwnedObjectPath) -> Result<Vec<AccessPoint>> {
    let p = proxy(conn, wifi.as_str(), WIFI_IFACE).await?;
    let paths: Vec<OwnedObjectPath> = p.call("GetAccessPoints", &()).await?;
    let mut result = Vec::with_capacity(paths.len());
    for path in paths {
        let ap = proxy(conn, path.as_str(), AP_IFACE).await?;
        let ssid: Vec<u8> = ap.get_property("Ssid").await.unwrap_or_default();
        let flags: u32 = ap.get_property("Flags").await.unwrap_or(0);
        let wpa: u32 = ap.get_property("WpaFlags").await.unwrap_or(0);
        let rsn: u32 = ap.get_property("RsnFlags").await.unwrap_or(0);
        let strength: u8 = ap.get_property("Strength").await.unwrap_or(0);
        let frequency: u32 = ap.get_property("Frequency").await.unwrap_or(0);
        result.push(AccessPoint {
            path,
            ssid: String::from_utf8_lossy(&ssid).into_owned(),
            strength,
            secured: is_secure(flags, wpa, rsn),
            frequency,
        });
    }
    sort_access_points(&mut result);
    result.dedup_by(|a, b| a.ssid == b.ssid);
    Ok(result)
}

pub async fn snapshot(conn: &Connection) -> Result<NetworkSnapshot> {
    let manager = proxy(conn, NM_PATH, NM_IFACE).await?;
    let wifi_enabled: bool = manager.get_property("WirelessEnabled").await?;
    let wifi = wifi_device(conn).await.ok();
    let access_points = match wifi.as_ref() {
        Some(path) if wifi_enabled => access_points(conn, path).await.unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut ethernet = EthernetStatus {
        connected: false,
        interface: None,
    };
    for path in devices(conn).await? {
        if device_type(conn, &path).await.unwrap_or(0) == 1 {
            let state: u32 = proxy(conn, path.as_str(), DEVICE_IFACE)
                .await?
                .get_property("State")
                .await
                .unwrap_or(0);
            if state == 100 {
                ethernet.connected = true;
                ethernet.interface = device_interface(conn, &path).await.ok();
                break;
            }
        }
    }
    Ok(NetworkSnapshot {
        wifi_enabled,
        access_points,
        ethernet,
    })
}

pub async fn set_wifi(conn: &Connection, enabled: bool) -> Result<()> {
    proxy(conn, NM_PATH, NM_IFACE)
        .await?
        .set_property("WirelessEnabled", enabled)
        .await?;
    Ok(())
}

pub async fn request_scan(conn: &Connection) -> Result<()> {
    let wifi = wifi_device(conn).await?;
    let p = proxy(conn, wifi.as_str(), WIFI_IFACE).await?;
    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let _: () = p.call("RequestScan", &(options)).await?;
    Ok(())
}

pub async fn register_secret_agent(conn: &Connection, agent: SecretAgent) -> Result<()> {
    conn.object_server().at(AGENT_PATH, agent).await?;
    let manager = zbus::Proxy::new(
        conn,
        NM,
        "/org/freedesktop/NetworkManager/AgentManager",
        "org.freedesktop.NetworkManager.AgentManager",
    )
    .await?;
    let _: () = manager.call("Register", &("zex", 0u32)).await?;
    Ok(())
}

pub async fn activate_ssid(
    conn: &Connection,
    agent: &SecretAgent,
    ssid: &str,
    password: Option<&str>,
) -> Result<()> {
    if let Some(password) = password.filter(|value| !value.is_empty()) {
        agent.set_secret(ssid, password);
    }
    let wifi = wifi_device(conn).await?;
    let aps = access_points(conn, &wifi).await?;
    let ap = aps
        .into_iter()
        .find(|ap| ap.ssid == ssid)
        .context("access point disappeared")?;
    let mut settings: HashMap<&str, HashMap<&str, Value<'_>>> = HashMap::new();
    let mut connection = HashMap::new();
    connection.insert("id", Value::from(ssid));
    connection.insert("type", Value::from("802-11-wireless"));
    settings.insert("connection", connection);
    let mut wireless = HashMap::new();
    wireless.insert("ssid", Value::from(ssid.as_bytes().to_vec()));
    settings.insert("802-11-wireless", wireless);
    if password.is_some() || ap.secured {
        let mut security = HashMap::new();
        security.insert("key-mgmt", Value::from("wpa-psk"));
        settings.insert("802-11-wireless-security", security);
    }
    let _: (OwnedObjectPath, OwnedObjectPath) = proxy(conn, NM_PATH, NM_IFACE)
        .await?
        .call("AddAndActivateConnection", &(settings, wifi, ap.path))
        .await?;
    Ok(())
}

pub async fn disconnect_wifi(conn: &Connection) -> Result<()> {
    let wifi = wifi_device(conn).await?;
    let state: u32 = proxy(conn, wifi.as_str(), DEVICE_IFACE)
        .await?
        .get_property("State")
        .await
        .unwrap_or(0);
    if state >= 30 {
        let _: () = proxy(conn, NM_PATH, NM_IFACE)
            .await?
            .call("DeactivateConnection", &(wifi))
            .await?;
    }
    Ok(())
}

pub async fn session() -> Result<Connection> {
    Connection::system()
        .await
        .map_err(|err| anyhow!(err))
        .context("connecting to NetworkManager system bus")
}
