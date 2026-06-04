#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use glide_core::clipboard::ClipboardKind;
use glide_core::policy::Policy;
use std::sync::Mutex;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

/// Shared app state.
struct AppState {
    connection_status: Mutex<String>,
    policy: Mutex<Policy>,
    server_url: Mutex<String>,
    sync_paused: Mutex<bool>,
    input_sharing_enabled: Mutex<bool>,
}

/// Build the system tray menu.
fn build_system_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let show = CustomMenuItem::new("show".to_string(), "显示窗口");
    let pause = CustomMenuItem::new("pause".to_string(), "暂停同步");
    let input_toggle = CustomMenuItem::new("input_toggle".to_string(), "键鼠共享");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(pause)
        .add_item(input_toggle)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new().with_menu(tray_menu).with_tooltip("Glide - 剪贴板同步")
}

fn main() {
    let state = AppState {
        connection_status: Mutex::new("disconnected".to_string()),
        policy: Mutex::new(Policy::default()),
        server_url: Mutex::new("".to_string()),
        sync_paused: Mutex::new(false),
        input_sharing_enabled: Mutex::new(false),
    };

    tauri::Builder::default()
        .manage(state)
        .system_tray(build_system_tray())
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick { .. } => {
                // Single click on tray icon shows the window.
                if let Some(window) = app.get_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    app.exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "pause" => {
                    let state = app.state::<AppState>();
                    let mut paused = state.sync_paused.lock().unwrap();
                    *paused = !*paused;
                    let tray = app.tray_handle();
                    let item = tray.get_item("pause");
                    if *paused {
                        let _ = item.set_title("恢复同步");
                    } else {
                        let _ = item.set_title("暂停同步");
                    }
                }
                "input_toggle" => {
                    let state = app.state::<AppState>();
                    let mut enabled = state.input_sharing_enabled.lock().unwrap();
                    *enabled = !*enabled;
                    let tray = app.tray_handle();
                    let item = tray.get_item("input_toggle");
                    if *enabled {
                        let _ = item.set_title("键鼠共享: 开启");
                    } else {
                        let _ = item.set_title("键鼠共享");
                    }
                }
                _ => {}
            }),
            _ => {}
        })
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // Don't close the window, just hide it (background running).
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
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
            get_policy,
            set_device_policy,
            set_type_policy,
            get_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_connection_status(state: tauri::State<AppState>) -> String {
    state.connection_status.lock().unwrap().clone()
}

#[tauri::command]
fn get_clipboard_history(_state: tauri::State<AppState>, _limit: usize) -> serde_json::Value {
    serde_json::json!({ "items": [], "message": "连接服务器后显示剪贴板历史" })
}

#[tauri::command]
fn get_devices(_state: tauri::State<AppState>) -> serde_json::Value {
    serde_json::json!({ "devices": [], "message": "连接服务器后显示设备列表" })
}

#[tauri::command]
fn toggle_sync_pause(state: tauri::State<AppState>) -> bool {
    let mut paused = state.sync_paused.lock().unwrap();
    *paused = !*paused;
    *paused
}

#[tauri::command]
fn get_sync_paused(state: tauri::State<AppState>) -> bool {
    *state.sync_paused.lock().unwrap()
}

#[tauri::command]
fn toggle_input_sharing(state: tauri::State<AppState>) -> bool {
    let mut enabled = state.input_sharing_enabled.lock().unwrap();
    *enabled = !*enabled;
    *enabled
}

#[tauri::command]
fn get_input_sharing_enabled(state: tauri::State<AppState>) -> bool {
    *state.input_sharing_enabled.lock().unwrap()
}

#[tauri::command]
fn get_server_url(state: tauri::State<AppState>) -> String {
    state.server_url.lock().unwrap().clone()
}

#[tauri::command]
fn set_server_url(state: tauri::State<AppState>, url: String) -> bool {
    let mut server = state.server_url.lock().unwrap();
    if url.is_empty() || url.starts_with("http://") || url.starts_with("https://") {
        *server = url;
        true
    } else {
        false
    }
}

#[tauri::command]
fn get_policy(state: tauri::State<AppState>) -> serde_json::Value {
    let policy = state.policy.lock().unwrap();
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
        let mut policy = state.policy.lock().unwrap();
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
    let mut policy = state.policy.lock().unwrap();
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
