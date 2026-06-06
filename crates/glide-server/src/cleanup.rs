use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};

/// Result of a cleanup run.
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub items_deleted: i64,
    pub bytes_freed: i64,
}

/// Run retention and capacity cleanup.
pub async fn run_cleanup(pool: &Pool<Sqlite>) -> Result<CleanupResult> {
    let now = Utc::now().timestamp_millis();
    let retention_days = std::env::var("GLIDE_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(30);

    let max_storage = std::env::var("GLIDE_MAX_STORAGE_BYTES")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1_073_741_824); // 1 GB default

    // 1. Delete items older than retention period.
    let retention_cutoff = now - (retention_days * 24 * 60 * 60 * 1000);

    // Collect payload IDs of items being deleted.
    let old_payloads: Vec<(String, i64)> = sqlx::query_as(
        "SELECT p.payload_id, p.size FROM payloads p
         WHERE p.payload_id IN (
             SELECT DISTINCT json_each.value
             FROM clipboard_items ci, json_each(ci.representations)
             WHERE ci.created_at < ?
             AND json_valid(ci.representations) = 1
         )",
    )
    .bind(retention_cutoff)
    .fetch_all(pool)
    .await?;

    // Delete old clipboard items.
    let items_deleted = sqlx::query("DELETE FROM clipboard_items WHERE created_at < ?")
        .bind(retention_cutoff)
        .execute(pool)
        .await?
        .rows_affected() as i64;

    // Delete orphaned payloads.
    let mut bytes_freed: i64 = 0;
    for (pid, size) in old_payloads {
        let path = format!("{}/payloads/{}", get_data_dir(), pid);
        if std::fs::remove_file(&path).is_ok() {
            bytes_freed += size;
        }
        let _ = sqlx::query("DELETE FROM payloads WHERE payload_id = ?")
            .bind(&pid)
            .execute(pool)
            .await;
    }

    // 2. If still over capacity, delete oldest items first.
    let total_size: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM payloads")
        .fetch_one(pool)
        .await?;

    if total_size > max_storage {
        let overflow = total_size - max_storage;
        let mut freed: i64 = 0;

        let oldest_payloads: Vec<(String, i64)> =
            sqlx::query_as("SELECT p.payload_id, p.size FROM payloads p ORDER BY p.created_at ASC")
                .fetch_all(pool)
                .await?;

        for (pid, size) in oldest_payloads {
            if freed >= overflow {
                break;
            }
            let path = format!("{}/payloads/{}", get_data_dir(), pid);
            if std::fs::remove_file(&path).is_ok() {
                freed += size;
                let _ = sqlx::query("DELETE FROM payloads WHERE payload_id = ?")
                    .bind(&pid)
                    .execute(pool)
                    .await;
                // Also delete referencing clipboard items.
                let _ = sqlx::query(
                    "DELETE FROM clipboard_items WHERE item_id IN (
                        SELECT ci.item_id FROM clipboard_items ci, json_each(ci.representations)
                        WHERE json_each.value = ?
                    )",
                )
                .bind(&pid)
                .execute(pool)
                .await;
            }
        }

        bytes_freed += freed;
    }

    // 3. Clean up expired tokens.
    let tokens_deleted = crate::temp_token::cleanup_expired_tokens(pool)
        .await
        .unwrap_or(0);

    // 4. Log cleanup run.
    let _ = sqlx::query(
        "INSERT INTO cleanup_log (last_run_at, items_deleted, bytes_freed) VALUES (?, ?, ?)",
    )
    .bind(now)
    .bind(items_deleted)
    .bind(bytes_freed)
    .execute(pool)
    .await;

    Ok(CleanupResult {
        items_deleted,
        bytes_freed,
    })
}

fn get_data_dir() -> String {
    std::env::var("GLIDE_DATA_DIR").unwrap_or_else(|_| "./data".to_string())
}
