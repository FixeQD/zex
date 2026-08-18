//! Control socket server: routes incoming RPC calls into a request queue

use super::service::{Fault, Hit, Mode, ZexControl};
use futures::prelude::*;
use std::path::{Path, PathBuf};
use tarpc::context::Context;
use tarpc::server::{BaseChannel, Channel};
use tarpc::tokio_serde::formats::Json;
use tokio::net::UnixListener;
use tokio_util::codec::LengthDelimitedCodec;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Show(Option<Vec<Mode>>),
    Hide,
    Toggle(Option<Vec<Mode>>),
    Query { text: String, limit: u16 },
    Run(String),
    Quit,
    Reload,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Done,
    Hits(Vec<Hit>),
}

pub type Reply = tokio::sync::oneshot::Sender<Answer>;

#[derive(Debug)]
pub struct Demand {
    pub request: Request,
    pub reply: Reply,
}

#[derive(Clone)]
struct Control {
    queue: flume::Sender<Demand>,
}

impl ZexControl for Control {
    async fn show(self, _: Context, modes: Option<Vec<Mode>>) -> Result<(), Fault> {
        relay(&self.queue, Request::Show(modes)).await.map(|_| ())
    }

    async fn hide(self, _: Context) -> Result<(), Fault> {
        relay(&self.queue, Request::Hide).await.map(|_| ())
    }

    async fn toggle(self, _: Context, modes: Option<Vec<Mode>>) -> Result<(), Fault> {
        relay(&self.queue, Request::Toggle(modes)).await.map(|_| ())
    }

    async fn query(self, _: Context, text: String, limit: u16) -> Result<Vec<Hit>, Fault> {
        match relay(&self.queue, Request::Query { text, limit }).await? {
            Answer::Hits(hits) => Ok(hits),
            Answer::Done => Err(Fault::Rejected("query produced no answer".into())),
        }
    }

    async fn run(self, _: Context, name: String) -> Result<(), Fault> {
        relay(&self.queue, Request::Run(name)).await.map(|_| ())
    }

    async fn quit(self, _: Context) -> Result<(), Fault> {
        relay(&self.queue, Request::Quit).await.map(|_| ())
    }

    async fn reload(self, _: Context) -> Result<(), Fault> {
        relay(&self.queue, Request::Reload).await.map(|_| ())
    }
}

async fn relay(queue: &flume::Sender<Demand>, request: Request) -> Result<Answer, Fault> {
    let (reply, waiting) = tokio::sync::oneshot::channel();
    queue
        .send(Demand { request, reply })
        .map_err(|_| Fault::QueueClosed)?;
    waiting.await.map_err(|_| Fault::NoAnswer)
}

#[derive(Debug)]
pub struct Bridge {
    path: PathBuf,
    stop: CancellationToken,
}

impl Bridge {
    pub async fn start(queue: flume::Sender<Demand>) -> anyhow::Result<Self> {
        let path = super::socket_path();
        Self::start_at(queue, &path).await
    }

    pub async fn start_at(queue: flume::Sender<Demand>, path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if path.exists() {
            if super::is_listening(path) {
                anyhow::bail!("another control socket is live at {}", path.display());
            }
            std::fs::remove_file(path)?;
        }
        let path = path.to_path_buf();
        let listener = UnixListener::bind(&path)?;
        info!(socket = %path.display(), "control socket listening");
        let stop = CancellationToken::new();
        tokio::spawn(accept_loop(listener, queue, stop.clone()));
        Ok(Self { path, stop })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.cancel();
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!("cleaning up control socket failed: {e}"),
        }
    }
}

async fn accept_loop(
    listener: UnixListener,
    queue: flume::Sender<Demand>,
    stop: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = stop.cancelled() => break,
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(stream) => stream,
                    Err(e) => {
                        warn!("control socket accept failed: {e}");
                        continue;
                    }
                };
                let framed = tokio_util::codec::Framed::new(stream, LengthDelimitedCodec::new());
                let transport = tarpc::serde_transport::new(framed, Json::default());
                let channel = BaseChannel::with_defaults(transport);
                let control = Control {
                    queue: queue.clone(),
                };
                let serving = channel
                    .execute(control.serve())
                    .for_each(|reply| async move {
                        tokio::spawn(reply);
                    });
                tokio::spawn(async move {
                    serving.await;
                });
            }
        }
    }
}
