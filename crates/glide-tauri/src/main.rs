mod sync_engine;

use glide_core::clipboard::ClipboardKind;
use glide_core::policy::Policy;
use serde::{Deserialize, Serialize};
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

fn save_server_url(url: &str) -> Result<(), String> {
    ensure_runtime_dirs()?;
    let config = DesktopConfig {
        server_url: url.to_string(),
    };
    let data = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), data).map_err(|e| e.to_string())
}

/// Check if WebView2 Runtime is available on Windows.
/// If not, show a MessageBox with download instructions and exit.
#[cfg(target_os = "windows")]
fn check_webview2() -> bool {
    use std::process::Command;
    // Check if WebView2 loader DLL exists or if Edge is installed.
    let program_files = std::env::var("PROGRAMFILES(x86)").unwrap_or_default();
    let webview2_path = format!("{}\\Microsoft\\EdgeWebView\\Application", program_files);
    if std::path::Path::new(&webview2_path).exists() {
        return true;
    }
    // Check via registry (simpler approach)
    let output = Command::new("reg")
        .args(["query", "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BEB-235B8DB25D42}", "/v", "pv"])
        .output();
    if let Ok(out) = output {
        if out.status.success() {
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
fn show_webview2_error() {
    use std::process::Command;
    // Show a Windows MessageBox with download instructions.
    let msg = "Glide requires WebView2 Runtime to run.\n\n\
               Please download and install it from:\n\
               https://developer.microsoft.com/en-us/microsoft-edge/webview2/\n\n\
               After installing, restart Glide.";
    let _ = Command::new("powershell")
        .args(["-command", &format!(
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', 'Glide - Missing Component', 'OK', 'Error')",
            msg.replace('\'', "''")
        )])
        .output();
}

#[cfg(not(target_os = "windows"))]
fn show_webview2_error() {}

fn main() {
    if let Err(e) = ensure_runtime_dirs() {
        tracing::warn!("Failed to create runtime directories: {}", e);
    }

    // Check WebView2 availability before starting.
    if !check_webview2() {
        show_webview2_error();
        std::process::exit(1);
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
        *server_url = saved_config.server_url;
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
            // System tray.
            let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "暂停同步", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
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
                })
                .build(app)?;

            // Start clipboard monitor and incoming handler.
            let sync_engine_for_monitor = app.state::<AppState>().sync_engine.clone();
            let sync_engine_for_recv = app.state::<AppState>().sync_engine.clone();
            let app_handle = app.handle().clone();

            tokio::spawn(async move {
                let mut rx = {
                    let engine = sync_engine_for_recv.lock().await;
                    engine.take_incoming().await
                };

                let se_monitor = sync_engine_for_monitor.clone();
                let monitor_device_id = {
                    let engine = se_monitor.lock().await;
                    engine.device_id.clone()
                };

                tokio::spawn(async move {
                    sync_engine::monitor_clipboard(monitor_device_id, move |item| {
                        let se = se_monitor.clone();
                        tokio::spawn(async move {
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
            set_server_url,
            connect_to_server,
            get_policy,
            set_device_policy,
            set_type_policy,
            get_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
) -> Result<String, String> {
    let engine = state.sync_engine.lock().await;
    engine.connect(url.clone()).await?;
    save_server_url(&url)?;
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
        save_server_url(&url)?;
        Ok(true)
    } else {
        Ok(false)
    }
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
