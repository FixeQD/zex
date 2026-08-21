use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tarpc::client;
use tarpc::context;
use tarpc::tokio_serde::formats::Json;
use tokio::net::UnixStream;
use tokio_util::codec::LengthDelimitedCodec;
use zex_launcher::ipc::{Blocking, Hit, Mode, ZexControlClient};

#[derive(Parser)]
#[command(name = "zexctl", about = "Control the zex shell daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open a shell window
    OpenWindow { name: String },
    /// Toggle a shell window
    ToggleWindow { name: String },
    /// Close a shell window
    CloseWindow { name: String },
    /// Run a registered command
    Run { name: String },
    /// Lock the session
    Lock,
    /// Reload the shell (restart)
    Reload,
    /// Quit the shell
    Quit,
    /// Show the launcher (optionally with modes)
    Show {
        #[arg(long, value_delimiter = ',')]
        modes: Option<Vec<String>>,
    },
    /// Hide the launcher
    Hide,
    /// Toggle the launcher
    Toggle {
        #[arg(long, value_delimiter = ',')]
        modes: Option<Vec<String>>,
    },
    /// Query the launcher
    Query {
        text: String,
        #[arg(short, long, default_value = "10")]
        limit: u16,
    },
}

fn parse_modes(modes: Option<Vec<String>>) -> Option<Vec<Mode>> {
    modes.map(|m| {
        m.into_iter()
            .filter_map(|s| match s.to_lowercase().as_str() {
                "combined" => Some(Mode::Combined),
                "apps" => Some(Mode::Apps),
                "emojis" => Some(Mode::Emojis),
                "clipboard" => Some(Mode::Clipboard),
                "theme" => Some(Mode::Theme),
                "calculator" => Some(Mode::Calculator),
                "windows" => Some(Mode::Windows),
                "actions" => Some(Mode::Actions),
                _ => None,
            })
            .collect()
    })
}

async fn run_command(cli: Cli) -> Result<()> {
    let stream = UnixStream::connect(zex_launcher::ipc::socket_path())
        .await
        .context("connect to zex control socket")?;
    let framed = tokio_util::codec::Framed::new(stream, LengthDelimitedCodec::new());
    let transport = tarpc::serde_transport::new(framed, Json::default());
    let client = ZexControlClient::new(client::Config::default(), transport).spawn();

    match cli.command {
        Command::OpenWindow { name } => {
            let result = client.open_window(context::current(), name).await??;
            println!("{}", result);
        }
        Command::ToggleWindow { name } => {
            let result = client.toggle_window(context::current(), name).await??;
            println!("{}", result);
        }
        Command::CloseWindow { name } => {
            let result = client.close_window(context::current(), name).await??;
            println!("{}", result);
        }
        Command::Run { name } => {
            client.run(context::current(), name).await??;
            println!("command executed");
        }
        Command::Lock => {
            lock_session().await?;
            println!("locked");
        }
        Command::Reload => {
            client.reload(context::current()).await??;
            println!("reloaded");
        }
        Command::Quit => {
            client.quit(context::current()).await??;
            println!("quit");
        }
        Command::Show { modes } => {
            client.show(context::current(), parse_modes(modes)).await??;
        }
        Command::Hide => {
            client.hide(context::current()).await??;
        }
        Command::Toggle { modes } => {
            client.toggle(context::current(), parse_modes(modes)).await??;
        }
        Command::Query { text, limit } => {
            let hits = client.query(context::current(), text, limit).await??;
            for hit in hits {
                println!("{}: {} {:?}", hit.kind, hit.title, hit.note);
            }
        }
    }

    Ok(())
}

async fn lock_session() -> Result<()> {
    use zex_services::lockscreen::{LOCK_DESTINATION, LOCK_INTERFACE, LOCK_OBJECT_PATH};
    let conn = zbus::Connection::session().await.context("session bus unavailable")?;
    conn.call_method(
        Some(LOCK_DESTINATION),
        LOCK_OBJECT_PATH,
        Some(LOCK_INTERFACE),
        "Lock",
        &(),
    )
    .await
    .context("org.zex.Lock refused the lock request")?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;
    rt.block_on(run_command(cli))
}