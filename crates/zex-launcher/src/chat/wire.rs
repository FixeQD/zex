use super::{Event, Fault, Profile, Role, Turn};
use std::io::{BufRead, BufReader};
use std::time::Duration;
use tracing::warn;

pub(crate) fn run_stream(
    profile: &Profile,
    turns: Vec<Turn>,
    tx: &flume::Sender<Event>,
) -> Result<(), Fault> {
    let reply = send(profile, &turns, true)?;
    let reader = BufReader::new(reply.into_reader());
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => return Err(Fault::Unreachable(e.to_string())),
        };
        if line.trim().is_empty() {
            continue;
        }
        let chunk: StreamChunk = match serde_json::from_str(&line) {
            Ok(chunk) => chunk,
            Err(e) => return Err(Fault::Malformed(format!("{e}: {line}"))),
        };
        if let Some(why) = chunk.error {
            return Err(Fault::Denied(why));
        }
        if let Some(content) = chunk.message.and_then(|message| message.content) {
            if !content.is_empty() && tx.send(Event::Token(content)).is_err() {
                break;
            }
        }
        if chunk.done {
            break;
        }
    }
    Ok(())
}

pub(crate) fn send(
    profile: &Profile,
    turns: &[Turn],
    streaming: bool,
) -> Result<ureq::Response, Fault> {
    agent()
        .post(&chat_url(profile))
        .send_json(request_body(profile, turns, streaming))
        .map_err(|e| upstream(e))
}

fn chat_url(profile: &Profile) -> String {
    format!("{}/api/chat", profile.endpoint.trim_end_matches('/'))
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .build()
}

fn request_body(profile: &Profile, turns: &[Turn], streaming: bool) -> serde_json::Value {
    let mut messages: Vec<serde_json::Value> = Vec::with_capacity(turns.len() + 1);
    if !profile.system_prompt.is_empty() {
        messages.push(serde_json::json!({
            "role": Role::System.tag(),
            "content": profile.system_prompt,
        }));
    }
    for turn in turns {
        messages.push(serde_json::json!({
            "role": turn.role.tag(),
            "content": turn.text,
        }));
    }
    serde_json::json!({
        "model": profile.model,
        "messages": messages,
        "stream": streaming,
        "options": {
            "temperature": profile.temperature,
            "num_predict": profile.max_tokens,
        },
    })
}

fn upstream(e: ureq::Error) -> Fault {
    match e {
        ureq::Error::Status(code, reply) => {
            let body = reply.into_string().unwrap_or_default();
            let why = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| value["error"].as_str().map(str::to_string))
                .unwrap_or_else(|| body.trim().to_string());
            warn!(code, %why, "assistant rejected the request");
            Fault::Denied(format!("HTTP {code}: {why}"))
        }
        other => Fault::Unreachable(other.to_string()),
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct PlainReply {
    pub(crate) message: Option<PlainMessage>,
    pub(crate) error: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct PlainMessage {
    pub(crate) content: Option<String>,
}

#[derive(serde::Deserialize)]
struct StreamChunk {
    #[serde(default)]
    message: Option<PlainMessage>,
    error: Option<String>,
    #[serde(default)]
    done: bool,
}
