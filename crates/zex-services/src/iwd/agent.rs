//! iwd D-Bus authentication agent.

use std::sync::{Arc, Mutex};
use zbus::zvariant::OwnedObjectPath;

#[derive(Clone, Default)]
pub struct Agent {
    passphrase: Arc<Mutex<Option<String>>>,
}

impl Agent {
    pub fn set_passphrase(&self, passphrase: Option<String>) {
        *self.passphrase.lock().expect("iwd agent mutex poisoned") = passphrase;
    }

    pub fn clear(&self) {
        self.set_passphrase(None);
    }
}

#[zbus::interface(name = "net.connman.iwd.Agent")]
impl Agent {
    async fn release(&self) {}

    async fn request_passphrase(&self, _network: OwnedObjectPath) -> zbus::fdo::Result<String> {
        self.passphrase
            .lock()
            .expect("iwd agent mutex poisoned")
            .clone()
            .ok_or_else(|| zbus::fdo::Error::Failed("No passphrase supplied".into()))
    }

    async fn request_private_key_passphrase(
        &self,
        _network: OwnedObjectPath,
    ) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "No private key passphrase supplied".into(),
        ))
    }

    async fn request_user_name_and_password(
        &self,
        _network: OwnedObjectPath,
    ) -> zbus::fdo::Result<(String, String)> {
        Err(zbus::fdo::Error::Failed(
            "Interactive EAP credentials are not configured".into(),
        ))
    }

    async fn request_user_password(
        &self,
        _network: OwnedObjectPath,
        _user: String,
    ) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::Failed(
            "Interactive EAP password is not configured".into(),
        ))
    }

    fn cancel(&self, _reason: String) {}
}
