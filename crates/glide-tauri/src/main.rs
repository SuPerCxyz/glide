mod sync_engine;

use glide_core::clipboard::ClipboardKind;
use glide_core::policy::Policy;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tokio::sync::Mutex;

struct AppState {
    sync_engine: Arc<Mutex<sync_engine::SyncEngine>>,
    policy: Mutex<Policy>,
    sync_paused: Mutex<bool>,
    input_sharing_enabled: Mutex<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DesktopConfig {
    server_url: String,
    #[serde(default)]
    registration_token: String,
}

fn app_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("Glide");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("glide");
        }
    }

    std::env::temp_dir().join("glide")
}

fn app_log_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(localappdata).join("Glide").join("logs");
        }
    }
    app_config_dir().join("logs")
}

fn ensure_runtime_dirs() -> Result<(), String> {
    std::fs::create_dir_all(app_config_dir()).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(app_log_dir()).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_startup_log(message: &str) {
    let _ = ensure_runtime_dirs();
    let path = app_log_dir().join("startup.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{} {}", chrono::Utc::now().to_rfc3339(), message);
    }
}

fn install_panic_logger() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write_startup_log(&format!("panic: {}", info));
        default_hook(info);
    }));
}

fn config_path() -> PathBuf {
    app_config_dir().join("config.json")
}

fn load_config() -> DesktopConfig {
    let path = config_path();
    match std::fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => DesktopConfig::default(),
    }
}

fn save_config(config: &DesktopConfig) -> Result<(), String> {
    ensure_runtime_dirs()?;
    let data = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), data).map_err(|e| e.to_string())
}

/// Check if WebView2 Runtime is discoverable on Windows.
///
/// This check is diagnostic only. Tauri and the bundled installer own the
/// actual WebView2 bootstrap path, so a false negative must not abort startup.
#[cfg(target_os = "windows")]
fn check_webview2() -> bool {
    use std::process::Command;

    for var in ["PROGRAMFILES(X86)", "ProgramFiles(x86)", "PROGRAMFILES"] {
        if let Ok(program_files) = std::env::var(var) {
            let webview2_path = std::path::Path::new(&program_files)
                .join("Microsoft")
                .join("EdgeWebView")
                .join("Application");
            if webview2_path.exists() {
                return true;
            }
        }
    }

    for key in [
        "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BEB-235B8DB25D42}",
        "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BEB-235B8DB25D42}",
        "HKCU\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BEB-235B8DB25D42}",
    ] {
        if Command::new("reg")
            .args(["query", key, "/v", "pv"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

#[cfg(not(target_os = "windows"))]
fn check_webview2() -> bool {
    true // Non-Windows platforms always pass this check
}

#[cfg(target_os = "windows")]
fn show_startup_error(message: &str) {
    let escaped = message.replace('\'', "''");
    let command = format!(
        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', 'Glide startup error', 'OK', 'Error')",
        escaped
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &command])
        .output();
}

#[cfg(not(target_os = "windows"))]
fn show_startup_error(_message: &str) {}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "暂停同步", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &quit])?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Glide - 剪贴板同步")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "pause" => {
                let state = app.state::<AppState>();
                let mut paused = state.sync_paused.blocking_lock();
                *paused = !*paused;
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(w) = tray.app_handle().get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    } else {
        tracing::warn!("Default window icon is unavailable; creating tray without icon");
    }

    tray.build(app).map(|_| ())
}

fn main() {
    install_panic_logger();

    if let Err(e) = ensure_runtime_dirs() {
        tracing::warn!("Failed to create runtime directories: {}", e);
        write_startup_log(&format!("failed to create runtime directories: {}", e));
    }

    // Do not exit on a diagnostic false negative. Win11 and packaged installs
    // can provide WebView2 in locations that this lightweight check misses.
    if !check_webview2() {
        tracing::warn!(
            "WebView2 runtime was not found by the diagnostic check; continuing startup"
        );
    }

    let device_id = uuid::Uuid::new_v4().to_string();
    let device_name = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let sync_engine = Arc::new(Mutex::new(sync_engine::SyncEngine::new(
        device_id.clone(),
        device_name,
    )));
    let saved_config = load_config();
    if !saved_config.server_url.is_empty() {
        let engine = sync_engine.blocking_lock();
        let mut server_url = engine.server_url.blocking_lock();
        *server_url = saved_config.server_url.clone();
    }

    let state = AppState {
        sync_engine,
        policy: Mutex::new(Policy::default()),
        sync_paused: Mutex::new(false),
        input_sharing_enabled: Mutex::new(false),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .setup(move |app| {
            if let Err(e) = setup_tray(app) {
                tracing::warn!("Failed to initialize tray: {}", e);
                write_startup_log(&format!("failed to initialize tray: {}", e));
            }

            // Start clipboard monitor and incoming handler.
            // Must use tauri::async_runtime::spawn — plain tokio::spawn panics
            // with 'there is no reactor running' before Tauri's embedded
            // runtime is fully initialized.
            let sync_engine_for_monitor = app.state::<AppState>().sync_engine.clone();
            let sync_engine_for_recv = app.state::<AppState>().sync_engine.clone();
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let mut rx = {
                    let engine = sync_engine_for_recv.lock().await;
                    engine.take_incoming().await
                };

                let se_monitor = sync_engine_for_monitor.clone();
                let monitor_device_id = {
                    let engine = se_monitor.lock().await;
                    engine.device_id.clone()
                };

                let se_for_monitor = se_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    sync_engine::monitor_clipboard(monitor_device_id, move |item| {
                        let se = se_for_monitor.clone();
                        tauri::async_runtime::spawn(async move {
                            let engine = se.lock().await;
                            if let Err(e) = engine.send_clipboard(&item).await {
                                tracing::warn!("Failed to send clipboard: {}", e);
                            }
                        });
                    })
                    .await;
                });

                if let Some(ref mut rx) = rx {
                    while let Some(item) = rx.recv().await {
                        if let Err(e) = apply_clipboard(&item).await {
                            tracing::warn!("Failed to apply clipboard: {}", e);
                        }
                        let _ = app_handle.emit("clipboard-received", &item.item_id);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_connection_status,
            get_clipboard_history,
            get_devices,
            toggle_sync_pause,
            get_sync_paused,
            toggle_input_sharing,
            get_input_sharing_enabled,
            get_server_url,
            get_registration_token,
            set_server_url,
            connect_to_server,
            get_policy,
            set_device_policy,
            set_type_policy,
            get_version,
            login,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            let message = format!("error while running tauri application: {}", e);
            write_startup_log(&message);
            show_startup_error(&message);
            eprintln!("{}", message);
            std::process::exit(1);
        });
}

#[tauri::command]
async fn get_connection_status(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let engine = state.sync_engine.lock().await;
    let cs = engine.connection_status.lock().await;
    let status = cs.clone();
    drop(cs);
    drop(engine);
    Ok(status)
}

#[tauri::command]
async fn connect_to_server(
    state: tauri::State<'_, AppState>,
    url: String,
    registration_token: Option<String>,
) -> Result<String, String> {
    let engine = state.sync_engine.lock().await;
    engine
        .connect(url.clone(), registration_token.clone())
        .await?;
    save_config(&DesktopConfig {
        server_url: url,
        registration_token: registration_token.unwrap_or_default(),
    })?;
    Ok("connected".to_string())
}

#[tauri::command]
async fn get_clipboard_history(
    _state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "items": [], "message": "连接服务器后显示剪贴板历史" }))
}

#[tauri::command]
async fn get_devices(_state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "devices": [], "message": "连接服务器后显示设备列表" }))
}

#[tauri::command]
async fn toggle_sync_pause(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut paused = state.sync_paused.lock().await;
    *paused = !*paused;
    Ok(*paused)
}

#[tauri::command]
async fn get_sync_paused(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.sync_paused.lock().await)
}

#[tauri::command]
async fn toggle_input_sharing(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut enabled = state.input_sharing_enabled.lock().await;
    *enabled = !*enabled;
    Ok(*enabled)
}

#[tauri::command]
async fn get_input_sharing_enabled(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.input_sharing_enabled.lock().await)
}

#[tauri::command]
async fn get_server_url(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let engine = state.sync_engine.lock().await;
    let su = engine.server_url.lock().await;
    let url = su.clone();
    drop(su);
    drop(engine);
    Ok(url)
}

#[tauri::command]
async fn set_server_url(state: tauri::State<'_, AppState>, url: String) -> Result<bool, String> {
    let engine = state.sync_engine.lock().await;
    let mut server = engine.server_url.lock().await;
    if url.is_empty() || url.starts_with("http://") || url.starts_with("https://") {
        *server = url.clone();
        let mut config = load_config();
        config.server_url = url;
        save_config(&config)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn get_registration_token() -> Result<String, String> {
    Ok(load_config().registration_token)
}

#[tauri::command]
async fn get_policy(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let policy = state.policy.lock().await;
    Ok(serde_json::to_value(&*policy).unwrap_or(serde_json::json!({})))
}

#[tauri::command]
async fn set_device_policy(
    state: tauri::State<'_, AppState>,
    device_id: String,
    sync_enabled: bool,
    input_enabled: bool,
) -> Result<bool, String> {
    if let Ok(device_uuid) = device_id.parse() {
        let mut policy = state.policy.lock().await;
        policy
            .device_policies
            .retain(|dp| dp.device_id != device_uuid);
        policy
            .device_policies
            .push(glide_core::policy::DevicePolicy {
                device_id: device_uuid,
                sync_enabled,
                input_enabled,
            });
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn set_type_policy(
    state: tauri::State<'_, AppState>,
    kind: String,
    sync_enabled: bool,
    max_size: Option<u64>,
) -> Result<bool, String> {
    let clipboard_kind = match kind.as_str() {
        "text" => ClipboardKind::Text,
        "image" => ClipboardKind::Image,
        "file" => ClipboardKind::File,
        _ => return Ok(false),
    };
    let mut policy = state.policy.lock().await;
    policy.type_policies.retain(|tp| tp.kind != clipboard_kind);
    policy.type_policies.push(glide_core::policy::TypePolicy {
        kind: clipboard_kind,
        sync_enabled,
        max_size_bytes: max_size,
    });
    Ok(true)
}

#[tauri::command]
fn get_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Login with username/password and connect to server.
#[tauri::command]
async fn login(
    state: tauri::State<'_, AppState>,
    url: String,
    username: String,
    password: String,
) -> Result<String, String> {
    let engine = state.sync_engine.lock().await;
    let token = engine.login(url.clone(), username, password).await?;
    save_config(&DesktopConfig {
        server_url: url,
        registration_token: String::new(),
    })?;
    Ok(token)
}

/// Apply a received clipboard item to the local clipboard.
async fn apply_clipboard(item: &glide_core::clipboard::ClipboardItem) -> Result<(), String> {
    for rep in &item.representations {
        if let glide_core::mime_rep::RepresentationContent::Text(text) = &rep.content {
            if rep.mime_type == "text/plain" {
                #[cfg(target_os = "linux")]
                {
                    let mut child = tokio::process::Command::new("xclip")
                        .args(["-i", "-selection", "clipboard"])
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .map_err(|e| e.to_string())?;

                    if let Some(mut stdin) = child.stdin.take() {
                        use tokio::io::AsyncWriteExt;
                        stdin
                            .write_all(text.as_bytes())
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    child.wait().await.map_err(|e| e.to_string())?;
                    return Ok(());
                }

                #[cfg(target_os = "windows")]
                {
                    tokio::process::Command::new("powershell")
                        .args([
                            "-command",
                            &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''")),
                        ])
                        .output()
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }

                #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                {
                    let _ = text;
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}
