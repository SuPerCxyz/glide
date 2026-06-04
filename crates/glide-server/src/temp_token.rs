use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use tracing::warn;

use glide_core::error::GlideError;

/// Operations that a temporary token can authorize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TempTokenOperation {
    Copy,
    Paste,
    History,
    Devices,
}

impl TempTokenOperation {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "copy" => Some(Self::Copy),
            "paste" => Some(Self::Paste),
            "history" => Some(Self::History),
            "devices" => Some(Self::Devices),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::History => "history",
            Self::Devices => "devices",
        }
    }
}

/// Validated temporary token.
#[derive(Debug, Clone)]
pub struct ValidatedToken {
    pub token: String,
    pub allowed_ops: Vec<TempTokenOperation>,
    pub max_item_size: u64,
}

/// Check whether a temporary token is valid and increment its use count.
pub async fn validate_and_use_token(
    pool: &Pool<Sqlite>,
    token: &str,
    operation: TempTokenOperation,
    item_size: Option<u64>,
) -> Result<ValidatedToken, GlideError> {
    // Fetch the token record.
    let row = sqlx::query_as::<_, (i64, String, i64, i64, i64, i64, i64, String, i64, bool)>(
        r#"SELECT id, token, expires_at, ttl_secs, max_uses, use_count, max_item_size, allowed_operations, created_at, revoked
           FROM temp_tokens WHERE token = ?"#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .map_err(|e| GlideError::ConnectionError(e.to_string()))?;

    let row = match row {
        Some(r) => r,
        None => return Err(GlideError::InvalidToken("token not found".to_string())),
    };

    let (_id, _token, expires_at, _ttl_secs, max_uses, use_count, max_item_size, allowed_ops, _created_at, revoked) = row;

    // Check revoked.
    if revoked {
        return Err(GlideError::InvalidToken("token has been revoked".to_string()));
    }

    // Check expiry.
    let now = Utc::now().timestamp_millis();
    if now > expires_at {
        return Err(GlideError::TokenExpired);
    }

    // Check max uses.
    if use_count >= max_uses {
        return Err(GlideError::TokenMaxUsesExceeded);
    }

    // Parse allowed operations.
    let ops: Vec<String> = serde_json::from_str(&allowed_ops)
        .unwrap_or_else(|_| vec!["copy".to_string(), "paste".to_string()]);

    let allowed_ops: Vec<TempTokenOperation> = ops
        .into_iter()
        .filter_map(|s| TempTokenOperation::from_str(&s))
        .collect();

    // Check if the requested operation is allowed.
    if !allowed_ops.iter().any(|op| match (op, &operation) {
        (TempTokenOperation::Copy, TempTokenOperation::Copy) => true,
        (TempTokenOperation::Paste, TempTokenOperation::Paste) => true,
        (TempTokenOperation::History, TempTokenOperation::History) => true,
        (TempTokenOperation::Devices, TempTokenOperation::Devices) => true,
        _ => false,
    }) {
        return Err(GlideError::TokenOperationNotAllowed(
            operation.as_str().to_string(),
        ));
    }

    // Check item size limit.
    let max_item_size = max_item_size as u64;
    if let Some(size) = item_size {
        if size > max_item_size {
            return Err(GlideError::ItemTooLarge {
                size,
                limit: max_item_size,
            });
        }
    }

    // Increment use count.
    sqlx::query("UPDATE temp_tokens SET use_count = use_count + 1 WHERE token = ?")
        .bind(token)
        .execute(pool)
        .await
        .map_err(|e| GlideError::ConnectionError(e.to_string()))?;

    Ok(ValidatedToken {
        token: token.to_string(),
        allowed_ops,
        max_item_size,
    })
}

/// Create a new temporary token.
pub async fn create_temp_token(
    pool: &Pool<Sqlite>,
    ttl_secs: u64,
    max_uses: i64,
    allowed_operations: Vec<String>,
    max_item_size: i64,
) -> Result<String, GlideError> {
    let token = generate_token();
    let now = Utc::now().timestamp_millis();
    let expires_at = now + (ttl_secs as i64 * 1000);
    let allowed_ops_json = serde_json::to_string(&allowed_operations)
        .map_err(|e| GlideError::SerializationError(e.to_string()))?;

    sqlx::query(
        r#"INSERT INTO temp_tokens (token, expires_at, ttl_secs, max_uses, allowed_operations, max_item_size)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&token)
    .bind(expires_at)
    .bind(ttl_secs as i64)
    .bind(max_uses)
    .bind(allowed_ops_json)
    .bind(max_item_size)
    .execute(pool)
    .await
    .map_err(|e| GlideError::ConnectionError(e.to_string()))?;

    Ok(token)
}

/// Remove expired tokens.
pub async fn cleanup_expired_tokens(pool: &Pool<Sqlite>) -> Result<i64> {
    let now = Utc::now().timestamp_millis();
    let result = sqlx::query("DELETE FROM temp_tokens WHERE expires_at < ? OR revoked = TRUE")
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| GlideError::ConnectionError(e.to_string()))?;

    Ok(result.rows_affected() as i64)
}

fn generate_token() -> String {
    use std::time::SystemTime;
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Simple token: hex timestamp + random-ish suffix.
    format!("glide_tmp_{:x}", ts)
}
