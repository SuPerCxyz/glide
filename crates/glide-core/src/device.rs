use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a device in the Glide mesh.
pub type DeviceId = Uuid;

/// Represents a registered device in the Glide system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier.
    pub device_id: DeviceId,
    /// Human-readable name for the device.
    pub name: String,
    /// Platform type of the device.
    pub platform: Platform,
    /// Whether the device is part of the trusted mesh.
    pub trusted: bool,
    /// Public key or certificate fingerprint for TLS pinning.
    pub public_key_fingerprint: Option<String>,
    /// LAN address for direct transfer.
    pub lan_address: Option<String>,
    /// Last seen timestamp (epoch millis).
    pub last_seen_at: Option<i64>,
    /// When the device was registered.
    pub created_at: i64,
}

/// Platform type of a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    Windows,
    MacOs,
    Android,
    Ios,
    Web,
    Cli,
}

/// Registration type for a device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationType {
    /// Persistent trusted device with stored credentials.
    Persistent,
    /// Temporary CLI session, not part of the trusted mesh.
    Temporary {
        /// Token used for authentication.
        token: String,
        /// TTL in seconds.
        ttl_secs: u64,
    },
}
