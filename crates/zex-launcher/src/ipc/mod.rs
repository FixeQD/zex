//! Control socket: where the daemon listens and how tools reach it

pub mod bridge;
pub mod dial;
pub mod service;

pub use bridge::{Answer, Bridge, Demand, Reply, Request};
pub use dial::{Blocking, Dial};
pub use service::{Fault, Hit, Mode, ZexControl, ZexControlClient};

use std::path::PathBuf;

pub fn socket_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("zex").join("ipc.sock")
}

pub fn is_listening(path: &std::path::Path) -> bool {
    std::os::unix::net::UnixStream::connect(path).is_ok()
}

/// Run a future on a throwaway single-threaded runtime
pub fn block_on<F, T>(future: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(future)
}
