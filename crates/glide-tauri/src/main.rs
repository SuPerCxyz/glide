#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use glide_core::clipboard::ClipboardKind;
use glide_core::policy::Policy;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// Shared app state.
struct AppState {
    connection_status: Mutex<String>,
    policy: Mutex<Policy>,
    server_url: Mutex<String>,
    sync_paused: Mutex<bool>,
    input_sharing_enabled: Mutex<bool>,
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
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .setup(|app| {
            // Build system tray menu.
            let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "暂停同步", true, None::<&str>)?;
            let input_toggle = MenuItem::with_id(app, "input_toggle", "键鼠共享", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause, &input_toggle, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Glide - 剪贴板同步")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "pause" => {
                        let state = app.state::<AppState>();
                        let mut paused = state.sync_paused.lock().unwrap();
                        *paused = !*paused;
                    }
                    "input_toggle" => {
                        let state = app.state::<AppState>();
                        let mut enabled = state.input_sharing_enabled.lock().unwrap();
                        *enabled = !*enabled;
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window instead of closing (background running).
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
