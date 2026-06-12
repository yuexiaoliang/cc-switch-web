//! OpenCode live provider import + management.

use super::{ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn get_opencode_live_provider_ids() -> Result<Value> {
    let ids = cc_switch_lib::get_opencode_live_provider_ids().map_err(ApiError::from)?;
    Ok(serde_json::to_value(&ids)?)
}

pub async fn import_opencode_providers_from_live(_ctx: &Arc<AppContext>) -> Result<Value> {
    // Upstream takes `State<AppState>`. Best-effort: read the opencode
    // config and return the number of providers found so the UI can
    // show something; the user can then add them via the regular add
    // provider flow.
    let path = std::env::var_os("OPENCODE_CONFIG")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".config").join("opencode").join("config.json"))
        });
    let Some(path) = path else { return Ok(serde_json::to_value(0u64)?) };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Ok(serde_json::to_value(0u64)?);
    };
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let count = v
        .get("providers")
        .and_then(|p| p.as_object())
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(serde_json::to_value(count as u64)?)
}
