mod commands;
mod ui;
mod ws_client;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "herald")]
#[command(about = "CLI client for the Herald split-flap message board")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the Herald server
    Serve,
    /// Watch the board in your terminal
    Watch {
        /// WebSocket server URL
        #[arg(long, default_value = "ws://localhost:3000/ws")]
        server: String,
        /// Target frames per second for the UI refresh rate
        #[arg(long, default_value_t = 30)]
        fps: u16,
    },
    /// Push a message to the board
    Push,
    /// Manage countdowns
    Countdown,
    /// Manage the display queue
    Queue,
    /// View or update server configuration
    Config,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Watch { server, fps } => commands::watch::run(server, fps).await,
        Command::Serve => {
            eprintln!("serve is not yet implemented");
            Ok(())
        }
        Command::Push => {
            eprintln!("push is not yet implemented");
            Ok(())
        }
        Command::Countdown => {
            eprintln!("countdown is not yet implemented");
            Ok(())
        }
        Command::Queue => {
            eprintln!("queue is not yet implemented");
            Ok(())
        }
        Command::Config => {
            eprintln!("config is not yet implemented");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
