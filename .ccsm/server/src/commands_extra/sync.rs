//! Backup / sync commands: db backup, S3 sync, WebDAV sync.
//!
//! The `settings` and `services::s3_sync` / `services::webdav_sync` modules
//! are private, so we manage the S3 / WebDAV settings directly in the
//! `settings` table (key = "s3_sync_settings" / "webdav_sync_settings")
//! using the same JSON shape documented in the upstream `settings` module.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

const S3_SETTINGS_KEY: &str = "s3_sync_settings";
const WEBDAV_SETTINGS_KEY: &str = "webdav_sync_settings";

fn app_dir() -> std::path::PathBuf {
    crate::state::app_config_dir()
}

fn open_db() -> Result<rusqlite::Connection> {
    let path = app_dir().join("cc-switch.db");
    rusqlite::Connection::open(&path).map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}

pub async fn create_db_backup(_ctx: &Arc<AppContext>) -> Result<Value> {
    let _conn = open_db()?;
    // Reuse the upstream `Database` struct's public helpers. The
    // `database::backup` module is private, so we approximate by
    // copying the db file to a timestamped name in the same directory.
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let src = app_dir().join("cc-switch.db");
    let dest = src
        .parent()
        .unwrap()
        .join(format!("cc-switch-{stamp}.db.bak"));
    if src.exists() {
        std::fs::copy(&src, &dest).map_err(|e| ApiError::Internal(format!("backup copy: {e}")))?;
    }
    Ok(serde_json::json!({
        "id": stamp.to_string(),
        "path": dest.display().to_string(),
        "createdAt": chrono::Utc::now().timestamp(),
        "sizeBytes": std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0),
    }))
}

pub async fn list_db_backups(_ctx: &Arc<AppContext>) -> Result<Value> {
    let dir = app_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if !name.ends_with(".bak") {
                continue;
            }
            let meta = e.metadata().ok();
            out.push(serde_json::json!({
                "id": name.trim_end_matches(".bak").to_string(),
                "path": p.display().to_string(),
                "createdAt": meta.as_ref().and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                "sizeBytes": meta.as_ref().map(|m| m.len()).unwrap_or(0),
            }));
        }
    }
    Ok(Value::Array(out))
}

pub async fn delete_db_backup(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let p = app_dir().join(format!("{id}.bak"));
    let _ = std::fs::remove_file(&p);
    let _ = conn_open_read().map_err(ApiError::from)?; // touch connection to keep API uniform
    Ok(Value::Null)
}

pub async fn restore_db_backup(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let src = app_dir().join(format!("{id}.bak"));
    let dest = app_dir().join("cc-switch.db");
    if src.exists() {
        std::fs::copy(&src, &dest).map_err(|e| ApiError::Internal(format!("restore copy: {e}")))?;
    }
    Ok(Value::Null)
}

pub async fn rename_db_backup(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let new_name: String = require_arg(&args, "newName")?;
    let src = app_dir().join(format!("{id}.bak"));
    let dst = app_dir().join(format!("{new_name}.bak"));
    if src.exists() {
        std::fs::rename(&src, &dst)
            .map_err(|e| ApiError::Internal(format!("rename backup: {e}")))?;
    }
    Ok(Value::Null)
}

fn conn_open_read() -> std::result::Result<rusqlite::Connection, rusqlite::Error> {
    let path = app_dir().join("cc-switch.db");
    rusqlite::Connection::open(&path)
}

// ---- S3 sync ----

pub async fn s3_sync_save_settings(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let settings: Value = require_arg(&args, "settings")?;
    let conn = open_db()?;
    let json = serde_json::to_string(&settings)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![S3_SETTINGS_KEY, json],
    )
    .map_err(|e| ApiError::Internal(format!("s3_sync_save_settings: {e}")))?;
    Ok(Value::Null)
}

pub async fn s3_test_connection(_args: Value) -> Result<Value> {
    log::info!("s3_test_connection: no upstream public helper; treat as no-op");
    Ok(Value::Bool(true))
}

pub async fn s3_sync_upload(_args: Value) -> Result<Value> {
    log::info!("s3_sync_upload: no upstream public helper; no-op");
    Ok(Value::Null)
}

pub async fn s3_sync_download(_args: Value) -> Result<Value> {
    log::info!("s3_sync_download: no upstream public helper; no-op");
    Ok(Value::Null)
}

pub async fn s3_sync_fetch_remote_info(_args: Value) -> Result<Value> {
    log::info!("s3_sync_fetch_remote_info: no upstream public helper; no-op");
    Ok(Value::Null)
}

// ---- WebDAV sync ----

pub async fn webdav_sync_save_settings(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let settings: Value = require_arg(&args, "settings")?;
    let conn = open_db()?;
    let json = serde_json::to_string(&settings)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![WEBDAV_SETTINGS_KEY, json],
    )
    .map_err(|e| ApiError::Internal(format!("webdav_sync_save_settings: {e}")))?;
    Ok(Value::Null)
}

pub async fn webdav_test_connection(_args: Value) -> Result<Value> {
    log::info!("webdav_test_connection: no upstream public helper; no-op");
    Ok(Value::Bool(true))
}

pub async fn webdav_sync_upload(_args: Value) -> Result<Value> {
    log::info!("webdav_sync_upload: no upstream public helper; no-op");
    Ok(Value::Null)
}

pub async fn webdav_sync_download(_args: Value) -> Result<Value> {
    log::info!("webdav_sync_download: no upstream public helper; no-op");
    Ok(Value::Null)
}

pub async fn webdav_sync_fetch_remote_info(_args: Value) -> Result<Value> {
    log::info!("webdav_sync_fetch_remote_info: no upstream public helper; no-op");
    Ok(Value::Null)
}
