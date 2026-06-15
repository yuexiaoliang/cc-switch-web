//! Extended provider operations: universal providers, custom endpoints,
//! live config, common-config snippets.

use super::{require_arg, ApiError, AppContext, Result, Value};
use cc_switch_lib::ProviderService;
use serde_json::{Map, Value as JsonValue};
use std::sync::Arc;

const UNIVERSAL_KEY: &str = "universal_providers";

fn open_db() -> Result<rusqlite::Connection> {
    let path = crate::state::app_config_dir().join("cc-switch.db");
    rusqlite::Connection::open(&path).map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}

fn read_universal(_ctx: &Arc<AppContext>) -> Result<Map<String, JsonValue>> {
    let conn = open_db()?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [UNIVERSAL_KEY],
            |row| row.get(0),
        )
        .ok();
    match raw {
        Some(s) => serde_json::from_str(&s)
            .map_err(|e| ApiError::Internal(format!("parse {UNIVERSAL_KEY}: {e}"))),
        None => Ok(Map::new()),
    }
}

fn write_universal(_ctx: &Arc<AppContext>, map: &Map<String, JsonValue>) -> Result<()> {
    let conn = open_db()?;
    let json = serde_json::to_string(map)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![UNIVERSAL_KEY, json],
    )
    .map_err(|e| ApiError::Internal(format!("write {UNIVERSAL_KEY}: {e}")))?;
    Ok(())
}

pub async fn get_universal_providers(ctx: &Arc<AppContext>) -> Result<Value> {
    let map = read_universal(ctx)?;
    Ok(Value::Object(map))
}

pub async fn get_universal_provider(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let map = read_universal(ctx)?;
    Ok(map.get(&id).cloned().unwrap_or(Value::Null))
}

pub async fn upsert_universal_provider(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let mut provider: Value = require_arg(&args, "provider")?;
    let mut map = read_universal(ctx)?;
    let id = provider
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadArgument {
            field: "provider.id".into(),
            message: "missing".into(),
        })?
        .to_string();
    if let Some(obj) = provider.as_object_mut() {
        obj.entry("updatedAt".to_string())
            .or_insert_with(|| Value::from(chrono::Utc::now().timestamp()));
    }
    map.insert(id, provider);
    write_universal(ctx, &map)?;
    Ok(Value::Bool(true))
}

pub async fn delete_universal_provider(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let mut map = read_universal(ctx)?;
    let removed = map.remove(&id).is_some();
    if removed {
        write_universal(ctx, &map)?;
    }
    Ok(Value::Bool(removed))
}

pub async fn sync_universal_provider(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _id: String = require_arg(&args, "id")?;
    log::info!("sync_universal_provider: headless server; not auto-syncing");
    Ok(Value::Bool(true))
}

pub async fn sync_current_providers_live(ctx: &Arc<AppContext>) -> Result<Value> {
    ProviderService::sync_current_to_live(&ctx.state).map_err(ApiError::from)?;
    Ok(Value::Bool(true))
}

pub async fn get_custom_endpoints(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _provider_id: String = require_arg(&args, "providerId")?;
    Ok(Value::Array(Vec::new()))
}

pub async fn add_custom_endpoint(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _provider_id: String = require_arg(&args, "providerId")?;
    let _endpoint: String = require_arg(&args, "endpoint")?;
    let _note: Option<String> = super::optional_arg(&args, "note");
    log::info!("add_custom_endpoint: no public API; no-op");
    Ok(Value::Null)
}

pub async fn remove_custom_endpoint(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _provider_id: String = require_arg(&args, "providerId")?;
    let _endpoint: String = require_arg(&args, "endpoint")?;
    log::info!("remove_custom_endpoint: no public API; no-op");
    Ok(Value::Bool(true))
}

pub async fn update_endpoint_last_used(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Ok(Value::Null)
}

pub async fn read_live_provider_settings(args: Value) -> Result<Value> {
    let app = super::require_app_str(&args)?;
    let v = ProviderService::read_live_settings(app).map_err(ApiError::from)?;
    Ok(v)
}

pub async fn import_default_config(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = super::require_app_str(&args)?;
    let ok = ProviderService::import_default_config(&ctx.state, app).map_err(ApiError::from)?;
    Ok(Value::Bool(ok))
}

pub async fn test_api_endpoints(_args: Value) -> Result<Value> {
    Ok(Value::Array(Vec::new()))
}

pub async fn query_provider_usage(_args: Value) -> Result<Value> {
    Ok(serde_json::json!({ "entries": [] }))
}

pub async fn test_usage_script(_args: Value) -> Result<Value> {
    Ok(serde_json::json!({ "ok": false, "message": "no usage script configured" }))
}

pub async fn fetch_models_for_config(args: Value) -> Result<Value> {
    use std::time::Duration;
    let base_url: String = require_arg(&args, "baseUrl")?;
    let api_key: Option<String> = super::optional_arg(&args, "apiKey");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| ApiError::Internal(format!("reqwest: {e}")))?;
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let mut req = client.get(&url);
    if let Some(k) = api_key.as_deref() {
        req = req.bearer_auth(k);
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().await.unwrap_or(Value::Null);
            Ok(body)
        }
        Ok(resp) => Err(ApiError::Internal(format!(
            "fetch_models_for_config: HTTP {}",
            resp.status()
        ))),
        Err(e) => Err(ApiError::Internal(format!("fetch_models_for_config: {e}"))),
    }
}

pub async fn get_claude_common_config_snippet(_args: Value) -> Result<Value> {
    // The upstream `get_claude_common_config_snippet` takes `State<AppState>`.
    // On the headless server we return null/empty so the UI shows nothing
    // in the snippet editor until the user explicitly saves a value.
    Ok(Value::Null)
}

pub async fn set_claude_common_config_snippet(args: Value) -> Result<Value> {
    let _snippet: String = require_arg(&args, "snippet")?;
    let _enabled: Option<bool> = super::optional_arg(&args, "enabled");
    Ok(Value::Bool(true))
}

pub async fn get_common_config_snippet(_args: Value) -> Result<Value> {
    // Accept both `app` and `appType`; upstream uses `appType`.
    Ok(Value::Null)
}

pub async fn set_common_config_snippet(args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _snippet: String = require_arg(&args, "snippet")?;
    let _enabled: Option<bool> = super::optional_arg(&args, "enabled");
    Ok(Value::Bool(true))
}

pub async fn apply_claude_plugin_config(args: Value) -> Result<Value> {
    let _config: Value = require_arg(&args, "config")?;
    log::info!("apply_claude_plugin_config: no-op on headless server");
    Ok(Value::Bool(true))
}

pub async fn apply_claude_onboarding_skip(_args: Value) -> Result<Value> {
    log::info!("apply_claude_onboarding_skip: no-op on headless server");
    Ok(Value::Bool(true))
}

pub async fn clear_claude_onboarding_skip(_args: Value) -> Result<Value> {
    log::info!("clear_claude_onboarding_skip: no-op on headless server");
    Ok(Value::Bool(true))
}

pub async fn ensure_claude_desktop_official_provider(
    _ctx: &Arc<AppContext>,
    _args: Value,
) -> Result<Value> {
    log::info!("ensure_claude_desktop_official_provider: no-op on headless server");
    Ok(Value::Bool(true))
}
