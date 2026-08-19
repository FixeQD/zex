//! Lock service: exports `org.zex.Lock` on the session bus so tools like
//! `zexctl lock` can request a session lock

use anyhow::{Context, Result};
use flume::Sender;
use tokio::task::JoinHandle;
use zbus::{Connection, interface};

pub const LOCK_DESTINATION: &str = "org.zex.Lock";
pub const LOCK_OBJECT_PATH: &str = "/org/zex/Lock";
pub const LOCK_INTERFACE: &str = "org.zex.Lock";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockEvent {
    Locked,
}

#[derive(Debug, Clone)]
struct LockState {
    events: Sender<LockEvent>,
}

#[interface(name = "org.zex.Lock")]
impl LockState {
    /// Lock the session
    async fn lock(&self) -> zbus::fdo::Result<()> {
        self.events
            .send(LockEvent::Locked)
            .map_err(|_| zbus::fdo::Error::Failed("lock channel closed".to_string()))
    }
}

pub struct Lockscreen {
    _task: JoinHandle<()>,
}

impl Lockscreen {
    pub async fn connect(conn: Connection, events: Sender<LockEvent>) -> Result<Self> {
        conn.object_server()
            .at(LOCK_OBJECT_PATH, LockState { events })
            .await
            .context("exporting lock object")?;
        conn.request_name(LOCK_DESTINATION)
            .await
            .context("requesting org.zex.Lock name")?;
        let task = tokio::spawn(async move {
            let _conn = conn; // keep the registration alive
            std::future::pending::<()>().await
        });
        Ok(Self { _task: task })
    }
}
