use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonSettings {
    pub server_url: String,
    pub device_name: String,
    pub clipboard_enabled: bool,
    pub input_enabled: bool,
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8080".to_string(),
            device_name: default_device_name(),
            clipboard_enabled: true,
            input_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonStatus {
    pub running: bool,
    pub connected: bool,
    pub connection_status: String,
    pub server_url: String,
    pub device_name: String,
    pub clipboard_enabled: bool,
    pub input_enabled: bool,
    pub platform: String,
}

#[derive(Debug, Clone)]
pub struct DaemonState {
    running: bool,
    connected: bool,
    settings: DaemonSettings,
    logs: Vec<String>,
}

impl DaemonState {
    pub fn new(settings: DaemonSettings) -> Self {
        let mut state = Self {
            running: true,
            connected: false,
            settings,
            logs: Vec::new(),
        };
        state.log("daemon initialized");
        state
    }

    pub fn status(&self) -> DaemonStatus {
        DaemonStatus {
            running: self.running,
            connected: self.connected,
            connection_status: if self.connected {
                "已连接".to_string()
            } else {
                "未连接".to_string()
            },
            server_url: self.settings.server_url.clone(),
            device_name: self.settings.device_name.clone(),
            clipboard_enabled: self.settings.clipboard_enabled,
            input_enabled: self.settings.input_enabled,
            platform: std::env::consts::OS.to_string(),
        }
    }

    pub fn update_settings(&mut self, settings: DaemonSettings) {
        self.settings = settings;
        self.log("settings updated");
    }

    pub fn connect(&mut self, server_url: &str) -> anyhow::Result<()> {
        let server_url = server_url.trim();
        if server_url.is_empty() {
            anyhow::bail!("server_url must not be empty");
        }
        self.settings.server_url = server_url.to_string();
        self.connected = true;
        self.log(&format!("connect requested: {}", mask_url(server_url)));
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.log("disconnect requested");
    }

    pub fn set_clipboard_enabled(&mut self, enabled: bool) {
        self.settings.clipboard_enabled = enabled;
        self.log(if enabled {
            "clipboard enabled"
        } else {
            "clipboard disabled"
        });
    }

    pub fn set_input_enabled(&mut self, enabled: bool) {
        self.settings.input_enabled = enabled;
        self.log(if enabled {
            "input enabled"
        } else {
            "input disabled"
        });
    }

    pub fn tail_logs(&self, limit: usize) -> Vec<String> {
        let len = self.logs.len();
        self.logs[len.saturating_sub(limit)..].to_vec()
    }

    fn log(&mut self, message: &str) {
        self.logs
            .push(format!("[{}] {}", Local::now().format("%H:%M:%S"), message));
    }
}

pub fn default_device_name() -> String {
    std::env::var("GLIDE_DEVICE_NAME")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "glide-device".to_string())
}

pub fn mask_url(url: &str) -> String {
    if let Some((prefix, _)) = url.split_once("token=") {
        format!("{prefix}token=***")
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_url, DaemonSettings, DaemonState};

    #[test]
    fn daemon_starts_disconnected() {
        let daemon = DaemonState::new(DaemonSettings::default());

        let status = daemon.status();

        assert!(status.running);
        assert!(!status.connected);
        assert_eq!("未连接", status.connection_status);
    }

    #[test]
    fn daemon_connect_rejects_empty_url() {
        let mut daemon = DaemonState::new(DaemonSettings::default());

        let result = daemon.connect("");

        assert!(result.is_err());
        assert!(!daemon.status().connected);
    }

    #[test]
    fn daemon_connect_updates_status() {
        let mut daemon = DaemonState::new(DaemonSettings::default());

        daemon.connect("http://server:8080").unwrap();

        let status = daemon.status();
        assert!(status.connected);
        assert_eq!("已连接", status.connection_status);
        assert_eq!("http://server:8080", status.server_url);
    }

    #[test]
    fn daemon_toggles_capabilities() {
        let mut daemon = DaemonState::new(DaemonSettings::default());

        daemon.set_clipboard_enabled(false);
        daemon.set_input_enabled(true);

        let status = daemon.status();
        assert!(!status.clipboard_enabled);
        assert!(status.input_enabled);
    }

    #[test]
    fn mask_url_hides_token_query() {
        let masked = mask_url("http://server:8080?token=secret");

        assert_eq!("http://server:8080?token=***", masked);
    }
}
