use anyhow::Context;
use clap::Parser;

#[derive(Parser)]
#[command(name = "zexctl", about = "Control the zex shell daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Open a shell window
    OpenWindow { name: String },
    /// Toggle a shell window
    ToggleWindow { name: String },
    /// Run a registered command
    Run { name: String },
    /// Lock the session through org.zex.Lock
    Lock,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::OpenWindow { name } => {
            println!("open-window {name}");
        }
        Command::ToggleWindow { name } => {
            println!("toggle-window {name}");
        }
        Command::Run { name } => {
            println!("run-command {name}");
        }
        Command::Lock => lock_session()?,
    }

    Ok(())
}

/// Ask the session bus lock service to lock
/// Fails when the shell is not running or has not exported `org.zex.Lock` yet
fn lock_session() -> anyhow::Result<()> {
    use zex_services::lockscreen::{LOCK_DESTINATION, LOCK_INTERFACE, LOCK_OBJECT_PATH};
    let conn = zbus::blocking::Connection::session().context("session bus unavailable")?;
    conn.call_method(
        Some(LOCK_DESTINATION),
        LOCK_OBJECT_PATH,
        Some(LOCK_INTERFACE),
        "Lock",
        &(),
    )
    .context("org.zex.Lock refused the lock request")?;
    println!("locked");
    Ok(())
}
