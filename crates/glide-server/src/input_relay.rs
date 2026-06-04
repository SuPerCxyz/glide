use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use glide_core::input_event::{InputEvent, InputEventKind};

use crate::state::ServerState;

/// Handle an input relay WebSocket connection.
pub async fn handle_input_ws(
    socket: WebSocket,
    _state: ServerState,
    device_id: String,
    target_id: String,
) {
    // Check if input relay is enabled.
    let enabled = std::env::var("GLIDE_INPUT_RELAY_ENABLED")
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if !enabled {
        warn!("Input relay connection attempt but relay is disabled");
        return;
    }

    info!(
        "Input relay connection: controller={} target={}",
        device_id, target_id
    );

    let (mut tx, mut rx) = socket.split();
    let (tx_ch, mut rx_ch) = mpsc::unbounded_channel::<Message>();

    // Spawn task to forward queued messages to the client.
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx_ch.recv().await {
            if tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Receive messages from client.
    let mut last_heartbeat = Instant::now();

    while let Some(Ok(msg)) = rx.next().await {
        if let Message::Text(text) = &msg {
            // Check heartbeat timeout.
            if last_heartbeat.elapsed() > Duration::from_secs(30) {
                warn!("Heartbeat timeout for input session");
                break;
            }

            if let Ok(event) = serde_json::from_str::<InputEvent>(text) {
                match &event.event {
                    InputEventKind::EmergencyRelease => {
                        info!("Emergency release received for device {}", device_id);
                        break;
                    }
                    _ => {
                        // In a full implementation, this would relay to the target device.
                        let _ = tx_ch.send(Message::Text(format!(
                            "relayed: {}", text.chars().take(50).collect::<String>()
                        )));
                    }
                }
            } else if text == "heartbeat" {
                last_heartbeat = Instant::now();
                let _ = tx_ch.send(Message::Text("heartbeat_ack".to_string()));

                let latency_ms = last_heartbeat.elapsed().as_millis() as u64;
                let max_latency = std::env::var("GLIDE_INPUT_RELAY_MAX_LATENCY_MS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(200);

                if latency_ms > max_latency {
                    warn!(
                        "Input relay latency high: {}ms (max: {}ms)",
                        latency_ms, max_latency
                    );
                }
            }
        }

        if send_task.is_finished() {
            break;
        }
    }

    info!("Input relay connection closed: controller={}", device_id);
}
