//! Misc / utility commands.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn open_external(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let url: String = require_arg(&args, "url")?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(ApiError::BadArgument {
            field: "url".into(),
            message: "only http(s) URLs are allowed".into(),
        });
    }
    log::info!("open_external: {url}");
    Ok(Value::Bool(true))
}

pub async fn open_provider_terminal(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _provider_id: Option<String> = super::optional_arg(&args, "providerId");
    log::info!("open_provider_terminal: headless server; not opening a terminal");
    Ok(Value::Bool(false))
}

pub async fn open_workspace_directory(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _path: String = require_arg(&args, "path")?;
    log::info!("open_workspace_directory: headless server; not opening a file manager");
    Ok(Value::Bool(false))
}

pub async fn pick_directory(_args: Value) -> Result<Value> {
    Ok(Value::Null)
}

pub async fn open_zip_file_dialog(_args: Value) -> Result<Value> {
    Ok(Value::Null)
}

pub async fn run_tool_lifecycle_action(_args: Value) -> Result<Value> {
    log::info!("run_tool_lifecycle_action: no-op on headless server");
    Ok(serde_json::json!({ "ok": false, "message": "lifecycle actions are desktop-only" }))
}

pub async fn probe_tool_installations(_args: Value) -> Result<Value> {
    let bins = [
        "claude", "codex", "gemini", "opencode", "hermes", "openclaw",
    ];
    let mut out: Vec<Value> = Vec::new();
    for bin in bins {
        let present = which(bin);
        out.push(serde_json::json!({
            "tool": bin,
            "installed": present.is_some(),
            "path": present.map(|p| p.display().to_string()).unwrap_or_default(),
        }));
    }
    Ok(Value::Array(out))
}

fn which(cmd: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub async fn get_tool_versions(_args: Value) -> Result<Value> {
    let bins = [
        "claude", "codex", "gemini", "opencode", "hermes", "openclaw",
    ];
    let out: Vec<Value> = bins
        .iter()
        .map(|b| serde_json::json!({ "tool": b, "version": "unknown" }))
        .collect();
    Ok(Value::Array(out))
}

pub async fn export_config_to_file(_args: Value) -> Result<Value> {
    log::info!("export_config_to_file: headless server; provide a configPath to dump the DB");
    Ok(Value::Null)
}

pub async fn import_config_from_file(_args: Value) -> Result<Value> {
    log::info!("import_config_from_file: headless server; provide a configPath to import");
    Ok(Value::Null)
}

pub async fn get_balance(args: Value) -> Result<Value> {
    let base_url: String = require_arg(&args, "baseUrl")?;
    let api_key: String = require_arg(&args, "apiKey")?;
    let result = cc_switch_lib::get_balance(base_url, api_key)
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn get_codex_oauth_quota(_args: Value) -> Result<Value> {
    // Upstream `get_codex_oauth_quota` takes `State<CodexOAuthState>`.
    // The headless server has no OAuth manager in scope; report null
    // and log a warning.
    log::warn!("get_codex_oauth_quota: no OAuth manager in headless server");
    Ok(Value::Null)
}

pub async fn get_codex_oauth_models(_args: Value) -> Result<Value> {
    log::warn!("get_codex_oauth_models: no OAuth manager in headless server");
    Ok(Value::Null)
}

pub async fn get_coding_plan_quota(args: Value) -> Result<Value> {
    let base_url: String = require_arg(&args, "baseUrl")?;
    let api_key: String = require_arg(&args, "apiKey")?;
    let access_key_id: Option<String> = super::optional_arg(&args, "accessKeyId");
    let secret_access_key: Option<String> = super::optional_arg(&args, "secretAccessKey");
    let v = cc_switch_lib::get_coding_plan_quota(
        base_url,
        api_key,
        access_key_id,
        secret_access_key,
    )
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&v)?)
}

pub async fn get_log_config(_ctx: &Arc<AppContext>) -> Result<Value> {
    let conn = open_db()?;
    let json: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'log_config'",
            [],
            |row| row.get(0),
        )
        .ok();
    if let Some(s) = json {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            return Ok(v);
        }
    }
    Ok(serde_json::json!({
        "level": "info",
        "maxFileSize": 5_242_880u64,
        "maxFiles": 3u32,
    }))
}

pub async fn set_log_config(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let config: Value = require_arg(&args, "config")?;
    let conn = open_db()?;
    let json = serde_json::to_string(&config)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('log_config', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [json],
    )
    .map_err(|e| ApiError::Internal(format!("set_log_config: {e}")))?;
    Ok(Value::Null)
}

pub async fn set_app_config_dir_override(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let dir: Option<String> = super::optional_arg(&args, "dir");
    if let Some(d) = &dir {
        std::env::set_var("XDG_CONFIG_HOME", d);
        log::info!("set_app_config_dir_override: {d}");
    } else {
        std::env::remove_var("XDG_CONFIG_HOME");
        log::info!("set_app_config_dir_override: cleared");
    }
    Ok(Value::Null)
}

pub async fn get_app_config_dir_override() -> Result<Value> {
    Ok(std::env::var("XDG_CONFIG_HOME")
        .map(Value::String)
        .unwrap_or(Value::Null))
}

fn open_db() -> Result<rusqlite::Connection> {
    let path = crate::state::app_config_dir().join("cc-switch.db");
    rusqlite::Connection::open(&path).map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}
