//! OMO (OpenCode Multi-model Orchestrator) operations.
//!
//! The upstream `services::omo` module is private. We re-implement the
//! minimum set of operations on top of the database + filesystem so the
//! frontend's "OMO providers" UI works.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

const OMO_FILE: &str = "omo.json";
const OMO_SLIM_FILE: &str = "omo-slim.json";

fn opencode_dir() -> std::path::PathBuf {
    if let Some(override_dir) = std::env::var_os("OPENCODE_DIR") {
        return std::path::PathBuf::from(override_dir);
    }
    dirs::home_dir()
        .map(|h| h.join(".config").join("opencode"))
        .unwrap_or_else(|| std::path::PathBuf::from(".config/opencode"))
}

fn omo_file(slim: bool) -> std::path::PathBuf {
    opencode_dir().join(if slim { OMO_SLIM_FILE } else { OMO_FILE })
}

fn read_omo(slim: bool) -> Value {
    let path = omo_file(slim);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Value::Null;
    };
    serde_json::from_str(&raw).unwrap_or(Value::Null)
}

pub async fn read_omo_local_file() -> Result<Value> {
    Ok(read_omo(false))
}

pub async fn read_omo_slim_local_file() -> Result<Value> {
    Ok(read_omo(true))
}

pub async fn get_current_omo_provider_id(ctx: &Arc<AppContext>) -> Result<Value> {
    let provider = ctx
        .state
        .db
        .get_current_omo_provider("opencode", "omo")
        .map_err(ApiError::from)?;
    Ok(Value::String(provider.map(|p| p.id).unwrap_or_default()))
}

pub async fn get_current_omo_slim_provider_id(ctx: &Arc<AppContext>) -> Result<Value> {
    let provider = ctx
        .state
        .db
        .get_current_omo_provider("opencode", "omo-slim")
        .map_err(ApiError::from)?;
    Ok(Value::String(provider.map(|p| p.id).unwrap_or_default()))
}

pub async fn disable_current_omo(ctx: &Arc<AppContext>) -> Result<Value> {
    let providers = ctx
        .state
        .db
        .get_all_providers("opencode")
        .map_err(ApiError::from)?;
    for (id, p) in &providers {
        if p.category.as_deref() == Some("omo") {
            ctx.state
                .db
                .clear_omo_provider_current("opencode", id, "omo")
                .map_err(ApiError::from)?;
        }
    }
    let _ = std::fs::remove_file(omo_file(false));
    Ok(Value::Null)
}

pub async fn disable_current_omo_slim(ctx: &Arc<AppContext>) -> Result<Value> {
    let providers = ctx
        .state
        .db
        .get_all_providers("opencode")
        .map_err(ApiError::from)?;
    for (id, p) in &providers {
        if p.category.as_deref() == Some("omo") {
            ctx.state
                .db
                .clear_omo_provider_current("opencode", id, "omo-slim")
                .map_err(ApiError::from)?;
        }
    }
    let _ = std::fs::remove_file(omo_file(true));
    Ok(Value::Null)
}

// New OMO/slim commands introduced by the upstream UI. We expose them
// with the documented JSON shapes even though the upstream
// `services::omo` types are not reachable from the dispatch layer.

pub async fn get_provider_health(_args: Value) -> Result<Value> {
    // The proxy service maintains a health snapshot; the headless server
    // reports an empty map because the live proxy is not in this binary.
    Ok(serde_json::json!({}))
}

pub async fn get_optimizer_config(_args: Value) -> Result<Value> {
    Ok(serde_json::json!({}))
}

pub async fn set_optimizer_config(args: Value) -> Result<Value> {
    let _config: Value = require_arg(&args, "config")?;
    Ok(Value::Bool(true))
}

pub async fn get_rectifier_config(_args: Value) -> Result<Value> {
    Ok(serde_json::json!({}))
}

pub async fn set_rectifier_config(args: Value) -> Result<Value> {
    let _config: Value = require_arg(&args, "config")?;
    Ok(Value::Bool(true))
}
