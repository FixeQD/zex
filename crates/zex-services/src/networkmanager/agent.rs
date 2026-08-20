//! NetworkManager SecretAgent implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

#[derive(Clone, Default)]
pub struct SecretAgent {
    secrets: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretAgent {
    pub fn set_secret(&self, ssid: impl Into<String>, secret: impl Into<String>) {
        self.secrets
            .lock()
            .expect("NetworkManager secret registry poisoned")
            .insert(ssid.into(), secret.into());
    }

    pub fn forget(&self, ssid: &str) {
        self.secrets
            .lock()
            .expect("NetworkManager secret registry poisoned")
            .remove(ssid);
    }
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.SecretAgent")]
impl SecretAgent {
    async fn get_secrets(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
        _connection_path: OwnedObjectPath,
        _flags: u32,
        _hints: Vec<String>,
    ) -> zbus::fdo::Result<HashMap<String, HashMap<String, OwnedValue>>> {
        let ssid = connection
            .get("802-11-wireless")
            .and_then(|settings| settings.get("ssid"))
            .and_then(|value| <Vec<u8>>::try_from(value.clone()).ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        let Some(secret) = self
            .secrets
            .lock()
            .expect("NetworkManager secret registry poisoned")
            .get(&ssid)
            .cloned()
        else {
            return Err(zbus::fdo::Error::Failed(
                "No secret is stored for this network".into(),
            ));
        };
        let mut wireless_security = HashMap::new();
        wireless_security.insert(
            "psk".to_string(),
            OwnedValue::try_from(Value::from(secret))
                .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?,
        );
        let mut result = HashMap::new();
        result.insert("802-11-wireless-security".to_string(), wireless_security);
        Ok(result)
    }

    async fn save_secrets(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
        _connection_path: OwnedObjectPath,
    ) -> zbus::fdo::Result<()> {
        let ssid = connection
            .get("802-11-wireless")
            .and_then(|settings| settings.get("ssid"))
            .and_then(|value| <Vec<u8>>::try_from(value.clone()).ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        let password = connection
            .get("802-11-wireless-security")
            .and_then(|settings| settings.get("psk"))
            .and_then(|value| <String>::try_from(value.clone()).ok());
        if let Some(password) = password {
            self.set_secret(ssid, password);
        }
        Ok(())
    }

    async fn delete_secrets(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
        _connection_path: OwnedObjectPath,
    ) -> zbus::fdo::Result<()> {
        let ssid = connection
            .get("802-11-wireless")
            .and_then(|settings| settings.get("ssid"))
            .and_then(|value| <Vec<u8>>::try_from(value.clone()).ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        self.forget(&ssid);
        Ok(())
    }

    async fn cancel_get_secrets(&self, _connection_path: OwnedObjectPath) -> zbus::fdo::Result<()> {
        Ok(())
    }
}
