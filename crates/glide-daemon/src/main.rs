use glide_daemon::{DaemonSettings, DaemonState};
use tracing::info;

fn default_device_id() -> String {
    std::env::var("GLIDE_DEVICE_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn default_device_name() -> String {
    std::env::var("GLIDE_DEVICE_NAME")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| hostname())
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "glide-device".to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let daemon = DaemonState::new(DaemonSettings::default());

    if args.iter().any(|arg| arg == "--print-status") {
        println!("{}", serde_json::to_string_pretty(&daemon.status())?);
        return Ok(());
    }

    // Get device identity
    let device_id = default_device_id();
    let device_name = default_device_name();
    let lan_sync_port: u16 = std::env::var("GLIDE_LAN_SYNC_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9999);

    info!(
        "glide-daemon starting: device={} name={} lan_port={}",
        device_id, device_name, lan_sync_port
    );

    // Start LAN sync engine (UDP discovery + WebSocket clipboard sync)
    let sync_engine = glide_desktop::lan_sync::LanSyncEngine::new(
        device_id.clone(),
        device_name.clone(),
        lan_sync_port,
    );
    sync_engine.start().await?;
    info!("LAN sync engine started on port {}", lan_sync_port);

    // Start LAN input engine for keyboard/mouse relay (port +1)
    let input_engine = glide_desktop::lan_input::LanInputEngine::new(
        device_id,
        device_name,
        lan_sync_port + 1,
    );
    input_engine.start().await?;
    info!("LAN input engine started on port {}", lan_sync_port + 1);

    println!(
        "glide-daemon running. LAN sync on port {}, input relay on port {}. Press Ctrl+C to stop.",
        lan_sync_port,
        lan_sync_port + 1
    );
    tokio::signal::ctrl_c().await?;
    info!("glide-daemon stopped");
    Ok(())
}
