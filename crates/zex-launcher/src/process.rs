//! Detached process helpers: fire and forget, outliving the launcher.

use crate::apps::session_env;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

/// Spawn `program` detached from the session, with the captured environment and all stdio silenced
pub fn fire(program: &str, args: &[&str]) -> anyhow::Result<()> {
    if program.trim().is_empty() {
        anyhow::bail!("empty program");
    }
    let mut command = Command::new(program);
    command.args(args);
    detach(&mut command)
}

/// Run `line` through `sh -c` in a detached process
pub fn shell(line: &str) -> anyhow::Result<()> {
    if line.trim().is_empty() {
        anyhow::bail!("empty shell command");
    }
    fire("sh", &["-c", line])
}

/// Hand `target` to the desktop's default opener
pub fn open(target: &str) -> anyhow::Result<()> {
    fire("xdg-open", &[target])
}

/// Run `command` inside the terminal described by `template`
pub fn in_terminal(template: &str, command: &str) -> anyhow::Result<()> {
    let wrapped = if template.contains("%command%") {
        template.replace("%command%", command)
    } else {
        format!("{template} {command}")
    };
    shell(&wrapped)
}

fn detach(command: &mut Command) -> anyhow::Result<()> {
    command
        .envs(session_env())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    command.spawn().map(|_| ()).map_err(Into::into)
}
