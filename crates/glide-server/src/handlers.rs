use anyhow::Result;
use axum::{
    extract::ws::WebSocketUpgrade,
    extract::{Json, Path, Query, State},
    routing::{get, post},
    Router,
};
use serde_json;
use sqlx::Row;

use crate::state::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/", get(admin_page))
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/tokens/create", post(create_token))
        .route("/api/v1/devices/register", post(device_register))
        .route("/api/v1/devices", get(list_devices))
        .route("/api/v1/tokens/validate", post(validate_token))
        .route("/api/v1/clipboard/history", get(clipboard_history))
        .route("/api/v1/payload/upload", post(payload_upload))
        .route("/api/v1/payload/:payload_id", get(payload_download))
        .route("/api/v1/cleanup", post(trigger_cleanup))
        // WebSocket endpoint for sync.
        .route("/ws/sync", get(ws_handler))
        // WebSocket endpoint for input relay.
        .route("/ws/input", get(input_ws_handler))
        // Pairing endpoints.
        .route("/api/v1/pairing/initiate", post(pairing_initiate))
        .route("/api/v1/pairing/confirm", post(pairing_confirm))
        // Trust management.
        .route("/api/v1/devices/:device_id/untrust", post(device_untrust))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn admin_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../static/index.html"))
}

/// Login endpoint: authenticate with username/password.
/// Returns a session token on success.
async fn login(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let username = req
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "username required"})),
            )
        })?;
    let password = req
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "password required"})),
            )
        })?;

    // Check environment variable credentials.
    let admin_user = std::env::var("GLIDE_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_pass = std::env::var("GLIDE_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    if username == admin_user && password == admin_pass {
        // Generate session token.
        let token = format!("session_{}", uuid::Uuid::new_v4());
        Ok(Json(serde_json::json!({
            "status": "ok",
            "token": token,
            "username": username,
        })))
    } else {
        Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid username or password"})),
        ))
    }
}

/// Create a temporary token (requires admin auth).
async fn create_token(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let ttl_secs = req.get("ttl_secs").and_then(|v| v.as_u64()).unwrap_or(3600);
    let max_uses = req.get("max_uses").and_then(|v| v.as_i64()).unwrap_or(10);
    let max_item_size = req
        .get("max_item_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(10_485_760);
    let allowed = req
        .get("allowed_operations")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["copy".to_string(), "paste".to_string()]);

    match crate::temp_token::create_temp_token(
        &state.db,
        ttl_secs,
        max_uses,
        allowed,
        max_item_size,
    )
    .await
    {
        Ok(token) => Ok(Json(serde_json::json!({
            "status": "ok",
            "token": token,
            "ttl_secs": ttl_secs,
            "max_uses": max_uses,
        }))),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn device_register(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let device_id = req
        .get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "device_id required"})),
            )
        })?;

    let name = req
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed");
    let platform = req
        .get("platform")
        .and_then(|v| v.as_str())
        .unwrap_or("linux");
    let trusted = req.get("trusted").and_then(|v| v.as_bool()).unwrap_or(true);
    let public_key = req.get("public_key_fingerprint").and_then(|v| v.as_str());
    let registration_token = std::env::var("GLIDE_REGISTRATION_TOKEN").ok();

    // Verify registration token if configured.
    if let Some(ref token) = registration_token {
        let provided = req
            .get("registration_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "registration_token required"})),
                )
            })?;
        if provided != token {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid registration_token"})),
            ));
        }
    }

    // Insert or update device.
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"INSERT INTO devices (device_id, name, platform, trusted, public_key_fingerprint, created_at)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(device_id) DO UPDATE SET name=?, platform=?, trusted=?, last_seen_at=?,
             public_key_fingerprint=COALESCE(?, public_key_fingerprint)"#,
    )
    .bind(device_id)
    .bind(name)
    .bind(platform)
    .bind(trusted)
    .bind(public_key)
    .bind(now)
    .bind(name)
    .bind(platform)
    .bind(trusted)
    .bind(now)
    .bind(public_key)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "registered",
        "device_id": device_id,
    })))
}

/// Generate a pairing code for device pairing.
/// Code is 6 chars, expires in 5 minutes.
async fn pairing_initiate(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let device_id = req.get("device_id").and_then(|v| v.as_str()).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "device_id required"})),
        )
    })?;

    // Generate 6-char uppercase code from UUID
    let code: String = uuid::Uuid::new_v4().to_string()[..6].to_uppercase();

    let now = chrono::Utc::now().timestamp_millis();
    let expires_at = now + 300_000; // 5 minutes

    sqlx::query(
        "INSERT INTO pairing_codes (code, initiator_device_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&code)
    .bind(device_id)
    .bind(now)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "code": code,
        "expires_at": expires_at,
    })))
}

/// Confirm a pairing code. Sets both devices as trusted with paired_at.
async fn pairing_confirm(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let code = req.get("code").and_then(|v| v.as_str()).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "code required"})),
        )
    })?;

    let device_id = req.get("device_id").and_then(|v| v.as_str()).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "device_id required"})),
        )
    })?;

    // Look up the pairing code
    let row = sqlx::query("SELECT initiator_device_id, expires_at, used FROM pairing_codes WHERE code = ?")
        .bind(code)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let (initiator_id, expires_at, used) = match row {
        Some(r) => (
            r.get::<String, _>("initiator_device_id"),
            r.get::<i64, _>("expires_at"),
            r.get::<bool, _>("used"),
        ),
        None => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "invalid or expired pairing code"})),
            ));
        }
    };
    let now = chrono::Utc::now().timestamp_millis();

    if used {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "pairing code already used"})),
        ));
    }

    if now > expires_at {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "pairing code has expired"})),
        ));
    }

    // Check that the confirming device is not the initiator
    if device_id == initiator_id {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "cannot pair with yourself"})),
        ));
    }

    // Mark pairing code as used
    sqlx::query("UPDATE pairing_codes SET used = TRUE, confirmed_device_id = ? WHERE code = ?")
        .bind(device_id)
        .bind(code)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    // Set trusted = true and paired_at for BOTH devices
    sqlx::query("UPDATE devices SET trusted = TRUE, paired_at = ? WHERE device_id = ?")
        .bind(now)
        .bind(initiator_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    sqlx::query("UPDATE devices SET trusted = TRUE, paired_at = ? WHERE device_id = ?")
        .bind(now)
        .bind(device_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "paired": true,
        "paired_at": now,
    })))
}

/// Revoke trust for a device.
async fn device_untrust(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    sqlx::query("UPDATE devices SET trusted = FALSE WHERE device_id = ?")
        .bind(&device_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "trusted": false,
        "device_id": device_id,
    })))
}

async fn list_devices(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let rows = sqlx::query("SELECT device_id, name, platform, trusted, lan_address, last_seen_at, created_at, paired_at FROM devices ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await;

    let devices = match rows {
        Ok(rows) => rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "device_id": r.get::<String, _>("device_id"),
                    "name": r.get::<String, _>("name"),
                    "platform": r.get::<String, _>("platform"),
                    "trusted": r.get::<bool, _>("trusted"),
                    "lan_address": r.get::<Option<String>, _>("lan_address"),
                    "last_seen_at": r.get::<Option<i64>, _>("last_seen_at"),
                    "created_at": r.get::<i64, _>("created_at"),
                    "paired_at": r.get::<Option<i64>, _>("paired_at"),
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    Json(serde_json::json!({ "devices": devices }))
}

async fn validate_token(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = req.get("token").and_then(|v| v.as_str()).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "token required"})),
        )
    })?;

    let operation_str = req
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("copy");
    let operation =
        crate::temp_token::TempTokenOperation::from_str(operation_str).ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid operation"})),
            )
        })?;

    let item_size = req.get("item_size").and_then(|v| v.as_u64());

    match crate::temp_token::validate_and_use_token(&state.db, token, operation, item_size).await {
        Ok(validated) => Ok(Json(serde_json::json!({
            "valid": true,
            "allowed_operations": validated.allowed_ops.iter().map(|o| o.as_str()).collect::<Vec<_>>(),
            "max_item_size": validated.max_item_size,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "valid": false,
            "error": e.to_string(),
        }))),
    }
}

async fn clipboard_history(
    State(state): State<ServerState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(50);
    let offset = params
        .get("offset")
        .and_then(|o| o.parse::<i64>().ok())
        .unwrap_or(0);
    let device_id = params.get("device_id").cloned();

    let query = match &device_id {
        Some(did) => format!(
            "SELECT item_id, source_device_id, source_session_type, kind, representations, payload_refs, size, created_at, checksum, delivery_policy FROM clipboard_items WHERE source_device_id != '{}' ORDER BY created_at DESC LIMIT {} OFFSET {}",
            did, limit, offset
        ),
        None => format!(
            "SELECT item_id, source_device_id, source_session_type, kind, representations, payload_refs, size, created_at, checksum, delivery_policy FROM clipboard_items ORDER BY created_at DESC LIMIT {} OFFSET {}",
            limit, offset
        ),
    };

    let rows = sqlx::query(&query)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let items = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "item_id": r.get::<String, _>("item_id"),
                "source_device_id": r.get::<String, _>("source_device_id"),
                "source_session_type": api_session_type(&r.get::<String, _>("source_session_type")),
                "kind": api_clipboard_kind(&r.get::<String, _>("kind")),
                "representations": serde_json::from_str::<serde_json::Value>(&r.get::<String, _>("representations")).unwrap_or(serde_json::json!([])),
                "payload_refs": serde_json::from_str::<serde_json::Value>(&r.get::<String, _>("payload_refs")).unwrap_or(serde_json::json!([])),
                "size": r.get::<i64, _>("size"),
                "created_at": r.get::<i64, _>("created_at"),
                "checksum": r.get::<String, _>("checksum"),
                "delivery_policy": serde_json::from_str::<serde_json::Value>(&r.get::<String, _>("delivery_policy")).unwrap_or(serde_json::json!({"type":"broadcast"})),
            })
        })
        .collect::<Vec<_>>();

    Json(serde_json::json!({ "items": items, "limit": limit, "offset": offset }))
}

fn api_clipboard_kind(kind: &str) -> &'static str {
    match kind {
        "text" => "Text",
        "image" => "Image",
        "file" => "File",
        _ => "Text",
    }
}

fn api_session_type(session_type: &str) -> &'static str {
    match session_type {
        "persistent" => "Persistent",
        "temporary" => "Temporary",
        _ => "Persistent",
    }
}

async fn payload_upload(
    State(state): State<ServerState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    use sha2::{Digest, Sha256};
    use std::io::Write;

    let mut payload_id: Option<String> = None;
    let mut payload_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })? {
        if let Some(name) = field.name() {
            match name {
                "payload_id" => {
                    if let Ok(text) = field.text().await {
                        payload_id = Some(text);
                    }
                }
                "data" => {
                    if let Some(bytes) = field.bytes().await.ok() {
                        payload_bytes = Some(bytes.to_vec());
                    }
                }
                _ => {}
            }
        }
    }

    let payload_id = payload_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let payload_bytes = payload_bytes.unwrap_or_default();
    let total_size = payload_bytes.len() as u64;
    let mut checksum_hasher = Sha256::new();
    checksum_hasher.update(&payload_bytes);
    let checksum = format!("{:x}", checksum_hasher.finalize());

    let dir = format!("{}/payloads", state.data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let file_path = format!("{}/payloads/{}", state.data_dir, payload_id);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&file_path)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    file.write_all(&payload_bytes).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    sqlx::query(
        "INSERT OR REPLACE INTO payloads (payload_id, file_path, size, checksum) VALUES (?, ?, ?, ?)",
    )
    .bind(&payload_id)
    .bind(&file_path)
    .bind(total_size as i64)
    .bind(&checksum)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(serde_json::json!({
        "payload_id": payload_id,
        "size": total_size,
        "checksum": checksum,
    })))
}

#[cfg(test)]
mod tests {
    use super::{api_clipboard_kind, api_session_type};

    #[test]
    fn history_api_uses_core_enum_variant_names() {
        assert_eq!("Text", api_clipboard_kind("text"));
        assert_eq!("Image", api_clipboard_kind("image"));
        assert_eq!("File", api_clipboard_kind("file"));
        assert_eq!("Persistent", api_session_type("persistent"));
        assert_eq!("Temporary", api_session_type("temporary"));
    }
}

async fn payload_download(
    State(state): State<ServerState>,
    Path(payload_id): Path<String>,
) -> Result<axum::response::Response, (axum::http::StatusCode, Json<serde_json::Value>)> {
    use axum::body::Body;
    use axum::http::{header, Response, StatusCode};

    let dir = format!("{}/payloads", state.data_dir);
    let path = format!("{}/{}", dir, payload_id);

    // Security: reject path traversal.
    if payload_id.contains('/') || payload_id.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid payload_id"})),
        ));
    }

    let data = std::fs::read(&path).map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "payload not found"})),
        )
    })?;

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, data.len())
        .body(Body::from(data))
        .unwrap();

    Ok(resp)
}

async fn trigger_cleanup(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match crate::cleanup::run_cleanup(&state.db).await {
        Ok(result) => Json(serde_json::json!({
            "status": "ok",
            "items_deleted": result.items_deleted,
            "bytes_freed": result.bytes_freed,
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        })),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(state): State<ServerState>,
) -> axum::response::Response {
    let device_id = params.get("device_id").cloned().unwrap_or_default();
    ws.on_upgrade(move |socket| crate::ws::handle_ws(socket, state, device_id))
}

async fn input_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(state): State<ServerState>,
) -> axum::response::Response {
    let device_id = params.get("device_id").cloned().unwrap_or_default();
    let target_id = params.get("target_id").cloned().unwrap_or_default();
    ws.on_upgrade(move |socket| {
        crate::input_relay::handle_input_ws(socket, state, device_id, target_id)
    })
}

// Search is already available via the /api/v1/clipboard/history endpoint
// by adding ?q=<search_term> parameter
