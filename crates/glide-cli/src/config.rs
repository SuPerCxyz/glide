use anyhow::Result;
use serde::{Deserialize, Serialize};

use glide_core::device::DeviceId;

/// CLI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Server URL.
    pub server_url: String,
    /// Persistent device ID.
    pub device_id: DeviceId,
    /// Device name.
    pub device_name: String,
    /// Registration token (only used during initial registration).
    pub registration_token: Option<String>,
}

impl CliConfig {
    /// Load config from the default path.
    pub fn load() -> Result<Option<Self>> {
        let path = config_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&data)?;
        Ok(Some(config))
    }

    /// Save config to the default path.
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}

/// Get the default config file path.
fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".config/glide/config.json")
}
