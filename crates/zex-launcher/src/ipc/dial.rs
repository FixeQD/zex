//! Control socket client for tools that talk to a running daemon

use super::block_on;
use super::service::{Hit, Mode, ZexControlClient};
use tarpc::client;
use tarpc::context;
use tarpc::tokio_serde::formats::Json;
use tokio::net::UnixStream;
use tokio_util::codec::LengthDelimitedCodec;

/// Connected remote handle to the [daemon's socket](super::socket_path)
#[derive(Clone)]
pub struct Dial {
    inner: ZexControlClient,
}

impl Dial {
    pub async fn open() -> anyhow::Result<Self> {
        Self::open_at(&super::socket_path()).await
    }

    pub async fn open_at(path: &std::path::Path) -> anyhow::Result<Self> {
        let stream = UnixStream::connect(path).await?;
        let framed = tokio_util::codec::Framed::new(stream, LengthDelimitedCodec::new());
        let transport = tarpc::serde_transport::new(framed, Json::default());
        let inner = ZexControlClient::new(client::Config::default(), transport).spawn();
        Ok(Self { inner })
    }

    /// Connect from a plain thread using a throwaway runtime
    pub fn open_blocking() -> anyhow::Result<Self> {
        block_on(Self::open())
    }

    pub async fn show(&self, modes: Option<Vec<Mode>>) -> anyhow::Result<()> {
        Ok(self.inner.clone().show(context::current(), modes).await??)
    }

    pub async fn hide(&self) -> anyhow::Result<()> {
        Ok(self.inner.clone().hide(context::current()).await??)
    }

    pub async fn toggle(&self, modes: Option<Vec<Mode>>) -> anyhow::Result<()> {
        Ok(self
            .inner
            .clone()
            .toggle(context::current(), modes)
            .await??)
    }

    pub async fn query(&self, text: &str, limit: u16) -> anyhow::Result<Vec<Hit>> {
        Ok(self
            .inner
            .clone()
            .query(context::current(), text.to_string(), limit)
            .await??)
    }

    pub async fn run(&self, name: &str) -> anyhow::Result<()> {
        Ok(self
            .inner
            .clone()
            .run(context::current(), name.to_string())
            .await??)
    }

    pub async fn quit(&self) -> anyhow::Result<()> {
        Ok(self.inner.clone().quit(context::current()).await??)
    }

    pub async fn reload(&self) -> anyhow::Result<()> {
        Ok(self.inner.clone().reload(context::current()).await??)
    }

    // Window management
    pub async fn open_window(&self, name: &str) -> anyhow::Result<String> {
        Ok(self
            .inner
            .clone()
            .open_window(context::current(), name.to_string())
            .await??)
    }

    pub async fn toggle_window(&self, name: &str) -> anyhow::Result<String> {
        Ok(self
            .inner
            .clone()
            .toggle_window(context::current(), name.to_string())
            .await??)
    }

    pub async fn close_window(&self, name: &str) -> anyhow::Result<String> {
        Ok(self
            .inner
            .clone()
            .close_window(context::current(), name.to_string())
            .await??)
    }
}

/// Blocking flavour for CLIs and keybinding helpers
pub struct Blocking(pub Dial);

impl Blocking {
    pub fn open() -> anyhow::Result<Self> {
        Dial::open_blocking().map(Self)
    }

    pub fn show(&self, modes: Option<Vec<Mode>>) -> anyhow::Result<()> {
        block_on(self.0.show(modes))
    }

    pub fn hide(&self) -> anyhow::Result<()> {
        block_on(self.0.hide())
    }

    pub fn toggle(&self, modes: Option<Vec<Mode>>) -> anyhow::Result<()> {
        block_on(self.0.toggle(modes))
    }

    pub fn query(&self, text: &str, limit: u16) -> anyhow::Result<Vec<Hit>> {
        block_on(self.0.query(text, limit))
    }

    pub fn run(&self, name: &str) -> anyhow::Result<()> {
        block_on(self.0.run(name))
    }

    pub fn quit(&self) -> anyhow::Result<()> {
        block_on(self.0.quit())
    }

    pub fn reload(&self) -> anyhow::Result<()> {
        block_on(self.0.reload())
    }

    // Window management (blocking)
    pub fn open_window(&self, name: &str) -> anyhow::Result<String> {
        block_on(self.0.open_window(name))
    }

    pub fn toggle_window(&self, name: &str) -> anyhow::Result<String> {
        block_on(self.0.toggle_window(name))
    }

    pub fn close_window(&self, name: &str) -> anyhow::Result<String> {
        block_on(self.0.close_window(name))
    }
}
