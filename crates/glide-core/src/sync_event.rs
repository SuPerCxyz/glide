use serde::{Deserialize, Serialize};

use super::clipboard::ClipboardItem;

/// Events that flow through the sync layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum SyncEvent {
    /// A new clipboard item was captured on a device.
    ClipboardCaptured { item: ClipboardItem },
    /// Clipboard item was delivered to a target device.
    ClipboardDelivered {
        item_id: String,
        target_device_id: String,
    },
    /// A device joined the mesh.
    DeviceJoined { device_id: String, name: String },
    /// A device left the mesh.
    DeviceLeft { device_id: String },
    /// A device's LAN address was updated.
    DeviceAddressUpdated {
        device_id: String,
        lan_address: String,
    },
    /// Heartbeat from a device to signal liveness.
    Heartbeat { device_id: String, timestamp: i64 },
    /// Sync session established between two nodes.
    SyncSessionEstablished {
        local_device_id: String,
        remote_device_id: String,
    },
    /// Trust request from a peer (for LAN pairing).
    TrustRequest {
        device_id: String,
        device_name: String,
    },
    /// Trust request accepted by a peer.
    TrustAccept {
        device_id: String,
    },
    /// Screen saver state changed on a device.
    ScreenSaverChanged {
        device_id: String,
        active: bool,
        timestamp: i64,
    },
    /// Lock state changed on a device (screen locked/unlocked).
    LockStateChanged {
        device_id: String,
        locked: bool,
        timestamp: i64,
    },
    /// System is going to sleep / shutting down.
    SystemSleep {
        device_id: String,
        timestamp: i64,
    },
    /// System is waking up from sleep.
    SystemWake {
        device_id: String,
        timestamp: i64,
    },
}

impl SyncEvent {
    /// Get the item_id if this event is clipboard-related.
    pub fn clipboard_item_id(&self) -> Option<&str> {
        match self {
            SyncEvent::ClipboardCaptured { item } => Some(&item.item_id),
            SyncEvent::ClipboardDelivered { item_id, .. } => Some(item_id),
            _ => None,
        }
    }
}
