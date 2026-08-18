//! Assistant backend: an Ollama chat client with token streaming
mod wire;

use tracing::debug;

/// Endpoint and model selection for [`answer`] / [`stream`]
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// Base URL of the Ollama server, e.g. `http://localhost:11434`
    pub endpoint: String,
    /// Model served by that endpoint
    pub model: String,
    /// Sampling temperature
    pub temperature: f32,
    /// Maximum number of tokens in the reply
    pub max_tokens: u32,
    /// System prompt prepended to every conversation
    pub system_prompt: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: "Qwythos-9B-v2:latest".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            system_prompt: "You are a concise, helpful assistant.".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub(crate) fn tag(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    pub role: Role,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Unreachable(String),
    Denied(String),
    Malformed(String),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Unreachable(why) => write!(f, "assistant unreachable: {why}"),
            Fault::Denied(why) => write!(f, "assistant refused: {why}"),
            Fault::Malformed(why) => write!(f, "assistant reply malformed: {why}"),
        }
    }
}

impl std::error::Error for Fault {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Token(String),
    Done,
    Fault(Fault),
}

pub fn answer(profile: &Profile, turns: &[Turn]) -> Result<String, Fault> {
    debug!(model = %profile.model, "asking the assistant");
    let reply = wire::send(profile, turns, false)?
        .into_string()
        .map_err(|e| Fault::Unreachable(e.to_string()))?;
    let reply: wire::PlainReply =
        serde_json::from_str(&reply).map_err(|e| Fault::Malformed(format!("{e}: {reply}")))?;
    if let Some(why) = reply.error {
        return Err(Fault::Denied(why));
    }
    reply
        .message
        .and_then(|message| message.content)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| Fault::Malformed("missing message content".into()))
}

/// Start a streaming generation on a worker thread
pub fn stream(profile: &Profile, turns: Vec<Turn>) -> flume::Receiver<Event> {
    let (tx, rx) = flume::unbounded();
    let profile = profile.clone();
    std::thread::Builder::new()
        .name("zex-assistant".into())
        .spawn(move || {
            let outcome = wire::run_stream(&profile, turns, &tx);
            if let Err(fault) = outcome {
                let _ = tx.send(Event::Fault(fault));
            }
            let _ = tx.send(Event::Done);
        })
        .expect("spawning the assistant thread failed");
    rx
}
