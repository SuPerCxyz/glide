use anyhow::Result;
use sqlx::{Pool, Sqlite};

/// Create a SQLite connection pool.
pub async fn create_pool(db_path: &str) -> Result<Pool<Sqlite>> {
    let url = if db_path.starts_with("sqlite:") {
        db_path.to_string()
    } else {
        format!("sqlite:{}?mode=rwc", db_path)
    };
    let pool = sqlx::SqlitePool::connect(&url).await?;
    Ok(pool)
}

/// Run database migrations.
pub async fn migrate(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            device_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            platform TEXT NOT NULL DEFAULT 'linux',
            trusted BOOLEAN NOT NULL DEFAULT FALSE,
            public_key_fingerprint TEXT,
            lan_address TEXT,
            last_seen_at INTEGER,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
        );

        CREATE TABLE IF NOT EXISTS clipboard_items (
            item_id TEXT PRIMARY KEY,
            source_device_id TEXT NOT NULL REFERENCES devices(device_id),
            source_session_type TEXT NOT NULL DEFAULT 'persistent',
            kind TEXT NOT NULL DEFAULT 'text',
            representations TEXT NOT NULL DEFAULT '[]',
            payload_refs TEXT NOT NULL DEFAULT '[]',
            size INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            checksum TEXT NOT NULL,
            delivery_policy TEXT NOT NULL DEFAULT '{"type":"broadcast"}'
        );
        CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_items(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clipboard_source ON clipboard_items(source_device_id);

        CREATE TABLE IF NOT EXISTS payloads (
            payload_id TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            size INTEGER NOT NULL,
            checksum TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            ref_count INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS temp_tokens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            expires_at INTEGER NOT NULL,
            ttl_secs INTEGER NOT NULL,
            max_uses INTEGER NOT NULL DEFAULT 1,
            use_count INTEGER NOT NULL DEFAULT 0,
            allowed_operations TEXT NOT NULL DEFAULT '["copy","paste"]',
            max_item_size INTEGER NOT NULL DEFAULT 10485760,
            revoked BOOLEAN NOT NULL DEFAULT FALSE
        );
        CREATE INDEX IF NOT EXISTS idx_temp_tokens_token ON temp_tokens(token);

        CREATE TABLE IF NOT EXISTS input_sessions (
            session_id TEXT PRIMARY KEY,
            controller_id TEXT NOT NULL REFERENCES devices(device_id),
            target_id TEXT NOT NULL REFERENCES devices(device_id),
            route TEXT NOT NULL DEFAULT 'lan_direct',
            active BOOLEAN NOT NULL DEFAULT TRUE,
            latency_ms INTEGER,
            started_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            ended_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS cleanup_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            last_run_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            items_deleted INTEGER NOT NULL DEFAULT 0,
            bytes_freed INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS pairing_codes (
            code TEXT PRIMARY KEY,
            initiator_device_id TEXT NOT NULL REFERENCES devices(device_id),
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            expires_at INTEGER NOT NULL,
            used BOOLEAN NOT NULL DEFAULT FALSE,
            confirmed_device_id TEXT
        );

        CREATE TABLE IF NOT EXISTS device_clipboard_seen (
            device_id TEXT NOT NULL REFERENCES devices(device_id),
            item_id TEXT NOT NULL REFERENCES clipboard_items(item_id),
            delivered_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
            PRIMARY KEY (device_id, item_id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query(
        "ALTER TABLE clipboard_items ADD COLUMN payload_refs TEXT NOT NULL DEFAULT '[]'",
    )
    .execute(pool)
    .await;

    // Add paired_at column to devices table if not present
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN paired_at INTEGER")
        .execute(pool)
        .await;

    Ok(())
}
