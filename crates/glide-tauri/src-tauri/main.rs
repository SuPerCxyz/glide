#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use glide_core::clipboard::ClipboardKind;
use glide_core::policy::Policy;
use std::sync::Mutex;

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
        .manage(state)
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
    serde_json::json!({ "items": [], "message": "Connect to server to view history" })
}

#[tauri::command]
fn get_devices(_state: tauri::State<AppState>) -> serde_json::Value {
    serde_json::json!({ "devices": [], "message": "Connect to server to view devices" })
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
