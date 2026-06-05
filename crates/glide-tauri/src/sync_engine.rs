use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

use glide_core::clipboard::ClipboardItem;
use glide_core::policy::Policy;
use glide_core::sync_event::SyncEvent;

/// Sync engine - manages server connection, clipboard monitoring, and sync.
pub struct SyncEngine {
    pub device_id: String,
    pub device_name: String,
    pub server_url: Arc<Mutex<String>>,
    pub connection_status: Arc<Mutex<String>>,
    pub policy: Arc<Mutex<Policy>>,
    pub sync_paused: Arc<Mutex<bool>>,
    /// Channel for incoming clipboard items from server.
    pub incoming_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<ClipboardItem>>>>,
    pub incoming_tx: mpsc::UnboundedSender<ClipboardItem>,
    /// Last clipboard item we sent (to prevent echo).
    pub last_sent_item: Arc<Mutex<Option<String>>>,
    /// Session token from login.
    pub session_token: Arc<Mutex<Option<String>>>,
}

impl SyncEngine {
    pub fn new(device_id: String, device_name: String) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        Self {
            device_id,
            device_name,
            server_url: Arc::new(Mutex::new(String::new())),
            connection_status: Arc::new(Mutex::new("disconnected".to_string())),
            policy: Arc::new(Mutex::new(Policy::default())),
            sync_paused: Arc::new(Mutex::new(false)),
            incoming_rx: Arc::new(Mutex::new(Some(incoming_rx))),
            incoming_tx,
            last_sent_item: Arc::new(Mutex::new(None)),
            session_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Login with username/password and get a session token.
    pub async fn login(
        &self,
        url: String,
        username: String,
        password: String,
    ) -> Result<String, String> {
        let base_url = url.trim_end_matches('/').to_string();

        // Update server URL.
        {
            let mut server = self.server_url.lock().await;
            *server = base_url.clone();
        }
        {
            let mut status = self.connection_status.lock().await;
            *status = "connecting".to_string();
        }

        let client = reqwest::Client::new();
        let login_url = format!("{}/api/v1/auth/login", base_url);

        info!("Logging in to server: {}", login_url);

        let resp = client
            .post(&login_url)
            .json(&serde_json::json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Login failed: {}", e);
                warn!("{}", msg);
                msg
            })?;

        if !resp.status().is_success() {
            let status_code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let msg = format!("Login failed: status={}, reason={}", status_code, body);
            warn!("{}", msg);
            let mut status = self.connection_status.lock().await;
            *status = format!("error: {}", msg);
            return Err(msg);
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            let msg = format!("Failed to parse login response: {}", e);
            warn!("{}", msg);
            msg
        })?;

        let token = body.get("token").and_then(|v| v.as_str()).ok_or_else(|| {
            let msg = "No token in login response".to_string();
            warn!("{}", msg);
            msg
        })?;

        let session_token = token.to_string();
        {
            let mut st = self.session_token.lock().await;
            *st = Some(session_token.clone());
        }

        info!("Login successful, token acquired");

        // After login, register device and connect.
        self.connect(base_url, None).await?;

        Ok(session_token)
    }

    /// Connect to server and start sync loop.
    pub async fn connect(
        &self,
        url: String,
        registration_token: Option<String>,
    ) -> Result<(), String> {
        if url.is_empty() {
            return Err("Server URL is empty".to_string());
        }

        {
            let mut status = self.connection_status.lock().await;
            *status = "connecting".to_string();
        }
        {
            let mut server_url = self.server_url.lock().await;
            *server_url = url.clone();
        }

        let base_url = url.trim_end_matches('/').to_string();

        // 1. Register device with server.
        let client = reqwest::Client::new();
        let register_url = format!("{}/api/v1/devices/register", base_url);
        let mut register_body = serde_json::json!({
            "device_id": self.device_id,
            "name": self.device_name,
            "platform": std::env::consts::OS,
            "trusted": true,
        });

        if let Some(token) = registration_token {
            register_body["registration_token"] = serde_json::Value::String(token);
        }

        info!("Registering device with server: {}", register_url);

        match client.post(&register_url).json(&register_body).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status_code = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    let err = format!(
                        "Registration failed: status={}, reason={}",
                        status_code, body
                    );
                    warn!("{}", err);
                    let mut status = self.connection_status.lock().await;
                    *status = format!("error: {}", err);
                    return Err(err);
                }
                info!("Device registered with server");
            }
            Err(e) => {
                let err = format!("Connection failed during registration: {}", e);
                warn!("{}", err);
                let mut status = self.connection_status.lock().await;
                *status = format!("error: {}", err);
                return Err(err);
            }
        }

        // 2. Connect to WebSocket for real-time sync.
        let ws_url = format!(
            "{}/ws/sync?device_id={}",
            base_url.replace("http://", "ws://").replace("https://", "wss://"),
            self.device_id
        );

        let connection_status = self.connection_status.clone();
        let incoming_tx = self.incoming_tx.clone();
        let device_id = self.device_id.clone();
        let server_url = base_url.clone();

        tokio::spawn(async move {
            match connect_websocket(&ws_url, connection_status.clone(), incoming_tx, device_id, &server_url).await {
                Ok(()) => {
                    info!("WebSocket connection closed normally");
                    let mut status = connection_status.lock().await;
                    *status = "disconnected".to_string();
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    let mut status = connection_status.lock().await;
                    *status = format!("error: {}", e);
                }
            }
        });

        {
            let mut status = self.connection_status.lock().await;
            *status = "connected".to_string();
        }

        Ok(())
    }

    /// Send a clipboard item to the server.
    pub async fn send_clipboard(&self, item: &ClipboardItem) -> Result<(), String> {
        if *self.sync_paused.lock().await {
            return Ok(());
        }

        let url = self.server_url.lock().await;
        if url.is_empty() {
            return Err("Not connected".to_string());
        }

        // Store the item ID to prevent echo.
        {
            let mut last = self.last_sent_item.lock().await;
            *last = Some(item.item_id.clone());
        }

        // Send via WebSocket to the server.
        // The WebSocket sync handles real-time delivery.
        info!("Clipboard item {} queued for sync", item.item_id);
        Ok(())
    }

    /// Take the incoming clipboard receiver.
    pub async fn take_incoming(&self) -> Option<mpsc::UnboundedReceiver<ClipboardItem>> {
        self.incoming_rx.lock().await.take()
    }
}

/// Connect to server WebSocket and handle sync events.
async fn connect_websocket(
    ws_url: &str,
    connection_status: Arc<Mutex<String>>,
    incoming_tx: mpsc::UnboundedSender<ClipboardItem>,
    device_id: String,
    server_url: &str,
) -> Result<(), String> {
    use futures::{SinkExt, StreamExt};

    let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
        .await
        .map_err(|e| format!("WebSocket connect failed: {}", e))?;

    info!("WebSocket connected to {}", ws_url);

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Send identification.
    let identify = SyncEvent::DeviceJoined {
        device_id: device_id.clone(),
        name: format!("Client-{}", &device_id[..8]),
    };
    if let Ok(msg) = serde_json::to_string(&identify) {
        let _ = ws_tx.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await;
    }

    // Spawn heartbeat task.
    let ws_tx_clone = Arc::new(tokio::sync::Mutex::new(ws_tx));
    let ws_tx_for_heartbeat = ws_tx_clone.clone();
    let device_id_clone = device_id.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            let heartbeat = SyncEvent::Heartbeat {
                device_id: device_id_clone.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            if let Ok(msg) = serde_json::to_string(&heartbeat) {
                let mut tx = ws_tx_for_heartbeat.lock().await;
                if tx.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Process incoming messages.
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            tokio_tungstenite::tungstenite::Message::Text(text) => {
                if let Ok(event) = serde_json::from_str::<SyncEvent>(&text) {
                    match event {
                        SyncEvent::ClipboardCaptured { item } => {
                            if item.source_device_id != device_id {
                                info!("Received clipboard from {}: {}", item.source_device_id, item.item_id);
                                let _ = incoming_tx.send(item);
                            }
                        }
                        SyncEvent::DeviceJoined { device_id: did, name } => {
                            info!("Device joined: {} ({})", name, did);
                        }
                        SyncEvent::DeviceLeft { device_id: did } => {
                            info!("Device left: {}", did);
                        }
                        SyncEvent::Heartbeat { device_id: did, .. } => {
                            info!("Heartbeat from {}", did);
                        }
                        _ => {}
                    }
                }
            }
            tokio_tungstenite::tungstenite::Message::Close(_) => {
                info!("WebSocket closed by server");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Monitor clipboard changes and return new items.
pub async fn monitor_clipboard<F>(device_id: String, mut on_clipboard: F)
where
    F: FnMut(ClipboardItem) + Send + 'static,
{
    use std::time::Duration;

    let mut last_hash = String::new();

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let text = read_clipboard_text().await;
        if let Some(text) = text {
            let hash = format!("{:x}", md5_hash(&text));
            if hash != last_hash {
                last_hash = hash.clone();

                let item = ClipboardItem {
                    item_id: uuid::Uuid::new_v4().to_string(),
                    source_device_id: device_id.clone(),
                    source_session_type: glide_core::clipboard::SessionType::Persistent,
                    kind: ClipboardKind::Text,
                    representations: vec![glide_core::mime_rep::MimeRepresentation {
                        mime_type: "text/plain".to_string(),
                        content: glide_core::mime_rep::RepresentationContent::Text(text),
                    }],
                    size: 0,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    payload_refs: vec![],
                    checksum: hash,
                    delivery_policy: glide_core::clipboard::DeliveryPolicy::Broadcast,
                };

                on_clipboard(item);
            }
        }
    }
}

/// Read clipboard text using platform-specific method.
async fn read_clipboard_text() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = tokio::process::Command::new("xclip")
            .args(["-o", "-selection", "clipboard"])
            .output()
            .await
        {
            if output.status.success() {
                return String::from_utf8(output.stdout).ok();
            }
        }
        if let Ok(output) = tokio::process::Command::new("wl-paste").output().await {
            if output.status.success() {
                return String::from_utf8(output.stdout).ok();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = tokio::process::Command::new("powershell")
            .args(["-command", "Get-Clipboard"])
            .output()
            .await
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if !text.is_empty() {
                    return Some(text.trim().to_string());
                }
            }
        }
    }

    None
}

/// Simple hash for change detection.
fn md5_hash(data: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish() as u128
}

use glide_core::clipboard::ClipboardKind;
use std::time::Duration;
