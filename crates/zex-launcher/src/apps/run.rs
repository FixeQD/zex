//! Launching applications from parsed entries.

use super::model::AppInfo;
use super::session::session_env;
use std::process::Command;

/// Placeholder codes stripped from `Exec=` lines before launching
const FIELD_CODES: [&str; 11] = [
    "%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%k",
];

/// Default terminal wrapper for entries that request one.
pub const DEFAULT_TERMINAL_TEMPLATE: &str = "ghostty %command%";

/// Launch an application from its entry.
pub fn spawn_entry(app: &AppInfo, terminal_template: &str) -> anyhow::Result<()> {
    let command = strip_field_codes(&app.command);
    if command.is_empty() {
        anyhow::bail!("empty Exec line in {}", app.id);
    }
    if app.wants_terminal {
        let wrapped = if terminal_template.contains("%command%") {
            terminal_template.replace("%command%", &command)
        } else {
            format!("{terminal_template} {command}")
        };
        detach("sh", &["-c", &wrapped])
    } else {
        let mut words = command.split_whitespace();
        let bin = words.next().expect("command is not empty");
        let args: Vec<&str> = words.collect();
        detach(bin, &args)
    }
}

/// Remove freedesktop Exec field codes that need file or URL arguments
pub fn strip_field_codes(raw: &str) -> String {
    let mut command = raw.to_string();
    for code in FIELD_CODES {
        command = command.replace(code, "");
    }
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Spawn a detached process with the session environment
fn detach(bin: &str, args: &[&str]) -> anyhow::Result<()> {
    Command::new(bin)
        .args(args)
        .envs(session_env())
        .spawn()
        .map(|_| ())
        .map_err(Into::into)
}
