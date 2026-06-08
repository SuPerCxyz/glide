mod commands;
/// Glide CLI — headless clipboard tool.
///
/// Supports persistent mode (local config) and temporary single-use auth mode
/// (--server + --token without writing credentials).
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "glide", version, about = "LAN-first clipboard sync CLI")]
struct Cli {
    /// Server URL (overrides config).
    #[arg(long)]
    server: Option<String>,
    /// Temporary token for one-off authentication.
    #[arg(long)]
    token: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy content to the clipboard.
    Copy {
        /// Text to copy.
        text: Option<String>,
        /// File to copy.
        #[arg(long)]
        file: Option<String>,
        /// Directory to copy.
        #[arg(long)]
        dir: Option<String>,
        /// Image to copy.
        #[arg(long)]
        image: Option<String>,
    },
    /// Paste content from the clipboard.
    Paste {
        /// Output path for binary/file payloads.
        #[arg(long)]
        output: Option<String>,
    },
    /// View clipboard history.
    History {
        /// Number of items to show.
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// List registered devices.
    Devices,
    /// Initiate pairing and get a pairing code.
    Pair {
        /// Action: initiate or confirm
        action: String,
        /// Device ID (required for confirm)
        #[arg(long)]
        device_id: Option<String>,
        /// Pairing code (required for confirm)
        #[arg(long)]
        code: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("glide_cli=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    let client = commands::Client::new(cli.server.as_deref(), cli.token.as_deref()).await?;

    match cli.command {
        Commands::Copy {
            text,
            file,
            dir,
            image,
        } => commands::copy(&client, text, file, dir, image).await,
        Commands::Paste { output } => commands::paste(&client, output).await,
        Commands::History { limit } => commands::history(&client, limit).await,
        Commands::Devices => commands::devices(&client).await,
        Commands::Pair { action, device_id, code } => {
            match action.as_str() {
                "initiate" => commands::pair_initiate(&client).await,
                "confirm" => commands::pair_confirm(&client, code.as_deref(), device_id.as_deref()).await,
                _ => {
                    eprintln!("Usage: glide pair initiate | glide pair confirm --code <CODE> --device-id <DEVICE_ID>");
                    std::process::exit(1);
                }
            }
        },
    }
}
