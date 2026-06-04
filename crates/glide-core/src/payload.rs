use serde::{Deserialize, Serialize};

/// Reference to a stored payload object on the server or in the local filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadRef {
    /// Unique identifier for the payload object.
    pub payload_id: String,
    /// File path or URL to retrieve the payload from.
    pub location: String,
    /// Expected size in bytes.
    pub size: u64,
    /// SHA-256 checksum for integrity verification.
    pub checksum: String,
}

impl PayloadRef {
    /// Create a new payload reference.
    pub fn new(payload_id: String, location: String, size: u64, checksum: String) -> Self {
        Self {
            payload_id,
            location,
            size,
            checksum,
        }
    }
}
