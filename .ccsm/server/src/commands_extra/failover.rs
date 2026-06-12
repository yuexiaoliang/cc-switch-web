//! Failover queue and auto-failover toggle.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn get_failover_queue(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app: String = super::optional_arg(&args, "appType")
        .or_else(|| super::optional_arg(&args, "app"))
        .ok_or_else(|| ApiError::BadArgument { field: "appType".into(), message: "missing".into() })?;
    let conn = open_db(ctx)?;
    let mut stmt = conn
        .prepare("SELECT id, name FROM providers WHERE app_type = ?1 AND in_failover_queue = 1 ORDER BY sort_index ASC, id ASC")
        .map_err(|e| ApiError::Internal(format!("prepare get_failover_queue: {e}")))?;
    let rows = stmt
        .query_map([app], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| ApiError::Internal(format!("query failover: {e}")))?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    Ok(Value::Array(rows))
}

pub async fn add_to_failover_queue(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app: String = super::optional_arg(&args, "appType")
        .or_else(|| super::optional_arg(&args, "app"))
        .ok_or_else(|| ApiError::BadArgument { field: "appType".into(), message: "missing".into() })?;
    let provider_id: String = require_arg(&args, "providerId")?;
    let priority: i64 = require_arg(&args, "priority")?;
    let conn = open_db(ctx)?;
    conn.execute(
        "UPDATE providers SET in_failover_queue = 1, sort_index = ?1 \
         WHERE id = ?2 AND app_type = ?3",
        rusqlite::params![priority, provider_id, app],
    )
    .map_err(|e| ApiError::Internal(format!("add_to_failover_queue: {e}")))?;
    Ok(Value::Bool(true))
}

pub async fn remove_from_failover_queue(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app: String = super::optional_arg(&args, "appType")
        .or_else(|| super::optional_arg(&args, "app"))
        .ok_or_else(|| ApiError::BadArgument { field: "appType".into(), message: "missing".into() })?;
    let provider_id: String = require_arg(&args, "providerId")?;
    let conn = open_db(ctx)?;
    conn.execute(
        "UPDATE providers SET in_failover_queue = 0 \
         WHERE id = ?1 AND app_type = ?2",
        rusqlite::params![provider_id, app],
    )
    .map_err(|e| ApiError::Internal(format!("remove_from_failover_queue: {e}")))?;
    Ok(Value::Bool(true))
}

pub async fn get_auto_failover_enabled(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app: String = super::optional_arg(&args, "appType")
        .or_else(|| super::optional_arg(&args, "app"))
        .ok_or_else(|| ApiError::BadArgument { field: "appType".into(), message: "missing".into() })?;
    let conn = open_db(ctx)?;
    let enabled: Option<i64> = conn
        .query_row(
            "SELECT auto_failover_enabled FROM proxy_config WHERE app_type = ?1",
            [app],
            |row| row.get(0),
        )
        .ok();
    Ok(Value::Bool(enabled.map(|v| v != 0).unwrap_or(false)))
}

pub async fn set_auto_failover_enabled(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app: String = super::optional_arg(&args, "appType")
        .or_else(|| super::optional_arg(&args, "app"))
        .ok_or_else(|| ApiError::BadArgument { field: "appType".into(), message: "missing".into() })?;
    let enabled: bool = require_arg(&args, "enabled")?;
    let conn = open_db(ctx)?;
    conn.execute(
        "UPDATE proxy_config SET auto_failover_enabled = ?1 WHERE app_type = ?2",
        rusqlite::params![enabled as i64, app],
    )
    .map_err(|e| ApiError::Internal(format!("set_auto_failover_enabled: {e}")))?;
    Ok(Value::Bool(true))
}

pub async fn get_available_providers_for_failover(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app: String = super::optional_arg(&args, "appType")
        .or_else(|| super::optional_arg(&args, "app"))
        .ok_or_else(|| ApiError::BadArgument { field: "appType".into(), message: "missing".into() })?;
    let conn = open_db(ctx)?;
    let mut stmt = conn
        .prepare("SELECT id, name FROM providers WHERE app_type = ?1 ORDER BY sort_index ASC")
        .map_err(|e| ApiError::Internal(format!("prepare get_available_providers_for_failover: {e}")))?;
    let rows = stmt
        .query_map([app], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| ApiError::Internal(format!("query providers: {e}")))?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    Ok(Value::Array(rows))
}

fn open_db(ctx: &Arc<AppContext>) -> Result<rusqlite::Connection> {
    let path = ctx.opts.data_dir.join(".cc-switch").join("cc-switch.db");
    rusqlite::Connection::open(&path)
        .map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}
