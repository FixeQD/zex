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
    RunCommand { name: String },
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
        Command::RunCommand { name } => {
            println!("run-command {name}");
        }
    }

    Ok(())
}
