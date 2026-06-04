use serde::{Deserialize, Serialize};

use super::mime_rep::MimeRepresentation;
use super::payload::PayloadRef;

/// User-visible clipboard type categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardKind {
    /// Plain text, HTML/RTF rich text, URL, or color.
    Text,
    /// Raster image (PNG, JPEG, etc.).
    Image,
    /// A single file or a list of files/folders.
    File,
}

/// A clipboard item as it appears on the wire.
///
/// Each item may carry multiple MIME representations to preserve
/// paste fidelity across platforms (e.g. plain text + HTML for rich text).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    /// Unique identifier for this clipboard item.
    pub item_id: String,
    /// Device that originated this clipboard event.
    pub source_device_id: String,
    /// Type of session that created this item.
    pub source_session_type: SessionType,
    /// User-visible category.
    pub kind: ClipboardKind,
    /// One or more MIME representations of the content.
    pub representations: Vec<MimeRepresentation>,
    /// Total payload size in bytes.
    pub size: u64,
    /// Creation timestamp (epoch millis).
    pub created_at: i64,
    /// References to stored payloads (for file/image data).
    pub payload_refs: Vec<PayloadRef>,
    /// SHA-256 checksum of the primary representation.
    pub checksum: String,
    /// How this item should be delivered to other nodes.
    pub delivery_policy: DeliveryPolicy,
}

/// Session type that originated a clipboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    /// From a persistent trusted desktop node.
    Persistent,
    /// From a temporary CLI session.
    Temporary,
}

/// Delivery policy for clipboard items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryPolicy {
    /// Sync to all trusted nodes in the mesh.
    Broadcast,
    /// Sync only to specific device IDs.
    Targeted(Vec<String>),
    /// Do not sync (local-only clipboard event).
    LocalOnly,
}

impl Default for DeliveryPolicy {
    fn default() -> Self {
        Self::Broadcast
    }
}
