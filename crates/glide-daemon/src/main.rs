use glide_daemon::{DaemonSettings, DaemonState};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let daemon = DaemonState::new(DaemonSettings::default());

    if args.iter().any(|arg| arg == "--print-status") {
        println!("{}", serde_json::to_string_pretty(&daemon.status())?);
        return Ok(());
    }

    info!("glide-daemon skeleton started");
    println!("glide-daemon skeleton is running. Use --print-status for JSON status.");
    tokio::signal::ctrl_c().await?;
    info!("glide-daemon stopped");
    Ok(())
}
