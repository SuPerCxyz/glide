use glide_core::device::Device;

use sqlx::{Pool, Sqlite};

/// Server-side database models.

/// Device record stored in the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub device_id: String,
    pub name: String,
    pub platform: String,
    pub trusted: bool,
    pub public_key_fingerprint: Option<String>,
    pub lan_address: Option<String>,
    pub last_seen_at: Option<i64>,
    pub created_at: i64,
}

impl From<DeviceRow> for Device {
    fn from(row: DeviceRow) -> Self {
        let platform = match row.platform.as_str() {
            "linux" => glide_core::device::Platform::Linux,
            "windows" => glide_core::device::Platform::Windows,
            "macos" => glide_core::device::Platform::MacOs,
            "android" => glide_core::device::Platform::Android,
            "ios" => glide_core::device::Platform::Ios,
            "web" => glide_core::device::Platform::Web,
            "cli" => glide_core::device::Platform::Cli,
            _ => glide_core::device::Platform::Linux,
        };

        Self {
            device_id: row.device_id.parse().unwrap_or_else(|_| uuid::Uuid::nil()),
            name: row.name,
            platform,
            trusted: row.trusted,
            public_key_fingerprint: row.public_key_fingerprint,
            lan_address: row.lan_address,
            last_seen_at: row.last_seen_at,
            created_at: row.created_at,
        }
    }
}

/// Clipboard item record.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClipboardItemRow {
    pub item_id: String,
    pub source_device_id: String,
    pub source_session_type: String,
    pub kind: String,
    pub representations: String, // JSON
    pub size: i64,
    pub created_at: i64,
    pub checksum: String,
    pub delivery_policy: String, // JSON
}

/// Temporary token record.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TempTokenRow {
    pub id: i64,
    pub token: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub ttl_secs: i64,
    pub max_uses: i64,
    pub use_count: i64,
    pub allowed_operations: String, // JSON array of strings
    pub max_item_size: i64,
    pub revoked: bool,
}

/// Payload object record.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PayloadRow {
    pub payload_id: String,
    pub file_path: String,
    pub size: i64,
    pub checksum: String,
    pub created_at: i64,
    pub ref_count: i64,
}

/// Input relay session record.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InputSessionRow {
    pub session_id: String,
    pub controller_id: String,
    pub target_id: String,
    pub route: String,
    pub active: bool,
    pub latency_ms: Option<i64>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

/// Cleanup metadata record.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CleanupRow {
    pub id: i64,
    pub last_run_at: i64,
    pub items_deleted: i64,
    pub bytes_freed: i64,
}
