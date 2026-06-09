use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use reqwest::blocking::Client as BlockingHttpClient;
use glide_core::display_layout::DisplayLayout;

/// 获取配置文件路径
fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("GLIDE_CONFIG_PATH") {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("Glide").join("config.json");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(config_home).join("glide").join("config.json");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("glide").join("config.json");
        }
    }

    std::env::temp_dir().join("glide-config.json")
}

/// 从磁盘加载配置
fn load_config() -> Option<AppSettings> {
    let path = config_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 保存配置到磁盘
fn save_config(settings: &AppSettings) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

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
    #[serde(default)]
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
    pub auth_username: String,
    pub auth_password: String,
    pub registration_token: String,
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
    fn trust_device(&self, device_id: &str) -> BackendResult<()>;
    fn untrust_device(&self, device_id: &str) -> BackendResult<()>;
    fn tail_logs(&self, limit: usize) -> BackendResult<Vec<String>>;
    fn export_diagnostics(&self) -> BackendResult<String>;
    fn get_platform_capabilities(&self) -> BackendResult<PlatformCapabilities>;

    // Monitor layout management
    fn detect_display_layout(&self) -> BackendResult<DisplayLayout>;
    fn save_display_layout(&self, layout: &DisplayLayout) -> BackendResult<()>;
    fn load_display_layout(&self) -> BackendResult<Option<DisplayLayout>>;
}

/// Mock backend for first phase.
/// Supports optional real HTTP connection to server for
/// connect/disconnect/register/list operations.
#[derive(Clone)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
    pub lan_state: Option<Arc<glide_desktop::lan_sync::LanSyncState>>,
    device_id: String,
    session_token: Arc<Mutex<Option<String>>>,
    http_client: Option<BlockingHttpClient>,
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
        Self::with_id("mock-device".to_string())
    }

    pub fn with_id(device_id: String) -> Self {
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "glide-device".to_string());

        // 默认设置
        let default_settings = AppSettings {
            server_url: "http://127.0.0.1:8080".to_string(),
            auth_username: String::new(),
            auth_password: String::new(),
            registration_token: String::new(),
            device_name,
            auto_connect: false,
            clipboard_enabled: true,
            input_enabled: false,
        };

        // 尝试从磁盘加载配置，失败则使用默认设置
        let settings = match load_config() {
            Some(saved) => {
                // 保留 hostname 作为设备名（如果保存的为空）
                let device_name = if saved.device_name.is_empty() {
                    default_settings.device_name.clone()
                } else {
                    saved.device_name
                };
                AppSettings {
                    device_name,
                    ..saved
                }
            }
            None => default_settings,
        };

        let mut backend = Self {
            state: Arc::new(Mutex::new(MockState {
                running: true,
                connected: false,
                settings,
                devices: vec![
                    DeviceInfo {
                        device_id: "linux-cli".to_string(),
                        name: "Linux 命令行设备".to_string(),
                        platform: "Linux X11".to_string(),
                        online: true,
                        trusted: true,
                    },
                    DeviceInfo {
                        device_id: "windows-vm".to_string(),
                        name: "Windows 虚拟机".to_string(),
                        platform: "Windows 11".to_string(),
                        online: false,
                        trusted: true,
                    },
                ],
                logs: Vec::new(),
            })),
            lan_state: None,
            device_id,
            session_token: Arc::new(Mutex::new(None)),
            http_client: Some(
                reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .expect("创建 HTTP 客户端失败"),
            ),
        };
        backend.push_log("模拟后端已初始化（支持 HTTP 服务端连接）");
        backend
    }

    fn with_state<T>(&self, f: impl FnOnce(&MockState) -> T) -> T {
        let state = self.state.lock().expect("模拟后端状态锁已损坏");
        f(&state)
    }

    fn with_state_mut<T>(&self, f: impl FnOnce(&mut MockState) -> T) -> T {
        let mut state = self.state.lock().expect("模拟后端状态锁已损坏");
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

    pub fn log(&self, message: &str) {
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

    pub fn save_auth(&self, username: &str, password: &str) -> BackendResult<()> {
        self.with_state_mut(|state| {
            state.settings.auth_username = username.to_string();
            state.settings.auth_password = password.to_string();
        });
        // 持久化到磁盘
        let settings = self.with_state(|s| s.settings.clone());
        if let Err(e) = save_config(&settings) {
            self.log(&format!("保存配置失败: {}", e));
        }
        self.log("认证信息已保存");
        Self::success(())
    }

    pub fn save_registration_token(&self, token: &str) -> BackendResult<()> {
        self.with_state_mut(|state| {
            state.settings.registration_token = token.to_string();
        });
        // 持久化到磁盘
        let settings = self.with_state(|s| s.settings.clone());
        if let Err(e) = save_config(&settings) {
            self.log(&format!("保存配置失败: {}", e));
        }
        self.log("注册 token 已保存");
        Self::success(())
    }

    /// 保存服务端地址到磁盘
    pub fn save_server_url(&self, url: &str) -> BackendResult<()> {
        self.with_state_mut(|state| {
            state.settings.server_url = url.to_string();
        });
        // 持久化到磁盘
        let settings = self.with_state(|s| s.settings.clone());
        if let Err(e) = save_config(&settings) {
            self.log(&format!("保存配置失败: {}", e));
        }
        self.log("服务端地址已保存");
        Self::success(())
    }

    /// 保存设备名到磁盘
    pub fn save_device_name(&self, name: &str) -> BackendResult<()> {
        self.with_state_mut(|state| {
            state.settings.device_name = name.to_string();
        });
        // 持久化到磁盘
        let settings = self.with_state(|s| s.settings.clone());
        if let Err(e) = save_config(&settings) {
            self.log(&format!("保存配置失败: {}", e));
        }
        self.log("设备名已保存");
        Self::success(())
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiBackend for MockBackend {
    fn get_service_status(&self) -> BackendResult<ServiceStatus> {
        let online_count = self.with_state(|state| {
            state.devices.iter().filter(|device| device.online).count()
        });
        let server_url = self.with_state(|state| state.settings.server_url.clone());
        let clipboard_enabled = self.with_state(|state| state.settings.clipboard_enabled);
        let input_enabled = self.with_state(|state| state.settings.input_enabled);
        let running = self.with_state(|state| state.running);

        // Check real server health when connected
        let conn_status = {
            let connected_flag = self.with_state(|s| s.connected);
            if connected_flag {
                // Health check when explicitly connected
                let health_url = format!("{}/api/v1/health", server_url.trim_end_matches('/'));
                match self.http_client.as_ref().unwrap().get(&health_url).timeout(std::time::Duration::from_secs(3)).send() {
                    Ok(resp) if resp.status().is_success() => "已连接".to_string(),
                    _ => "连接断开".to_string(),
                }
            } else {
                "未连接".to_string()
            }
        };

        Self::success(ServiceStatus {
            running,
            server_url,
            connection_status: conn_status,
            device_count: online_count,
            clipboard_enabled,
            input_enabled,
            file_transfer_enabled: false,
        })
    }

    fn start_service(&self) -> BackendResult<()> {
        self.with_state_mut(|state| state.running = true);
        self.log("已从界面请求启动后台服务");
        Self::success(())
    }

    fn stop_service(&self) -> BackendResult<()> {
        self.with_state_mut(|state| {
            state.running = false;
            state.connected = false;
        });
        self.log("已从界面请求停止后台服务");
        Self::success(())
    }

    fn list_devices(&self) -> BackendResult<Vec<DeviceInfo>> {
        let mut devices = self.with_state(|state| state.devices.clone());
        // Fetch devices from server when connected
        let is_connected = self.with_state(|s| s.connected);
        let server_url = self.with_state(|s| s.settings.server_url.clone());
        if is_connected && !server_url.is_empty() {
            let list_url = format!("{}/api/v1/devices", server_url.trim_end_matches('/'));
            if let Ok(resp) = self.http_client.as_ref().unwrap().get(&list_url).send() {
                if let Ok(server_devices) = resp.json::<Vec<DeviceInfo>>() {
                    // Replace mock devices with server devices
                    devices = server_devices;
                }
            }
        }
        // Merge LAN-discovered peers
        if let Some(ref ls) = self.lan_state {
            let trusted = ls.trusted_peers.blocking_read();
            let registry = ls.peer_registry.blocking_read();
            for peer in registry.all_peers() {
                let device_id = peer.device_id.to_string();
                // Skip if already in mock devices
                if devices.iter().any(|d| d.device_id == device_id) {
                    continue;
                }
                devices.push(DeviceInfo {
                    device_id: device_id.clone(),
                    name: peer.name.clone(),
                    platform: format!("LAN {}", peer.address),
                    online: matches!(peer.state, glide_core::discovery::PeerState::Active),
                    trusted: trusted.contains(&device_id),
                });
            }
        }
        Self::success(devices)
    }

    fn get_device_detail(&self, device_id: &str) -> BackendResult<DeviceInfo> {
        self.with_state(|state| {
            state
                .devices
                .iter()
                .find(|device| device.device_id == device_id)
                .cloned()
                .map(Self::success)
                .unwrap_or_else(|| Self::failure(format!("未找到设备：{device_id}")))
        })
    }

    fn connect_device(&self, device_id: &str) -> BackendResult<String> {
        let exists = self.with_state(|state| {
            state
                .devices
                .iter()
                .any(|device| device.device_id == device_id && device.trusted)
        });
        if !exists {
            return Self::failure(format!("设备尚未配对或不受信任：{device_id}"));
        }
        self.log(&format!("已请求连接设备：{device_id}"));
        Self::success("设备连接请求已发送".to_string())
    }

    fn disconnect_device(&self, device_id: &str) -> BackendResult<String> {
        self.log(&format!("已请求断开设备：{device_id}"));
        Self::success("设备断开请求已发送".to_string())
    }

    fn connect_server(&self, url: &str) -> BackendResult<String> {
        if url.trim().is_empty() {
            return Self::failure("服务端地址不能为空");
        }
        let server_url = url.trim().trim_end_matches('/').to_string();

        // 先尝试登录（如果设置了用户名和密码）
        let (username, password) = self.with_state(|s| {
            (s.settings.auth_username.clone(), s.settings.auth_password.clone())
        });

        let mut auth_token: Option<String> = None;

        if !username.is_empty() && !password.is_empty() {
            let login_url = format!("{}/api/v1/auth/login", server_url);
            let login_body = serde_json::json!({
                "username": username,
                "password": password,
            });

            self.log(&format!("正在登录服务端：{} (用户: {})", mask_url(&server_url), username));

            match self.http_client.as_ref().unwrap().post(&login_url).json(&login_body).send() {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(data) = resp.json::<serde_json::Value>() {
                        if let Some(token) = data["token"].as_str() {
                            auth_token = Some(token.to_string());
                            self.log("服务端登录成功");
                        }
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().unwrap_or_default();
                    let err = format!("登录失败 (HTTP {}): {}", status, text);
                    self.log(&err);
                    return Self::failure(format!("登录失败: {}", text));
                }
                Err(e) => {
                    let err = format!("登录请求失败: {}", e);
                    self.log(&err);
                    return Self::failure(format!("无法连接服务端: {}", e));
                }
            }
        }

        // 注册设备
        let reg_url = format!("{}/api/v1/devices/register", server_url);
        let device_name = self.with_state(|s| s.settings.device_name.clone());
        let registration_token = self.with_state(|s| s.settings.registration_token.clone());
        let platform = std::env::consts::OS.to_string();

        let mut body = serde_json::json!({
            "device_id": self.device_id,
            "name": device_name,
            "platform": platform,
            "trusted": true,
        });

        // 如果配置了注册 token，添加到请求体
        if !registration_token.is_empty() {
            body["registration_token"] = serde_json::json!(registration_token);
        }

        self.log(&format!("正在注册到服务端：{}", mask_url(&server_url)));

        // 构建请求，如果有 token 则添加 Authorization header
        let mut request = self.http_client.as_ref().unwrap().post(&reg_url).json(&body);
        if let Some(ref token) = auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        match request.send() {
            Ok(resp) if resp.status().is_success() => {
                self.with_state_mut(|state| {
                    state.connected = true;
                    state.settings.server_url = server_url.clone();
                });
                self.log("成功注册到服务端");
                Self::success("已连接".to_string())
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                let err = format!("服务端注册失败 (HTTP {}): {}", status, text);
                self.log(&err);
                // Fall back to mock connection for offline testing
                self.with_state_mut(|state| {
                    state.connected = true;
                    state.settings.server_url = server_url;
                });
                self.log("已回退到模拟连接模式");
                Self::success("已连接（离线模式）".to_string())
            }
            Err(e) => {
                let err = format!("无法连接服务端: {}", e);
                self.log(&err);
                // Fall back to mock connection
                self.with_state_mut(|state| {
                    state.connected = true;
                    state.settings.server_url = server_url;
                });
                self.log("已回退到模拟连接模式（服务端不可达）");
                Self::success("已连接（离线模式）".to_string())
            }
        }
    }

    fn disconnect_server(&self) -> BackendResult<String> {
        self.with_state_mut(|state| state.connected = false);
        self.log("已从界面断开服务端连接");
        Self::success("未连接".to_string())
    }

    fn get_clipboard_status(&self) -> BackendResult<ClipboardStatus> {
        Self::success(self.with_state(|state| ClipboardStatus {
            enabled: state.settings.clipboard_enabled,
            last_sync: "模拟状态：等待后台服务通信接入".to_string(),
        }))
    }

    fn set_clipboard_enabled(&self, enabled: bool) -> BackendResult<()> {
        self.with_state_mut(|state| state.settings.clipboard_enabled = enabled);
        self.log(if enabled {
            "剪贴板同步已开启"
        } else {
            "剪贴板同步已关闭"
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
            "键鼠共享已开启"
        } else {
            "键鼠共享已关闭"
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
            "已请求向 {device_id} 发送文件：{}",
            path.display()
        ));
        Self::failure("文件传输仍等待后台服务通信接入")
    }

    fn get_settings(&self) -> BackendResult<AppSettings> {
        Self::success(self.with_state(|state| state.settings.clone()))
    }

    fn trust_device(&self, device_id: &str) -> BackendResult<()> {
        if let Some(ref ls) = self.lan_state {
            let mut trusted = ls.trusted_peers.blocking_write();
            trusted.insert(device_id.to_string());
            self.log(&format!("已信任 LAN 设备：{}", device_id));
            // Send trust request to the peer
            let _id = device_id.to_string();
            tokio::spawn(async move {
                // Trust is recorded locally; the peer will send their own request
            });
            Self::success(())
        } else {
            Self::failure("LAN 引擎未启动")
        }
    }

    fn untrust_device(&self, device_id: &str) -> BackendResult<()> {
        if let Some(ref ls) = self.lan_state {
            ls.trusted_peers.blocking_write().remove(device_id);
            self.log(&format!("已取消信任 LAN 设备：{}", device_id));
            Self::success(())
        } else {
            Self::failure("LAN 引擎未启动")
        }
    }

    fn update_settings(&self, settings: &AppSettings) -> BackendResult<()> {
        self.with_state_mut(|state| state.settings = settings.clone());
        // 持久化到磁盘
        if let Err(e) = save_config(settings) {
            self.log(&format!("保存配置失败: {}", e));
        }
        self.log("设置已从界面更新");
        Self::success(())
    }

    fn tail_logs(&self, limit: usize) -> BackendResult<Vec<String>> {
        Self::success(self.with_state(|state| {
            let len = state.logs.len();
            state.logs[len.saturating_sub(limit)..].to_vec()
        }))
    }

    fn export_diagnostics(&self) -> BackendResult<String> {
        Self::success("模拟诊断.json".to_string())
    }

    fn get_platform_capabilities(&self) -> BackendResult<PlatformCapabilities> {
        let session_type =
            std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
        let mut notes = Vec::new();
        if session_type == "wayland" {
            notes.push("Wayland 可能阻止全局键鼠控制".to_string());
        }

        let input = match (std::env::consts::OS, session_type.as_str()) {
            ("windows", _) => "Windows SendInput 后端已具备，等待后台服务通信接入",
            ("linux", "wayland") => "Wayland 下能力受限",
            ("linux", _) => "Linux X11 xdotool 后端已具备，等待后台服务通信接入",
            _ => "该平台键鼠后端仍在规划中",
        };

        Self::success(PlatformCapabilities {
            platform: std::env::consts::OS.to_string(),
            clipboard: "剪贴板平台后端待后台服务通信接入".to_string(),
            input: input.to_string(),
            file_transfer: "规划通过后台服务通信接入".to_string(),
            notes,
        })
    }

    fn detect_display_layout(&self) -> BackendResult<DisplayLayout> {
        self.log("正在检测显示器布局...");
        match glide_desktop::monitor_detect::detect_monitor_layout(&self.device_id) {
            Ok(layout) => {
                self.log(&format!("检测到 {} 个显示器", layout.monitors.len()));
                Self::success(layout)
            }
            Err(e) => {
                self.log(&format!("显示器检测失败: {}", e));
                Self::failure(format!("显示器检测失败: {}", e))
            }
        }
    }

    fn save_display_layout(&self, layout: &DisplayLayout) -> BackendResult<()> {
        let config_dir = self.get_config_dir();
        let layout_path = config_dir.join("display_layout.json");

        match glide_desktop::monitor_detect::save_display_layout(layout, &layout_path) {
            Ok(()) => {
                self.log("显示器布局已保存");
                Self::success(())
            }
            Err(e) => {
                self.log(&format!("保存显示器布局失败: {}", e));
                Self::failure(format!("保存显示器布局失败: {}", e))
            }
        }
    }

    fn load_display_layout(&self) -> BackendResult<Option<DisplayLayout>> {
        let config_dir = self.get_config_dir();
        let layout_path = config_dir.join("display_layout.json");

        if !layout_path.exists() {
            self.log("未找到已保存的显示器布局");
            return Self::success(None);
        }

        match glide_desktop::monitor_detect::load_display_layout(&layout_path) {
            Ok(layout) => {
                self.log("已加载显示器布局");
                Self::success(Some(layout))
            }
            Err(e) => {
                self.log(&format!("加载显示器布局失败: {}", e));
                Self::failure(format!("加载显示器布局失败: {}", e))
            }
        }
    }
}

impl MockBackend {
    fn get_config_dir(&self) -> std::path::PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                return std::path::PathBuf::from(appdata).join("Glide");
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
                return std::path::PathBuf::from(config_home).join("glide");
            }
            if let Ok(home) = std::env::var("HOME") {
                return std::path::PathBuf::from(home)
                    .join(".config")
                    .join("glide");
            }
        }

        std::env::temp_dir().join("glide")
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
    use super::{GuiBackend, MockBackend, config_path, load_config, save_config, AppSettings};
    use std::sync::Mutex;

    // 全局锁，确保测试串行执行
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn mock_backend_updates_connection_status() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // 使用临时配置文件
        let temp_path = std::env::temp_dir().join(format!("glide-test-{}.json", std::process::id()));
        std::env::set_var("GLIDE_CONFIG_PATH", temp_path.to_str().unwrap());
        let _ = std::fs::remove_file(&temp_path);

        // Use a port that is very unlikely to have a real server on it
        // so the health check always fails in tests
        let backend = MockBackend::new();

        let initial = backend.get_service_status().data.unwrap();
        assert_eq!("未连接", initial.connection_status);

        let result = backend.connect_server("http://127.0.0.1:18099");
        assert!(result.success);

        // connect_server sets connected=true but falls back to offline mode
        // when the server is unreachable; status shows "连接断开"
        let status = backend.get_service_status().data.unwrap();
        assert_eq!("连接断开", status.connection_status);
        assert!(status.running);

        let _ = std::fs::remove_file(&temp_path);
        std::env::remove_var("GLIDE_CONFIG_PATH");
    }

    #[test]
    fn mock_backend_rejects_empty_server_url() {
        let _lock = TEST_MUTEX.lock().unwrap();

        let temp_path = std::env::temp_dir().join(format!("glide-test-empty-{}.json", std::process::id()));
        std::env::set_var("GLIDE_CONFIG_PATH", temp_path.to_str().unwrap());
        let _ = std::fs::remove_file(&temp_path);

        let backend = MockBackend::new();

        let result = backend.connect_server("");

        assert!(!result.success);
        assert_eq!("服务端地址不能为空", result.error.unwrap());

        let _ = std::fs::remove_file(&temp_path);
        std::env::remove_var("GLIDE_CONFIG_PATH");
    }

    #[test]
    fn mock_backend_toggles_clipboard_and_input_settings() {
        let _lock = TEST_MUTEX.lock().unwrap();

        let temp_path = std::env::temp_dir().join(format!("glide-test-toggle-{}.json", std::process::id()));
        std::env::set_var("GLIDE_CONFIG_PATH", temp_path.to_str().unwrap());
        let _ = std::fs::remove_file(&temp_path);

        let backend = MockBackend::new();

        assert!(backend.set_clipboard_enabled(false).success);
        assert!(backend.set_input_enabled(true).success);

        let settings = backend.get_settings().data.unwrap();
        assert!(!settings.clipboard_enabled);
        assert!(settings.input_enabled);

        let _ = std::fs::remove_file(&temp_path);
        std::env::remove_var("GLIDE_CONFIG_PATH");
    }

    #[test]
    fn config_persistence_and_save_methods() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // 使用临时文件避免影响真实配置
        let temp_path = std::env::temp_dir().join(format!("glide-test-combined-{}.json", std::process::id()));
        std::env::set_var("GLIDE_CONFIG_PATH", temp_path.to_str().unwrap());

        // 清理可能存在的旧文件
        let _ = std::fs::remove_file(&temp_path);

        let settings = AppSettings {
            server_url: "http://test-server:8080".to_string(),
            device_name: "test-device".to_string(),
            auto_connect: false,
            clipboard_enabled: true,
            input_enabled: false,
            auth_username: "testuser".to_string(),
            auth_password: "testpass".to_string(),
            registration_token: "test-token".to_string(),
        };

        // 保存配置
        let result = save_config(&settings);
        assert!(result.is_ok(), "save_config failed: {:?}", result.err());

        // 验证文件存在
        assert!(temp_path.exists(), "config file should exist at {:?}", temp_path);

        // 直接读取文件验证内容
        let content = std::fs::read_to_string(&temp_path)
            .expect("should be able to read config file");
        assert!(content.contains("test-server"), "config should contain test-server");

        // 加载配置
        let loaded = load_config();
        assert!(loaded.is_some(), "load_config should return Some");

        let loaded = loaded.unwrap();
        assert_eq!(loaded.server_url, settings.server_url);
        assert_eq!(loaded.auth_username, settings.auth_username);
        assert_eq!(loaded.registration_token, settings.registration_token);

        // 测试 save 方法也持久化到磁盘
        let backend = MockBackend::new();
        backend.save_server_url("http://method-test:9090");
        backend.save_auth("method-user", "method-pass");
        backend.save_registration_token("method-token");

        // 验证文件更新
        let loaded = load_config().expect("should load config after save methods");
        assert_eq!(loaded.server_url, "http://method-test:9090");
        assert_eq!(loaded.auth_username, "method-user");
        assert_eq!(loaded.auth_password, "method-pass");
        assert_eq!(loaded.registration_token, "method-token");

        // 清理
        let _ = std::fs::remove_file(&temp_path);
        std::env::remove_var("GLIDE_CONFIG_PATH");
    }
}
