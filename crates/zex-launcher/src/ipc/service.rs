//! Wire contract of the zex control socket.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Combined,
    Apps,
    Emojis,
    Clipboard,
    Theme,
    Calculator,
    Windows,
    Actions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub kind: String,
    pub title: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fault {
    QueueClosed,
    NoAnswer,
    Rejected(String),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::QueueClosed => write!(f, "control queue closed"),
            Fault::NoAnswer => write!(f, "control call went unanswered"),
            Fault::Rejected(why) => write!(f, "rejected: {why}"),
        }
    }
}

impl std::error::Error for Fault {}

#[tarpc::service]
pub trait ZexControl {
    async fn show(modes: Option<Vec<Mode>>) -> Result<(), Fault>;

    async fn hide() -> Result<(), Fault>;

    async fn toggle(modes: Option<Vec<Mode>>) -> Result<(), Fault>;

    async fn query(text: String, limit: u16) -> Result<Vec<Hit>, Fault>;

    async fn run(name: String) -> Result<(), Fault>;

    async fn quit() -> Result<(), Fault>;

    async fn reload() -> Result<(), Fault>;

    // Window management
    async fn open_window(name: String) -> Result<String, Fault>;

    async fn toggle_window(name: String) -> Result<String, Fault>;

    async fn close_window(name: String) -> Result<String, Fault>;
}
