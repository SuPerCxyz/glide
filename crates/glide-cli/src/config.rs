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
    config_path_from_env(
        std::env::var("GLIDE_CONFIG_PATH").ok(),
        std::env::var("APPDATA").ok(),
        std::env::var("HOME").ok(),
        cfg!(target_os = "windows"),
    )
}

fn config_path_from_env(
    explicit_path: Option<String>,
    appdata: Option<String>,
    home: Option<String>,
    is_windows: bool,
) -> std::path::PathBuf {
    if let Some(path) = explicit_path.filter(|p| !p.trim().is_empty()) {
        return std::path::PathBuf::from(path);
    }

    if is_windows {
        if let Some(appdata) = appdata.filter(|p| !p.trim().is_empty()) {
            return std::path::PathBuf::from(appdata)
                .join("Glide")
                .join("config.json");
        }
    }

    let home = home.unwrap_or_else(|| ".".to_string());
    std::path::PathBuf::from(home).join(".config/glide/config.json")
}

#[cfg(test)]
mod tests {
    use super::config_path_from_env;

    #[test]
    fn explicit_config_path_wins() {
        let path = config_path_from_env(
            Some("/tmp/glide.json".to_string()),
            Some("C:\\Users\\me\\AppData\\Roaming".to_string()),
            Some("/home/me".to_string()),
            true,
        );

        assert_eq!(path.to_string_lossy(), "/tmp/glide.json");
    }

    #[test]
    fn windows_uses_appdata() {
        let path = config_path_from_env(
            None,
            Some("C:\\Users\\me\\AppData\\Roaming".to_string()),
            Some("/home/me".to_string()),
            true,
        );

        assert_eq!(
            path.to_string_lossy(),
            "C:\\Users\\me\\AppData\\Roaming/Glide/config.json"
        );
    }

    #[test]
    fn non_windows_uses_home_config() {
        let path = config_path_from_env(None, None, Some("/home/me".to_string()), false);

        assert_eq!(path.to_string_lossy(), "/home/me/.config/glide/config.json");
    }
}
