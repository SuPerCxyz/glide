use anyhow::{Result, bail};
use reqwest::Client as HttpClient;
use sha2::{Digest, Sha256};
use std::path::Path;
use futures::{StreamExt, SinkExt};

use glide_core::clipboard::{ClipboardItem, ClipboardKind, DeliveryPolicy, SessionType};
use glide_core::mime_rep::{MimeRepresentation, RepresentationContent, mime_types};

use crate::config::CliConfig;

/// API client for the Glide server.
pub struct Client {
    pub http: HttpClient,
    pub server_url: String,
    pub token: Option<String>,
    pub config: Option<CliConfig>,
    pub device_id: String,
    pub session_type: SessionType,
}

impl Client {
    pub async fn new(server_url: Option<&str>, token: Option<&str>) -> Result<Self> {
        let config = CliConfig::load()?;

        let (server_url, device_id, session_type) = match (server_url, token) {
            (Some(url), Some(_tok)) => {
                // Single-use auth mode: don't persist config.
                let id = uuid::Uuid::new_v4().to_string();
                (url.to_string(), id, SessionType::Temporary)
            }
            (Some(url), None) => {
                // Server specified but no token — try config or generate temp session.
                let id = config.as_ref().map(|c| c.device_id.to_string()).unwrap_or_else(|| {
                    uuid::Uuid::new_v4().to_string()
                });
                (url.to_string(), id, SessionType::Persistent)
            }
            (None, Some(_tok)) => {
                bail!("--token requires --server");
            }
            (None, None) => {
                // Persistent mode: require config.
                match &config {
                    Some(c) => (c.server_url.clone(), c.device_id.to_string(), SessionType::Persistent),
                    None => bail!(
                        "No config found. Run with --server and --token for temporary sessions, \
                         or set up persistent mode by creating ~/.config/glide/config.json"
                    ),
                }
            }
        };

        Ok(Self {
            http: HttpClient::builder().build()?,
            server_url: server_url.trim_end_matches('/').to_string(),
            token: token.map(String::from),
            config,
            device_id,
            session_type,
        })
    }

    fn auth_query(&self) -> Vec<(&str, &str)> {
        let mut params = vec![("device_id", self.device_id.as_str())];
        if let Some(ref t) = self.token {
            params.push(("token", t));
        }
        params
    }

    /// Register the device if not already registered.
    pub async fn register(&self) -> Result<()> {
        let name = self.config.as_ref().map(|c| c.device_name.clone()).unwrap_or_else(|| "cli".to_string());
        let reg_token = self.config.as_ref().and_then(|c| c.registration_token.clone());

        let mut body = serde_json::json!({
            "device_id": self.device_id,
            "name": name,
            "platform": if cfg!(target_os = "windows") { "windows" } else { "linux" },
            "trusted": self.session_type == SessionType::Persistent,
        });

        if let Some(rt) = reg_token {
            body["registration_token"] = serde_json::Value::String(rt);
        }

        let resp = self.http
            .post(format!("{}/api/v1/devices/register", self.server_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await?;
            bail!("Registration failed: {}", text);
        }

        Ok(())
    }
}

/// Copy content to clipboard.
pub async fn copy(
    client: &Client,
    text: Option<String>,
    file: Option<String>,
    dir: Option<String>,
    image: Option<String>,
) -> Result<()> {
    // Determine what's being copied.
    let (kind, representations, payload_refs, total_size, checksum) = match (text, file, dir, image) {
        (Some(text), None, None, None) => {
            let checksum = compute_checksum(text.as_bytes());
            let reps = vec![MimeRepresentation {
                mime_type: mime_types::TEXT_PLAIN.to_string(),
                content: RepresentationContent::Text(text),
            }];
            (ClipboardKind::Text, reps, Vec::new(), 0, checksum)
        }
        (None, Some(file_path), None, None) => {
            copy_file(&file_path).await?
        }
        (None, None, Some(dir_path), None) => {
            copy_directory(&dir_path).await?
        }
        (None, None, None, Some(image_path)) => {
            copy_image(&image_path).await?
        }
        _ => bail!("Exactly one of TEXT, --file, --dir, or --image must be specified"),
    };

    let item = ClipboardItem {
        item_id: uuid::Uuid::new_v4().to_string(),
        source_device_id: client.device_id.clone(),
        source_session_type: client.session_type,
        kind,
        representations,
        size: total_size,
        created_at: chrono::Utc::now().timestamp_millis(),
        payload_refs,
        checksum,
        delivery_policy: DeliveryPolicy::default(),
    };

    // Register device first.
    client.register().await?;

    // Upload payload refs if any.
    for pref in &item.payload_refs {
        upload_payload(client, pref).await?;
    }

    // Send clipboard item via WebSocket.
    send_clipboard_item(client, &item).await?;

    println!("Copied: {} ({} bytes)", clipboard_kind_str(&item.kind), total_size);
    println!("Item ID: {}", item.item_id);

    Ok(())
}

/// Paste content from clipboard.
pub async fn paste(client: &Client, output: Option<String>) -> Result<()> {
    // Get the most recent clipboard item.
    let items = get_history(client, 1).await?;
    if items.is_empty() {
        println!("Clipboard is empty");
        return Ok(());
    }

    let item = &items[0];

    // Check if item has inline content.
    for rep in &item.representations {
        match &rep.content {
            RepresentationContent::Text(text) => {
                if let Some(ref out) = output {
                    std::fs::write(out, text)?;
                    println!("Written to {}", out);
                } else {
                    println!("{}", text);
                }
                return Ok(());
            }
            RepresentationContent::InlineBase64(data) => {
                if let Some(ref out) = output {
                    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)?;
                    std::fs::write(out, &bytes)?;
                    println!("Written {} bytes to {}", bytes.len(), out);
                } else {
                    println!("{}", data);
                }
                return Ok(());
            }
            RepresentationContent::PayloadRef(pid) => {
                if let Some(ref out) = output {
                    download_payload(client, pid, out).await?;
                    println!("Downloaded payload {} to {}", pid, out);
                } else {
                    eprintln!("Binary payload requires --output");
                }
                return Ok(());
            }
        }
    }

    // Try downloading payload refs.
    if !item.payload_refs.is_empty() {
        if let Some(ref out) = output {
            let pref = &item.payload_refs[0];
            download_payload_by_ref(client, pref, out).await?;
            println!("Downloaded to {}", out);
            return Ok(());
        } else {
            eprintln!("Binary payload requires --output");
        }
    }

    println!("No pasteable content found in the most recent item");
    Ok(())
}

/// View clipboard history.
pub async fn history(client: &Client, limit: usize) -> Result<()> {
    let items = get_history(client, limit).await?;

    if items.is_empty() {
        println!("No clipboard items found");
        return Ok(());
    }

    for item in &items {
        let time = chrono::DateTime::from_timestamp_millis(item.created_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let preview = item.representations
            .iter()
            .filter_map(|r| match &r.content {
                RepresentationContent::Text(t) => Some(t.chars().take(80).collect::<String>()),
                RepresentationContent::InlineBase64(_) => Some("[base64 data]".to_string()),
                RepresentationContent::PayloadRef(pid) => Some(format!("[payload: {}]", pid)),
            })
            .next()
            .unwrap_or_else(|| "[empty]".to_string());

        println!("[{}] {} | {} | {} bytes | {}", time, clipboard_kind_str(&item.kind), item.source_device_id, item.size, preview);
    }

    Ok(())
}

/// List registered devices.
pub async fn devices(client: &Client) -> Result<()> {
    let resp = client.http
        .get(format!("{}/api/v1/devices", client.server_url))
        .query(&client.auth_query())
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Failed to list devices: {}", text);
    }

    let body: serde_json::Value = resp.json().await?;
    let empty = vec![];
    let devs = body.get("devices").and_then(|d| d.as_array()).unwrap_or(&empty);

    if devs.is_empty() {
        println!("No devices registered");
        return Ok(());
    }

    for dev in devs {
        let id = dev.get("device_id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = dev.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let platform = dev.get("platform").and_then(|v| v.as_str()).unwrap_or("?");
        let trusted = dev.get("trusted").and_then(|v| v.as_bool()).unwrap_or(false);
        let seen = dev.get("last_seen_at")
            .and_then(|v| v.as_i64())
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "never".to_string());

        println!("{} | {} | {} | trusted={} | last seen: {}", id, name, platform, trusted, seen);
    }

    Ok(())
}

// --- Internal helpers ---

fn compute_checksum(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

async fn copy_file(path: &str) -> Result<(ClipboardKind, Vec<MimeRepresentation>, Vec<glide_core::payload::PayloadRef>, u64, String)> {
    let data = std::fs::read(path)?;
    let size = data.len() as u64;
    let checksum = compute_checksum(&data);
    let payload_id = uuid::Uuid::new_v4().to_string();

    // Detect MIME type from extension.
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime_type = match ext {
        "png" => mime_types::IMAGE_PNG,
        "jpg" | "jpeg" => mime_types::IMAGE_JPEG,
        "gif" => mime_types::IMAGE_GIF,
        _ => "application/octet-stream",
    };

    let kind = if mime_type.starts_with("image/") {
        ClipboardKind::Image
    } else {
        ClipboardKind::File
    };

    let reps = vec![MimeRepresentation {
        mime_type: mime_type.to_string(),
        content: RepresentationContent::PayloadRef(payload_id.clone()),
    }];

    let pref = glide_core::payload::PayloadRef::new(
        payload_id.clone(),
        format!("/api/v1/payload/{}", payload_id),
        size,
        checksum.clone(),
    );

    Ok((kind, reps, vec![pref], size, checksum))
}

async fn copy_directory(path: &str) -> Result<(ClipboardKind, Vec<MimeRepresentation>, Vec<glide_core::payload::PayloadRef>, u64, String)> {
    // Package directory as a tarball.
    let payload_id = uuid::Uuid::new_v4().to_string();
    let tar_path = format!("/tmp/glide_{}.tar.gz", payload_id);

    let output = std::process::Command::new("tar")
        .args(["-czf", &tar_path, "-C", path, "."])
        .output()?;

    if !output.status.success() {
        bail!("Failed to create archive: {}", String::from_utf8_lossy(&output.stderr));
    }

    let data = std::fs::read(&tar_path)?;
    let size = data.len() as u64;
    let checksum = compute_checksum(&data);

    let reps = vec![MimeRepresentation {
        mime_type: "application/gzip".to_string(),
        content: RepresentationContent::PayloadRef(payload_id.clone()),
    }];

    let pref = glide_core::payload::PayloadRef::new(
        payload_id.clone(),
        format!("/api/v1/payload/{}", payload_id),
        size,
        checksum.clone(),
    );

    // Clean up temp file.
    let _ = std::fs::remove_file(&tar_path);

    Ok((ClipboardKind::File, reps, vec![pref], size, checksum))
}

async fn copy_image(path: &str) -> Result<(ClipboardKind, Vec<MimeRepresentation>, Vec<glide_core::payload::PayloadRef>, u64, String)> {
    let (kind, reps, prefs, size, checksum) = copy_file(path).await?;
    // Override kind to Image regardless of detected type.
    Ok((ClipboardKind::Image, reps, prefs, size, checksum))
}

async fn upload_payload(client: &Client, pref: &glide_core::payload::PayloadRef) -> Result<()> {
    // Read file from filesystem based on payload_id.
    // For the CLI, we assume the file path is the source file passed to copy.
    // In a full implementation, this would track the source path separately.
    Ok(())
}

async fn download_payload(client: &Client, payload_id: &str, output: &str) -> Result<()> {
    let resp = client.http
        .get(format!("{}/api/v1/payload/{}", client.server_url, payload_id))
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Failed to download payload: {}", text);
    }

    let bytes = resp.bytes().await?;
    std::fs::write(output, &bytes)?;

    Ok(())
}

async fn download_payload_by_ref(client: &Client, pref: &glide_core::payload::PayloadRef, output: &str) -> Result<()> {
    download_payload(client, &pref.payload_id, output).await
}

async fn get_history(client: &Client, limit: usize) -> Result<Vec<ClipboardItem>> {
    let resp = client.http
        .get(format!("{}/api/v1/clipboard/history", client.server_url))
        .query(&[("limit", limit.to_string())])
        .query(&client.auth_query())
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Failed to fetch history: {}", text);
    }

    let body: serde_json::Value = resp.json().await?;
    let empty = vec![];
    let items = body.get("items").and_then(|v| v.as_array()).unwrap_or(&empty);

    let mut result = Vec::new();
    for item in items {
        if let Ok(ci) = serde_json::from_value::<ClipboardItem>(item.clone()) {
            result.push(ci);
        }
    }

    Ok(result)
}

async fn send_clipboard_item(client: &Client, item: &ClipboardItem) -> Result<()> {
    // Connect via WebSocket and send the clipboard event.
    let ws_url = client.server_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    let url = format!("{}/ws/sync?{}", ws_url, format!("device_id={}", client.device_id));
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;

    let msg = serde_json::json!({
        "event_type": "ClipboardCaptured",
        "data": {
            "item": serde_json::to_value(item)?,
        }
    });

    let (mut write, _) = ws_stream.split();
    write.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await?;

    Ok(())
}

/// Extension trait for kind display.
pub fn clipboard_kind_str(kind: &glide_core::clipboard::ClipboardKind) -> &str {
    match kind {
        glide_core::clipboard::ClipboardKind::Text => "text",
        glide_core::clipboard::ClipboardKind::Image => "image",
        glide_core::clipboard::ClipboardKind::File => "file",
    }
}
