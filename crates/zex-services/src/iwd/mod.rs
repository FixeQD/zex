//! Native iwd integration over D-Bus.

mod agent;
mod types;

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

pub use agent::Agent;
pub use types::{AccessPoint, Snapshot, network_sort_key, signal_dbm};

const SERVICE: &str = "net.connman.iwd";
const ROOT: &str = "/net/connman/iwd";
const OBJECT_MANAGER: &str = "org.freedesktop.DBus.ObjectManager";
const ADAPTER: &str = "net.connman.iwd.Adapter";
const DEVICE: &str = "net.connman.iwd.Device";
const STATION: &str = "net.connman.iwd.Station";
const NETWORK: &str = "net.connman.iwd.Network";
const AGENT_MANAGER: &str = "net.connman.iwd.AgentManager";
const AGENT_PATH: &str = "/net/connman/iwd/zex/agent";

async fn managed_objects(
    conn: &Connection,
) -> Result<HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>> {
    let proxy = zbus::Proxy::new(conn, SERVICE, ROOT, OBJECT_MANAGER).await?;
    Ok(proxy.call("GetManagedObjects", &()).await?)
}

async fn proxy<'a>(
    conn: &'a Connection,
    path: &'a OwnedObjectPath,
    iface: &'a str,
) -> Result<zbus::Proxy<'a>> {
    Ok(zbus::Proxy::new(conn, SERVICE, path.as_str(), iface).await?)
}

fn property<T: zbus::zvariant::Type + TryFrom<OwnedValue>>(
    map: &HashMap<String, OwnedValue>,
    key: &str,
) -> Option<T>
where
    <T as TryFrom<OwnedValue>>::Error: std::fmt::Debug,
{
    map.get(key).cloned().and_then(|v| T::try_from(v).ok())
}

pub async fn available(conn: &Connection) -> bool {
    zbus::Proxy::new(conn, SERVICE, ROOT, OBJECT_MANAGER)
        .await
        .is_ok()
}

pub async fn session() -> Result<Connection> {
    Connection::system()
        .await
        .map_err(|err| anyhow!(err))
        .context("connecting to iwd system bus")
}

async fn station(conn: &Connection) -> Result<(OwnedObjectPath, String, OwnedObjectPath)> {
    for (path, interfaces) in managed_objects(conn).await? {
        if let Some(device) = interfaces.get(DEVICE) {
            if property::<String>(device, "Name").as_deref() == Some("wlan0")
                || property::<String>(device, "Name").is_some()
            {
                if interfaces.contains_key(STATION) {
                    let name = property::<String>(device, "Name").unwrap_or_else(|| "wlan0".into());
                    let adapter = path
                        .as_str()
                        .rsplit_once('/')
                        .map(|(p, _)| OwnedObjectPath::try_from(p).ok())
                        .flatten()
                        .ok_or_else(|| anyhow!("invalid iwd adapter path"))?;
                    return Ok((path, name, adapter));
                }
            }
        }
    }
    Err(anyhow!("iwd station device not found"))
}

pub async fn snapshot(conn: &Connection) -> Result<Snapshot> {
    let objects = managed_objects(conn).await?;
    let mut adapter_powered = false;
    let mut adapter_name = String::new();
    let mut station_path = None;
    let mut interface = String::new();
    let mut state = "disconnected".to_string();
    let mut scanning = false;
    let mut connected_path = None;

    for (path, interfaces) in &objects {
        if let Some(adapter) = interfaces.get(ADAPTER) {
            adapter_powered = property::<bool>(adapter, "Powered").unwrap_or(false);
            adapter_name = property::<String>(adapter, "Name").unwrap_or_else(|| path.to_string());
        }
        if let Some(device) = interfaces.get(DEVICE) {
            interface = property::<String>(device, "Name").unwrap_or_else(|| interface.clone());
        }
        if let Some(station_props) = interfaces.get(STATION) {
            station_path = Some(path.clone());
            state =
                property::<String>(station_props, "State").unwrap_or_else(|| "disconnected".into());
            scanning = property::<bool>(station_props, "Scanning").unwrap_or(false);
            connected_path = property::<OwnedObjectPath>(station_props, "ConnectedNetwork");
        }
    }

    let mut networks = Vec::new();
    if let Some(station_path) = station_path {
        let station_proxy = proxy(conn, &station_path, STATION).await?;
        let ordered: Vec<(OwnedObjectPath, i16)> =
            station_proxy.call("GetOrderedNetworks", &()).await?;
        for (path, signal) in ordered {
            let network_proxy = proxy(conn, &path, NETWORK).await?;
            let name = network_proxy
                .get_property::<String>("Name")
                .await
                .unwrap_or_default();
            let kind = network_proxy
                .get_property::<String>("Type")
                .await
                .unwrap_or_else(|_| "unknown".into());
            let connected = network_proxy
                .get_property::<bool>("Connected")
                .await
                .unwrap_or(false);
            let known = network_proxy
                .get_property::<OwnedObjectPath>("KnownNetwork")
                .await
                .is_ok();
            networks.push(AccessPoint {
                path,
                ssid: name,
                signal_dbm: signal / 100,
                security: kind,
                connected,
                known,
            });
        }
    }

    Ok(Snapshot {
        adapter: adapter_name,
        adapter_powered,
        interface,
        state,
        scanning,
        connected: connected_path.map(|p| p.to_string()),
        networks,
    })
}

pub async fn set_power(conn: &Connection, enabled: bool) -> Result<()> {
    let objects = managed_objects(conn).await?;
    for (path, interfaces) in objects {
        if interfaces.contains_key(ADAPTER) {
            proxy(conn, &path, ADAPTER)
                .await?
                .set_property("Powered", enabled)
                .await?;
            return Ok(());
        }
    }
    Err(anyhow!("iwd adapter not found"))
}

pub async fn scan(conn: &Connection) -> Result<()> {
    let (path, _, _) = station(conn).await?;
    proxy(conn, &path, STATION)
        .await?
        .call::<_, _, ()>("Scan", &())
        .await?;
    Ok(())
}

pub async fn register_agent(conn: &Connection, agent: Agent) -> Result<()> {
    conn.object_server().at(AGENT_PATH, agent).await?;
    let manager = zbus::Proxy::new(conn, SERVICE, ROOT, AGENT_MANAGER).await?;
    manager
        .call::<_, _, ()>("RegisterAgent", &(OwnedObjectPath::try_from(AGENT_PATH)?))
        .await?;
    Ok(())
}

pub async fn connect(
    conn: &Connection,
    agent: &Agent,
    network: &OwnedObjectPath,
    passphrase: Option<String>,
) -> Result<()> {
    agent.set_passphrase(passphrase);
    let result = proxy(conn, network, NETWORK)
        .await?
        .call::<_, _, ()>("Connect", &())
        .await;
    agent.clear();
    result.map_err(Into::into)
}

pub async fn disconnect(conn: &Connection) -> Result<()> {
    let (path, _, _) = station(conn).await?;
    proxy(conn, &path, STATION)
        .await?
        .call::<_, _, ()>("Disconnect", &())
        .await?;
    Ok(())
}
