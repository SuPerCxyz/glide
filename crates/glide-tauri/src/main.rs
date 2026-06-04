#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod sync_engine;

use glide_core::clipboard::ClipboardKind;
use glide_core::policy::Policy;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

struct AppState {
    sync_engine: Arc<Mutex<sync_engine::SyncEngine>>,
    policy: Mutex<Policy>,
    sync_paused: Mutex<bool>,
    input_sharing_enabled: Mutex<bool>,
}

fn main() {
    let device_id = uuid::Uuid::new_v4().to_string();
    let device_name = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let sync_engine = Arc::new(Mutex::new(sync_engine::SyncEngine::new(
        device_id.clone(),
        device_name,
    )));

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

            // Start clipboard monitor.
            let sync_engine_clone = app.state::<AppState>().sync_engine.clone();
            let app_handle = app.handle().clone();

            tokio::spawn(async move {
                let engine = sync_engine_clone.lock().await;
                let device_id = engine.device_id.clone();
                let mut rx = engine.take_incoming().await;
                drop(engine);

                // Spawn clipboard monitor.
                let se = sync_engine_clone.clone();
                tokio::spawn(async move {
                    sync_engine::monitor_clipboard(device_id, move |item| {
                        let se = se.clone();
                        tokio::spawn(async move {
                            let engine = se.lock().await;
                            if let Err(e) = engine.send_clipboard(&item).await {
                                tracing::warn!("Failed to send clipboard: {}", e);
                            }
                        });
                    })
                    .await;
                });

                // Process incoming clipboard items.
                if let Some(mut rx) = rx {
                    while let Some(item) = rx.recv().await {
                        // Write to local clipboard.
                        if let Err(e) = apply_clipboard(&item).await {
                            tracing::warn!("Failed to apply clipboard: {}", e);
                        }
                        // Notify frontend.
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
async fn get_connection_status(state: tauri::State<'_, AppState>) -> String {
    let engine = state.sync_engine.lock().await;
    engine.connection_status.lock().await.clone()
}

#[tauri::command]
async fn connect_to_server(state: tauri::State<'_, AppState>, url: String) -> Result<String, String> {
    let engine = state.sync_engine.lock().await;
    engine.connect(url).await.map(|_| "connected".to_string())
}

#[tauri::command]
fn get_clipboard_history(_state: tauri::State<AppState>) -> serde_json::Value {
    serde_json::json!({ "items": [], "message": "连接服务器后显示剪贴板历史" })
}

#[tauri::command]
fn get_devices(_state: tauri::State<AppState>) -> serde_json::Value {
    serde_json::json!({ "devices": [], "message": "连接服务器后显示设备列表" })
}

#[tauri::command]
fn toggle_sync_pause(state: tauri::State<AppState>) -> bool {
    let mut paused = state.sync_paused.blocking_lock();
    *paused = !*paused;
    *paused
}

#[tauri::command]
fn get_sync_paused(state: tauri::State<AppState>) -> bool {
    *state.sync_paused.blocking_lock()
}

#[tauri::command]
fn toggle_input_sharing(state: tauri::State<AppState>) -> bool {
    let mut enabled = state.input_sharing_enabled.blocking_lock();
    *enabled = !*enabled;
    *enabled
}

#[tauri::command]
fn get_input_sharing_enabled(state: tauri::State<AppState>) -> bool {
    *state.input_sharing_enabled.blocking_lock()
}

#[tauri::command]
async fn get_server_url(state: tauri::State<'_, AppState>) -> String {
    let engine = state.sync_engine.lock().await;
    engine.server_url.lock().await.clone()
}

#[tauri::command]
async fn set_server_url(state: tauri::State<'_, AppState>, url: String) -> bool {
    let engine = state.sync_engine.lock().await;
    let mut server = engine.server_url.lock().await;
    if url.is_empty() || url.starts_with("http://") || url.starts_with("https://") {
        *server = url;
        true
    } else {
        false
    }
}

#[tauri::command]
fn get_policy(state: tauri::State<AppState>) -> serde_json::Value {
    let policy = state.policy.blocking_lock();
    serde_json::to_value(&*policy).unwrap_or(serde_json::json!({}))
}

#[tauri::command]
fn set_device_policy(
    state: tauri::State<AppState>,
    device_id: String,
    sync_enabled: bool,
    input_enabled: bool,
) -> bool {
    if let Ok(device_uuid) = device_id.parse() {
        let mut policy = state.policy.blocking_lock();
        policy.device_policies.retain(|dp| dp.device_id != device_uuid);
        policy.device_policies.push(glide_core::policy::DevicePolicy {
            device_id: device_uuid,
            sync_enabled,
            input_enabled,
        });
        true
    } else {
        false
    }
}

#[tauri::command]
fn set_type_policy(
    state: tauri::State<AppState>,
    kind: String,
    sync_enabled: bool,
    max_size: Option<u64>,
) -> bool {
    let clipboard_kind = match kind.as_str() {
        "text" => ClipboardKind::Text,
        "image" => ClipboardKind::Image,
        "file" => ClipboardKind::File,
        _ => return false,
    };
    let mut policy = state.policy.blocking_lock();
    policy.type_policies.retain(|tp| tp.kind != clipboard_kind);
    policy.type_policies.push(glide_core::policy::TypePolicy {
        kind: clipboard_kind,
        sync_enabled,
        max_size_bytes: max_size,
    });
    true
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
                        stdin.write_all(text.as_bytes()).await.map_err(|e| e.to_string())?;
                    }
                    child.wait().await.map_err(|e| e.to_string())?;
                    return Ok(());
                }

                #[cfg(target_os = "windows")]
                {
                    tokio::process::Command::new("powershell")
                        .args(["-command", &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"))])
                        .output()
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}
