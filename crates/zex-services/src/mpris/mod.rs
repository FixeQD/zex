//! MPRIS media player monitoring over the session bus

mod media;
mod monitor;
mod player;

pub use media::{MediaInfo, PlaybackStatus};
pub use player::MprisPlayer;

use anyhow::{Context, Result};
use flume::Receiver;
use tokio::task::JoinHandle;
use zbus::Connection;
use zbus::proxy::Builder;

pub const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub const PLAYER_OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";
pub const MEDIA_PLAYER2_INTERFACE: &str = "org.mpris.MediaPlayer2";
pub const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Debug, Clone, PartialEq)]
pub enum MprisEvent {
    PlayerAdded(MprisPlayer),
    PlayerRemoved(String),
    PlayerChanged(String, MediaInfo),
}

pub struct Mpris {
    conn: Connection,
    events: Receiver<MprisEvent>,
    _task: JoinHandle<()>,
}

impl Mpris {
    pub async fn connect(conn: Connection) -> Result<Self> {
        let (tx, rx) = flume::unbounded();
        let task = tokio::spawn(monitor::run(conn.clone(), tx));
        Ok(Self {
            conn,
            events: rx,
            _task: task,
        })
    }

    pub fn events(&self) -> &Receiver<MprisEvent> {
        &self.events
    }

    /// Fresh query of all running players
    pub async fn players(&self) -> Result<Vec<MprisPlayer>> {
        let names = player::bus_names(&self.conn).await?;
        let mut players = Vec::new();
        for name in names {
            if let Some(player) = player::fetch(&self.conn, &name).await? {
                players.push(player);
            }
        }
        Ok(players)
    }

    async fn player_proxy(&self, name: &str) -> Result<zbus::Proxy<'static>> {
        Builder::new(&self.conn)
            .destination(format!("{MPRIS_PREFIX}{name}"))?
            .path(PLAYER_OBJECT_PATH)?
            .interface(PLAYER_INTERFACE)?
            .build()
            .await
            .context("player proxy")
    }

    pub async fn play_pause(&self, name: &str) -> Result<()> {
        self.player_proxy(name)
            .await?
            .call_noreply("PlayPause", &())
            .await
            .context("PlayPause")
    }

    pub async fn play(&self, name: &str) -> Result<()> {
        self.player_proxy(name)
            .await?
            .call_noreply("Play", &())
            .await
            .context("Play")
    }

    pub async fn pause(&self, name: &str) -> Result<()> {
        self.player_proxy(name)
            .await?
            .call_noreply("Pause", &())
            .await
            .context("Pause")
    }

    pub async fn next(&self, name: &str) -> Result<()> {
        self.player_proxy(name)
            .await?
            .call_noreply("Next", &())
            .await
            .context("Next")
    }

    pub async fn previous(&self, name: &str) -> Result<()> {
        self.player_proxy(name)
            .await?
            .call_noreply("Previous", &())
            .await
            .context("Previous")
    }

    async fn root_proxy(&self, name: &str) -> Result<zbus::Proxy<'static>> {
        Builder::new(&self.conn)
            .destination(format!("{MPRIS_PREFIX}{name}"))?
            .path(PLAYER_OBJECT_PATH)?
            .interface(MEDIA_PLAYER2_INTERFACE)?
            .build()
            .await
            .context("media player2 proxy")
    }

    pub async fn raise(&self, name: &str) -> Result<()> {
        self.root_proxy(name)
            .await?
            .call_noreply("Raise", &())
            .await
            .context("raise")
    }

    pub async fn quit(&self, name: &str) -> Result<()> {
        self.root_proxy(name)
            .await?
            .call_noreply("Quit", &())
            .await
            .context("quit")
    }
}
