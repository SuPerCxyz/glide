use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Result from a backend operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Service status from the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub server_url: String,
    pub connection_status: String,
    pub device_count: usize,
    pub clipboard_enabled: bool,
    pub input_enabled: bool,
    pub file_transfer_enabled: bool,
}

/// Device info from the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub platform: String,
    pub online: bool,
    pub trusted: bool,
}

/// Settings from the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub server_url: String,
    pub device_name: String,
    pub auto_connect: bool,
    pub clipboard_enabled: bool,
    pub input_enabled: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardStatus {
    pub enabled: bool,
    pub last_sync: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputStatus {
    pub enabled: bool,
    pub platform_ready: bool,
    pub limitation: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferStatus {
    pub enabled: bool,
    pub pending_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    pub platform: String,
    pub clipboard: String,
    pub input: String,
    pub file_transfer: String,
    pub notes: Vec<String>,
}

/// Trait for GUI backend communication.
/// First phase: direct in-process calls.
/// Future: IPC via Unix socket / Named Pipe.
#[allow(dead_code)]
pub trait GuiBackend: Send + Sync {
    fn get_service_status(&self) -> BackendResult<ServiceStatus>;
    fn start_service(&self) -> BackendResult<()>;
    fn stop_service(&self) -> BackendResult<()>;
    fn list_devices(&self) -> BackendResult<Vec<DeviceInfo>>;
    fn get_device_detail(&self, device_id: &str) -> BackendResult<DeviceInfo>;
    fn pair_device(&self) -> BackendResult<String>;
    fn connect_device(&self, device_id: &str) -> BackendResult<String>;
    fn disconnect_device(&self, device_id: &str) -> BackendResult<String>;
    fn connect_server(&self, url: &str) -> BackendResult<String>;
    fn disconnect_server(&self) -> BackendResult<String>;
    fn get_clipboard_status(&self) -> BackendResult<ClipboardStatus>;
    fn set_clipboard_enabled(&self, enabled: bool) -> BackendResult<()>;
    fn get_input_status(&self) -> BackendResult<InputStatus>;
    fn set_input_enabled(&self, enabled: bool) -> BackendResult<()>;
    fn get_file_transfer_status(&self) -> BackendResult<FileTransferStatus>;
    fn send_file(&self, device_id: &str, path: &Path) -> BackendResult<()>;
    fn get_settings(&self) -> BackendResult<AppSettings>;
    fn update_settings(&self, settings: &AppSettings) -> BackendResult<()>;
    fn tail_logs(&self, limit: usize) -> BackendResult<Vec<String>>;
    fn export_diagnostics(&self) -> BackendResult<String>;
    fn get_platform_capabilities(&self) -> BackendResult<PlatformCapabilities>;
}

/// Mock backend for first phase.
#[derive(Clone)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
}

#[derive(Debug, Clone)]
struct MockState {
    running: bool,
    connected: bool,
    settings: AppSettings,
    devices: Vec<DeviceInfo>,
    logs: Vec<String>,
}

impl MockBackend {
    pub fn new() -> Self {
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "glide-device".to_string());

        let mut backend = Self {
            state: Arc::new(Mutex::new(MockState {
                running: true,
                connected: false,
                settings: AppSettings {
                    server_url: "http://127.0.0.1:8080".to_string(),
                    device_name,
                    auto_connect: false,
                    clipboard_enabled: true,
                    input_enabled: false,
                },
                devices: vec![
                    DeviceInfo {
                        device_id: "linux-cli".to_string(),
                        name: "Linux CLI".to_string(),
                        platform: "Linux X11".to_string(),
                        online: true,
                        trusted: true,
                    },
                    DeviceInfo {
                        device_id: "windows-vm".to_string(),
                        name: "Windows VM".to_string(),
                        platform: "Windows 11".to_string(),
                        online: false,
                        trusted: true,
                    },
                ],
                logs: Vec::new(),
            })),
        };
        backend.push_log("Mock backend initialized; daemon IPC is not attached yet");
        backend
    }

    fn with_state<T>(&self, f: impl FnOnce(&MockState) -> T) -> T {
        let state = self.state.lock().expect("mock backend state poisoned");
        f(&state)
    }

    fn with_state_mut<T>(&self, f: impl FnOnce(&mut MockState) -> T) -> T {
        let mut state = self.state.lock().expect("mock backend state poisoned");
        f(&mut state)
    }

    fn push_log(&mut self, message: &str) {
        let message = format!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), message);
        self.with_state_mut(|state| {
            state.logs.push(message);
            if state.logs.len() > 1000 {
                state.logs.drain(0..500);
            }
        });
    }

    fn log(&self, message: &str) {
        let message = format!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), message);
        self.with_state_mut(|state| {
            state.logs.push(message);
            if state.logs.len() > 1000 {
                state.logs.drain(0..500);
            }
        });
    }

    fn success<T>(data: T) -> BackendResult<T> {
        BackendResult {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn failure<T>(message: impl Into<String>) -> BackendResult<T> {
        BackendResult {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiBackend for MockBackend {
    fn get_service_status(&self) -> BackendResult<ServiceStatus> {
        Self::success(self.with_state(|state| ServiceStatus {
            running: state.running,
            server_url: state.settings.server_url.clone(),
            connection_status: if state.connected {
                "已连接".to_string()
            } else {
                "未连接".to_string()
            },
            device_count: state.devices.iter().filter(|device| device.online).count(),
            clipboard_enabled: state.settings.clipboard_enabled,
            input_enabled: state.settings.input_enabled,
            file_transfer_enabled: false,
        }))
    }

    fn start_service(&self) -> BackendResult<()> {
        self.with_state_mut(|state| state.running = true);
        self.log("Service start requested from GUI");
        Self::success(())
    }

    fn stop_service(&self) -> BackendResult<()> {
        self.with_state_mut(|state| {
            state.running = false;
            state.connected = false;
        });
        self.log("Service stop requested from GUI");
        Self::success(())
    }

    fn list_devices(&self) -> BackendResult<Vec<DeviceInfo>> {
        Self::success(self.with_state(|state| state.devices.clone()))
    }

    fn get_device_detail(&self, device_id: &str) -> BackendResult<DeviceInfo> {
        self.with_state(|state| {
            state
                .devices
                .iter()
                .find(|device| device.device_id == device_id)
                .cloned()
                .map(Self::success)
                .unwrap_or_else(|| Self::failure(format!("Device not found: {device_id}")))
        })
    }

    fn pair_device(&self) -> BackendResult<String> {
        self.log("Pairing requested; daemon IPC will provide the real pairing flow");
        Self::success("PAIR-000000".to_string())
    }

    fn connect_device(&self, device_id: &str) -> BackendResult<String> {
        let exists = self.with_state(|state| {
            state
                .devices
                .iter()
                .any(|device| device.device_id == device_id && device.trusted)
        });
        if !exists {
            return Self::failure(format!("Device is not paired or trusted: {device_id}"));
        }
        self.log(&format!("Device connect requested: {device_id}"));
        Self::success("设备连接请求已发送".to_string())
    }

    fn disconnect_device(&self, device_id: &str) -> BackendResult<String> {
        self.log(&format!("Device disconnect requested: {device_id}"));
        Self::success("设备断开请求已发送".to_string())
    }

    fn connect_server(&self, url: &str) -> BackendResult<String> {
        if url.trim().is_empty() {
            return Self::failure("服务端地址不能为空");
        }
        self.with_state_mut(|state| {
            state.connected = true;
            state.settings.server_url = url.trim().to_string();
        });
        self.log(&format!("Server connect requested: {}", mask_url(url)));
        Self::success("已连接".to_string())
    }

    fn disconnect_server(&self) -> BackendResult<String> {
        self.with_state_mut(|state| state.connected = false);
        self.log("Server disconnected from GUI");
        Self::success("未连接".to_string())
    }

    fn get_clipboard_status(&self) -> BackendResult<ClipboardStatus> {
        Self::success(self.with_state(|state| ClipboardStatus {
            enabled: state.settings.clipboard_enabled,
            last_sync: "mock: waiting for daemon IPC".to_string(),
        }))
    }

    fn set_clipboard_enabled(&self, enabled: bool) -> BackendResult<()> {
        self.with_state_mut(|state| state.settings.clipboard_enabled = enabled);
        self.log(if enabled {
            "Clipboard sync enabled"
        } else {
            "Clipboard sync disabled"
        });
        Self::success(())
    }

    fn get_input_status(&self) -> BackendResult<InputStatus> {
        Self::success(self.with_state(|state| InputStatus {
            enabled: state.settings.input_enabled,
            platform_ready: cfg!(target_os = "windows")
                || cfg!(target_os = "macos")
                || std::env::var("XDG_SESSION_TYPE").unwrap_or_default() != "wayland",
            limitation: if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
                Some("Wayland 下全局键鼠控制受合成器权限限制".to_string())
            } else {
                None
            },
        }))
    }

    fn set_input_enabled(&self, enabled: bool) -> BackendResult<()> {
        self.with_state_mut(|state| state.settings.input_enabled = enabled);
        self.log(if enabled {
            "Input sharing enabled"
        } else {
            "Input sharing disabled"
        });
        Self::success(())
    }

    fn get_file_transfer_status(&self) -> BackendResult<FileTransferStatus> {
        Self::success(FileTransferStatus {
            enabled: false,
            pending_count: 0,
        })
    }

    fn send_file(&self, device_id: &str, path: &Path) -> BackendResult<()> {
        self.log(&format!(
            "File transfer requested for {device_id}: {}",
            path.display()
        ));
        Self::failure("文件传输仍等待 daemon IPC 接入")
    }

    fn get_settings(&self) -> BackendResult<AppSettings> {
        Self::success(self.with_state(|state| state.settings.clone()))
    }

    fn update_settings(&self, settings: &AppSettings) -> BackendResult<()> {
        self.with_state_mut(|state| state.settings = settings.clone());
        self.log("Settings updated from GUI");
        Self::success(())
    }

    fn tail_logs(&self, limit: usize) -> BackendResult<Vec<String>> {
        Self::success(self.with_state(|state| {
            let len = state.logs.len();
            state.logs[len.saturating_sub(limit)..].to_vec()
        }))
    }

    fn export_diagnostics(&self) -> BackendResult<String> {
        Self::success("mock-diagnostics.json".to_string())
    }

    fn get_platform_capabilities(&self) -> BackendResult<PlatformCapabilities> {
        let session_type =
            std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
        let mut notes =
            vec!["GUI is currently backed by MockBackend; daemon IPC is pending".to_string()];
        if session_type == "wayland" {
            notes.push("Wayland may block global keyboard and mouse control".to_string());
        }

        Self::success(PlatformCapabilities {
            platform: std::env::consts::OS.to_string(),
            clipboard: "planned via glide-platform".to_string(),
            input: if session_type == "wayland" {
                "limited on Wayland".to_string()
            } else {
                "planned via glide-platform".to_string()
            },
            file_transfer: "planned via daemon IPC".to_string(),
            notes,
        })
    }
}

fn mask_url(url: &str) -> String {
    if let Some((base, _)) = url.split_once("token=") {
        format!("{base}token=***")
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{GuiBackend, MockBackend};

    #[test]
    fn mock_backend_updates_connection_status() {
        let backend = MockBackend::new();

        let initial = backend.get_service_status().data.unwrap();
        assert_eq!("未连接", initial.connection_status);

        let result = backend.connect_server("http://127.0.0.1:8080");
        assert!(result.success);

        let connected = backend.get_service_status().data.unwrap();
        assert_eq!("已连接", connected.connection_status);
    }

    #[test]
    fn mock_backend_rejects_empty_server_url() {
        let backend = MockBackend::new();

        let result = backend.connect_server("");

        assert!(!result.success);
        assert_eq!("服务端地址不能为空", result.error.unwrap());
    }

    #[test]
    fn mock_backend_toggles_clipboard_and_input_settings() {
        let backend = MockBackend::new();

        assert!(backend.set_clipboard_enabled(false).success);
        assert!(backend.set_input_enabled(true).success);

        let settings = backend.get_settings().data.unwrap();
        assert!(!settings.clipboard_enabled);
        assert!(settings.input_enabled);
    }
}
