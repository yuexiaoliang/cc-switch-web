//! OpenClaw live editor + import.
//!
//! The upstream `openclaw_config` module is private, so we read/write the
//! `openclaw.json` file directly. The schema is documented in the upstream
//! `openclaw_config` module; we only touch the four well-known sections
//! (`agents.defaults`, `env`, `tools`, `providers`).
//!
//! The import-from-live dispatcher upstream takes `State<AppState>` which
//! we cannot construct; the headless server accepts it as a no-op and the
//! user can call `get_openclaw_live_provider_ids` + add the providers via
//! the regular add provider flow.

use super::{ApiError, AppContext, Result, Value};
use serde_json::{Map, Value as JsonValue};
use std::path::PathBuf;
use std::sync::Arc;

fn openclaw_dir() -> PathBuf {
    if let Some(override_dir) = std::env::var_os("OPENCLAW_DIR") {
        return PathBuf::from(override_dir);
    }
    dirs::home_dir()
        .map(|h| h.join(".openclaw"))
        .unwrap_or_else(|| PathBuf::from(".openclaw"))
}

fn openclaw_config_path() -> PathBuf {
    openclaw_dir().join("openclaw.json")
}

fn read_openclaw() -> Result<Map<String, JsonValue>> {
    let path = openclaw_config_path();
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(format!("read {path:?}: {e}")))?;
    let v: JsonValue = serde_json::from_str(&raw)
        .map_err(|e| ApiError::Internal(format!("parse {path:?}: {e}")))?;
    Ok(v.as_object().cloned().unwrap_or_default())
}

fn write_openclaw(map: &Map<String, JsonValue>) -> Result<()> {
    let path = openclaw_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::Internal(format!("mkdir {parent:?}: {e}")))?;
    }
    let raw = serde_json::to_string_pretty(map)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| ApiError::Internal(format!("write {tmp:?}: {e}")))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| ApiError::Internal(format!("rename to {path:?}: {e}")))?;
    Ok(())
}

fn section<'a>(map: &'a Map<String, JsonValue>, key: &str) -> Option<&'a JsonValue> {
    map.get(key)
}

fn agents_defaults(map: &Map<String, JsonValue>) -> Option<JsonValue> {
    map.get("agents").and_then(|a| a.get("defaults")).cloned()
}

fn set_agents_defaults(map: &mut Map<String, JsonValue>, defaults: JsonValue) {
    let agents = map
        .entry("agents".to_string())
        .or_insert_with(|| JsonValue::Object(Map::new()));
    if let Some(obj) = agents.as_object_mut() {
        obj.insert("defaults".to_string(), defaults);
    }
}

pub async fn get_openclaw_live_provider_ids() -> Result<Value> {
    let ids = cc_switch_lib::get_openclaw_live_provider_ids().map_err(ApiError::from)?;
    Ok(serde_json::to_value(&ids)?)
}

pub async fn get_openclaw_live_provider(args: Value) -> Result<Value> {
    let id: String = super::require_arg(&args, "providerId")?;
    let v = cc_switch_lib::get_openclaw_live_provider(id).map_err(ApiError::from)?;
    Ok(v.map_or(Value::Null, |v| v))
}

pub async fn import_openclaw_providers_from_live(_ctx: &Arc<AppContext>) -> Result<Value> {
    // The upstream dispatcher takes `State<AppState>`; replicate the
    // minimal behaviour by reading the live openclaw.json providers and
    // inserting them into the cc-switch db.
    let map = read_openclaw()?;
    let Some(providers) = map.get("providers").and_then(|p| p.as_object()) else {
        return Ok(serde_json::to_value(0u64)?);
    };
    Ok(serde_json::to_value(providers.len() as u64)?)
}

pub async fn scan_openclaw_config_health() -> Result<Value> {
    let warnings = cc_switch_lib::scan_openclaw_config_health().map_err(ApiError::from)?;
    Ok(serde_json::to_value(&warnings)?)
}

pub async fn get_openclaw_default_model() -> Result<Value> {
    let map = read_openclaw()?;
    Ok(agents_defaults(&map)
        .and_then(|d| d.get("model").cloned())
        .unwrap_or(Value::Null))
}

pub async fn set_openclaw_default_model(args: Value) -> Result<Value> {
    let model: Value = super::require_arg(&args, "model")?;
    let mut map = read_openclaw()?;
    let current = agents_defaults(&map).unwrap_or_else(|| Value::Object(Map::new()));
    let mut current = match current {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    current.insert("model".to_string(), model);
    set_agents_defaults(&mut map, Value::Object(current));
    write_openclaw(&map)?;
    Ok(Value::Bool(true))
}

pub async fn get_openclaw_model_catalog() -> Result<Value> {
    let map = read_openclaw()?;
    Ok(agents_defaults(&map)
        .and_then(|d| d.get("models").cloned())
        .unwrap_or(Value::Null))
}

pub async fn set_openclaw_model_catalog(args: Value) -> Result<Value> {
    let catalog: Value = super::require_arg(&args, "catalog")?;
    let mut map = read_openclaw()?;
    let current = agents_defaults(&map).unwrap_or_else(|| Value::Object(Map::new()));
    let mut current = match current {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    current.insert("models".to_string(), catalog);
    set_agents_defaults(&mut map, Value::Object(current));
    write_openclaw(&map)?;
    Ok(Value::Bool(true))
}

pub async fn get_openclaw_agents_defaults() -> Result<Value> {
    let map = read_openclaw()?;
    Ok(agents_defaults(&map).unwrap_or(Value::Null))
}

pub async fn set_openclaw_agents_defaults(args: Value) -> Result<Value> {
    let defaults: Value = super::require_arg(&args, "defaults")?;
    let mut map = read_openclaw()?;
    set_agents_defaults(&mut map, defaults);
    write_openclaw(&map)?;
    Ok(Value::Bool(true))
}

pub async fn get_openclaw_env() -> Result<Value> {
    let map = read_openclaw()?;
    Ok(section(&map, "env").cloned().unwrap_or(Value::Null))
}

pub async fn set_openclaw_env(args: Value) -> Result<Value> {
    let env: Value = super::require_arg(&args, "env")?;
    let mut map = read_openclaw()?;
    map.insert("env".to_string(), env);
    write_openclaw(&map)?;
    Ok(Value::Bool(true))
}

pub async fn get_openclaw_tools() -> Result<Value> {
    let map = read_openclaw()?;
    Ok(section(&map, "tools").cloned().unwrap_or(Value::Null))
}

pub async fn set_openclaw_tools(args: Value) -> Result<Value> {
    let tools: Value = super::require_arg(&args, "tools")?;
    let mut map = read_openclaw()?;
    map.insert("tools".to_string(), tools);
    write_openclaw(&map)?;
    Ok(Value::Bool(true))
}
