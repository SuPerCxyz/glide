use std::collections::HashMap;
use std::sync::Arc;

use sqlx::{Pool, Sqlite};
use tokio::sync::{broadcast, RwLock};

use glide_core::sync_event::SyncEvent;

/// Shared server state.
#[derive(Clone)]
pub struct ServerState {
    pub db: Pool<Sqlite>,
    pub data_dir: String,
    /// Broadcast channel for sync events (capacity: 256).
    pub event_tx: broadcast::Sender<SyncEvent>,
    /// Session store: maps session token -> authenticated (bool).
    pub sessions: Arc<RwLock<HashMap<String, bool>>>,
}

impl ServerState {
    pub fn new(db: Pool<Sqlite>, data_dir: String) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            db,
            data_dir,
            event_tx,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to sync events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<SyncEvent> {
        self.event_tx.subscribe()
    }

    /// Broadcast a sync event to all WebSocket listeners.
    pub fn broadcast_event(&self, event: SyncEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Get the payload storage directory.
    pub fn payload_dir(&self) -> String {
        format!("{}/payloads", self.data_dir)
    }
}
