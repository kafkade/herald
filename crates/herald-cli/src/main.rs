mod api_client;
mod commands;
mod ui;
mod ws_client;

use clap::{Args, Parser, Subcommand};
use commands::watch::AnimationSpeed;

#[derive(Parser)]
#[command(name = "herald")]
#[command(about = "CLI client for the Herald split-flap message board")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Shared API connection arguments for admin commands.
#[derive(Args, Clone)]
struct ApiArgs {
    /// Herald server URL
    #[arg(long, default_value = "http://localhost:3000")]
    server: String,
    /// Bearer token for authentication
    #[arg(long, env = "HERALD_ADMIN_TOKEN")]
    token: String,
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
        /// Animation speed: fast, normal, slow, or off
        #[arg(long, default_value = "normal")]
        animation_speed: AnimationSpeed,
    },
    /// Push a message to the board
    Push {
        /// Message text to display
        text: String,
        /// Text alignment: left, center, or right
        #[arg(long, default_value = "center")]
        align: String,
        /// Expiry time in ISO-8601 format (e.g. "2025-12-31T23:59:59Z")
        #[arg(long)]
        expires: Option<String>,
        /// Message template (announcement, greeting, countdown, ticker)
        #[arg(long)]
        template: Option<String>,
        /// Preview the message on the board before pushing
        #[arg(long)]
        preview: bool,
        #[command(flatten)]
        api: ApiArgs,
    },
    /// Manage countdowns
    Countdown {
        #[command(subcommand)]
        command: CountdownCommand,
    },
    /// Manage the display queue
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },
    /// View or update server configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum QueueCommand {
    /// List the display queue
    List {
        #[command(flatten)]
        api: ApiArgs,
    },
    /// Reorder the display queue
    Reorder {
        /// Queue item IDs in desired order
        ids: Vec<String>,
        #[command(flatten)]
        api: ApiArgs,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Get configuration values
    Get {
        /// Specific key to look up (shows all if omitted)
        key: Option<String>,
        #[command(flatten)]
        api: ApiArgs,
    },
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// New value
        value: String,
        #[command(flatten)]
        api: ApiArgs,
    },
}

#[derive(Subcommand)]
enum CountdownCommand {
    /// Create a new countdown
    Create {
        /// Countdown label (e.g. "NEW YEAR")
        #[arg(long)]
        label: String,
        /// Target time in ISO-8601 format (e.g. "2025-12-31T00:00:00Z")
        #[arg(long)]
        target: String,
        /// Behavior when countdown reaches zero: show_zero, remove, or pause
        #[arg(long, default_value = "show_zero")]
        zero_behavior: String,
        #[command(flatten)]
        api: ApiArgs,
    },
    /// List all countdowns
    List {
        #[command(flatten)]
        api: ApiArgs,
    },
    /// Delete a countdown
    Delete {
        /// Countdown ID to delete
        id: String,
        #[command(flatten)]
        api: ApiArgs,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Watch {
            server,
            fps,
            animation_speed,
        } => commands::watch::run(server, fps, animation_speed).await,
        Command::Serve => {
            eprintln!("serve is not yet implemented");
            Ok(())
        }
        Command::Push {
            text,
            align,
            expires,
            template,
            preview,
            api,
        } => {
            commands::push::run(
                text, api.server, api.token, align, expires, template, preview,
            )
            .await
        }
        Command::Countdown { command } => match command {
            CountdownCommand::Create {
                label,
                target,
                zero_behavior,
                api,
            } => {
                commands::countdown::create(api.server, api.token, label, target, zero_behavior)
                    .await
            }
            CountdownCommand::List { api } => {
                commands::countdown::list(api.server, api.token).await
            }
            CountdownCommand::Delete { id, api } => {
                commands::countdown::delete(api.server, api.token, id).await
            }
        },
        Command::Queue { command } => match command {
            QueueCommand::List { api } => commands::queue::list(api.server, api.token).await,
            QueueCommand::Reorder { ids, api } => {
                commands::queue::reorder(api.server, api.token, ids).await
            }
        },
        Command::Config { command } => match command {
            ConfigCommand::Get { key, api } => {
                commands::config::get(api.server, api.token, key).await
            }
            ConfigCommand::Set { key, value, api } => {
                commands::config::set(api.server, api.token, key, value).await
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
