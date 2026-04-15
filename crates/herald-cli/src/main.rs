mod commands;

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
    Watch,
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
        Command::Watch => commands::watch::run().await,
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
