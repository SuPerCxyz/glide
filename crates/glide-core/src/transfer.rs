use serde::{Deserialize, Serialize};

/// A transfer session tracks an active clipboard payload transfer between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Source device sending the payload.
    pub source_device_id: String,
    /// Target device receiving the payload.
    pub target_device_id: String,
    /// Clipboard item being transferred.
    pub item_id: String,
    /// Transfer route used.
    pub route: TransferRoute,
    /// Current state of the transfer.
    pub state: TransferState,
    /// Start timestamp (epoch millis).
    pub started_at: i64,
    /// Completion timestamp (epoch millis).
    pub completed_at: Option<i64>,
    /// Error message if transfer failed.
    pub error: Option<String>,
}

/// Route used for a clipboard transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferRoute {
    /// Direct LAN connection between source and target.
    LanDirect,
    /// LAN reverse pull: target pulls from source over LAN.
    LanReversePull,
    /// Server fallback: via the central server relay.
    ServerFallback,
}

/// State of a transfer session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    /// Transfer has been initiated.
    Initiated,
    /// Payload is being transferred.
    InProgress,
    /// Transfer completed successfully.
    Completed,
    /// Transfer failed.
    Failed,
}
