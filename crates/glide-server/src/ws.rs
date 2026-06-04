use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{info, warn, error};

use glide_core::sync_event::SyncEvent;

use crate::state::ServerState;

/// Handle a WebSocket sync connection.
pub async fn handle_ws(
    socket: WebSocket,
    state: ServerState,
    device_id: String,
) {
    info!("WebSocket sync connection from device: {}", device_id);

    let (mut tx, mut rx) = socket.split();
    let mut event_rx = state.subscribe_events();

    // Spawn task to forward broadcast events to this client.
    let send_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let msg = match serde_json::to_string(&event) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to serialize sync event: {}", e);
                            continue;
                        }
                    };
                    if tx.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Sync event receiver lagged, dropping {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Receive messages from client.
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = rx.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<SyncEvent>(&text) {
                    Ok(event) => {
                        // Store clipboard items to database.
                        if let SyncEvent::ClipboardCaptured { ref item } = event {
                            info!("Storing clipboard item {} from {}", item.item_id, item.source_device_id);
                            store_clipboard_item(&state, item).await;
                        }
                        state.broadcast_event(event);
                    }
                    Err(e) => {
                        warn!("Failed to parse sync event: {} (raw: {})", e, &text[..text.len().min(100)]);
                    }
                }
            }
        }
    });

    // Wait for either task to finish.
    tokio::select! {
        _ = send_task => info!("Send task ended for device {}", device_id),
        _ = recv_task => info!("Recv task ended for device {}", device_id),
    }

    info!("WebSocket sync connection closed for device: {}", device_id);
}

async fn store_clipboard_item(state: &ServerState, item: &glide_core::clipboard::ClipboardItem) {
    let representations_json = serde_json::to_string(&item.representations).unwrap_or_else(|_| "[]".to_string());
    let delivery_policy_json = serde_json::to_string(&item.delivery_policy).unwrap_or_else(|_| r#"{"type":"broadcast"}"#.to_string());

    let kind = match item.kind {
        glide_core::clipboard::ClipboardKind::Text => "text",
        glide_core::clipboard::ClipboardKind::Image => "image",
        glide_core::clipboard::ClipboardKind::File => "file",
    };

    let session_type = match item.source_session_type {
        glide_core::clipboard::SessionType::Persistent => "persistent",
        glide_core::clipboard::SessionType::Temporary => "temporary",
    };

    if let Err(e) = sqlx::query(
        r#"INSERT OR REPLACE INTO clipboard_items
           (item_id, source_device_id, source_session_type, kind, representations, size, created_at, checksum, delivery_policy)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&item.item_id)
    .bind(&item.source_device_id)
    .bind(session_type)
    .bind(kind)
    .bind(representations_json)
    .bind(item.size as i64)
    .bind(item.created_at)
    .bind(&item.checksum)
    .bind(delivery_policy_json)
    .execute(&state.db)
    .await {
        error!("Failed to store clipboard item: {}", e);
    }
}
