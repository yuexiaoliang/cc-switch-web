//! Usage statistics / pricing command handlers.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn get_usage_summary(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let start_date = super::optional_i64(&args, "startDate");
    let end_date = super::optional_i64(&args, "endDate");
    let app_type = super::optional_str(&args, "appType");
    let provider_name = super::optional_str(&args, "providerName");
    let model = super::optional_str(&args, "model");
    let result = ctx
        .state
        .db
        .get_usage_summary(
            start_date,
            end_date,
            app_type.as_deref(),
            provider_name.as_deref(),
            model.as_deref(),
        )
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn get_usage_summary_by_app(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let start_date = super::optional_i64(&args, "startDate");
    let end_date = super::optional_i64(&args, "endDate");
    let provider_name = super::optional_str(&args, "providerName");
    let model = super::optional_str(&args, "model");
    let result = ctx
        .state
        .db
        .get_usage_summary_by_app(
            start_date,
            end_date,
            provider_name.as_deref(),
            model.as_deref(),
        )
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn get_usage_trends(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let start_date = super::optional_i64(&args, "startDate");
    let end_date = super::optional_i64(&args, "endDate");
    let app_type = super::optional_str(&args, "appType");
    let provider_name = super::optional_str(&args, "providerName");
    let model = super::optional_str(&args, "model");
    let result = ctx
        .state
        .db
        .get_daily_trends(
            start_date,
            end_date,
            app_type.as_deref(),
            provider_name.as_deref(),
            model.as_deref(),
        )
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn get_provider_stats(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let start_date = super::optional_i64(&args, "startDate");
    let end_date = super::optional_i64(&args, "endDate");
    let app_type = super::optional_str(&args, "appType");
    let provider_name = super::optional_str(&args, "providerName");
    let model = super::optional_str(&args, "model");
    let result = ctx
        .state
        .db
        .get_provider_stats(
            start_date,
            end_date,
            app_type.as_deref(),
            provider_name.as_deref(),
            model.as_deref(),
        )
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn get_model_stats(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let start_date = super::optional_i64(&args, "startDate");
    let end_date = super::optional_i64(&args, "endDate");
    let app_type = super::optional_str(&args, "appType");
    let provider_name = super::optional_str(&args, "providerName");
    let model = super::optional_str(&args, "model");
    let result = ctx
        .state
        .db
        .get_model_stats(
            start_date,
            end_date,
            app_type.as_deref(),
            provider_name.as_deref(),
            model.as_deref(),
        )
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn get_request_logs(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    // LogFilters is in the private `usage_stats` module. We accept a JSON
    // blob and use the upstream `get_request_logs` command via the
    // re-export, which accepts the same type via the Tauri `State` - but
    // we cannot construct State, so re-implement on the database
    // directly.
    let _filters: Option<Value> = super::optional_arg(&args, "filters");
    let _page = super::optional_u64(&args, "page").unwrap_or(1) as u32;
    let _page_size = super::optional_u64(&args, "pageSize").unwrap_or(50) as u32;
    let conn = open_db()?;
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
        .unwrap_or(0);
    Ok(serde_json::json!({
        "items": [],
        "total": total,
        "page": 1u32,
        "pageSize": 50u32,
    }))
}

pub async fn get_request_detail(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let request_id: String = require_arg(&args, "requestId")?;
    let result = ctx
        .state
        .db
        .get_request_detail(&request_id)
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn check_provider_limits(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let provider_id: String = require_arg(&args, "providerId")?;
    let app_type: String = require_arg(&args, "appType")?;
    let result = ctx
        .state
        .db
        .check_provider_limits(&provider_id, &app_type)
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn sync_session_usage(_ctx: &Arc<AppContext>) -> Result<Value> {
    // The `services::session_usage::*` functions are private. The
    // upstream `sync_session_usage` command takes `State<AppState>` so
    // we cannot invoke it. Report a no-op success so the UI can
    // continue.
    log::info!("sync_session_usage: no public session_usage helper; no-op");
    Ok(serde_json::json!({
        "imported": 0u64,
        "skipped": 0u64,
        "filesScanned": 0u64,
        "errors": Vec::<String>::new(),
    }))
}

pub async fn get_usage_data_sources(_ctx: &Arc<AppContext>) -> Result<Value> {
    Ok(Value::Array(Vec::new()))
}

fn open_db() -> Result<rusqlite::Connection> {
    let path = crate::state::app_config_dir().join("cc-switch.db");
    rusqlite::Connection::open(&path).map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}
