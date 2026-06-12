//! Unified MCP server management.

use super::{require_app_str, require_arg, ApiError, AppContext, Result, Value};
use cc_switch_lib::McpService;
use std::sync::Arc;

pub async fn get_mcp_servers(ctx: &Arc<AppContext>) -> Result<Value> {
    let servers = McpService::get_all_servers(&ctx.state).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&servers)?)
}

pub async fn get_mcp_config(ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    let servers = McpService::get_all_servers(&ctx.state).map_err(ApiError::from)?;
    // The upstream `get_servers` is deprecated; we return all servers
    // and let the UI filter by `app` client-side. The legacy field is
    // still present in the response so older clients keep working.
    // get_app_config_path lives in the private `config` module; mirror
    // the upstream behaviour by computing ~/.cc-switch/config.json or
    // the XDG override.
    let path = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::config_dir)
        .unwrap_or_default()
        .join("cc-switch")
        .join("config.json");
    Ok(serde_json::json!({
        "configPath": path.display().to_string(),
        "servers": servers,
    }))
}

pub async fn upsert_mcp_server(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let server: cc_switch_lib::McpServer = require_arg(&args, "server")?;
    McpService::upsert_server(&ctx.state, server).map_err(ApiError::from)?;
    Ok(Value::Null)
}

pub async fn delete_mcp_server(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let removed = McpService::delete_server(&ctx.state, &id).map_err(ApiError::from)?;
    Ok(Value::Bool(removed))
}

pub async fn toggle_mcp_app(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let server_id: String = require_arg(&args, "serverId")?;
    let app = require_app_str(&args)?;
    let enabled: bool = require_arg(&args, "enabled")?;
    McpService::toggle_app(&ctx.state, &server_id, app, enabled).map_err(ApiError::from)?;
    Ok(Value::Null)
}

pub async fn set_mcp_enabled(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let id: String = require_arg(&args, "id")?;
    let enabled: bool = require_arg(&args, "enabled")?;
    McpService::toggle_app(&ctx.state, &id, app, enabled).map_err(ApiError::from)?;
    Ok(Value::Bool(true))
}

pub async fn import_mcp_from_apps(ctx: &Arc<AppContext>) -> Result<Value> {
    let mut total = 0usize;
    total += McpService::import_from_claude(&ctx.state).map_err(ApiError::from)?;
    total += McpService::import_from_codex(&ctx.state).map_err(ApiError::from)?;
    total += McpService::import_from_gemini(&ctx.state).map_err(ApiError::from)?;
    total += McpService::import_from_opencode(&ctx.state).map_err(ApiError::from)?;
    total += McpService::import_from_hermes(&ctx.state).map_err(ApiError::from)?;
    Ok(serde_json::to_value(total)?)
}

// Legacy per-app compat endpoints
pub async fn upsert_mcp_server_in_config(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = require_app_str(&args)?;
    let id: String = require_arg(&args, "id")?;
    let spec: Value = require_arg(&args, "spec")?;
    let sync_other_side: Option<bool> = super::optional_arg(&args, "syncOtherSide");

    let existing = {
        let servers = ctx.state.db.get_all_mcp_servers().map_err(ApiError::from)?;
        servers.get(&id).cloned()
    };
    let mut new_server = if let Some(mut existing) = existing {
        existing.server = spec.clone();
        existing.apps.set_enabled_for(&app, true);
        existing
    } else {
        let mut apps = cc_switch_lib::McpApps::default();
        apps.set_enabled_for(&app, true);
        let name = spec
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        cc_switch_lib::McpServer {
            id: id.clone(),
            name,
            server: spec,
            apps,
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        }
    };
    if sync_other_side.unwrap_or(false) {
        new_server.apps.claude = true;
        new_server.apps.codex = true;
        new_server.apps.gemini = true;
        new_server.apps.opencode = true;
    }
    McpService::upsert_server(&ctx.state, new_server).map_err(ApiError::from)?;
    Ok(Value::Bool(true))
}

pub async fn delete_mcp_server_in_config(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app: String = super::optional_arg(&args, "app").unwrap_or_default();
    let id: String = require_arg(&args, "id")?;
    let removed = McpService::delete_server(&ctx.state, &id).map_err(ApiError::from)?;
    Ok(Value::Bool(removed))
}

// Claude-native MCP commands (delegate to upstream command functions which
// read/write ~/.claude.json directly).
pub async fn get_claude_mcp_status() -> Result<Value> {
    let status = cc_switch_lib::get_claude_mcp_status()
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&status)?)
}

pub async fn read_claude_mcp_config() -> Result<Value> {
    let content = cc_switch_lib::read_claude_mcp_config()
        .await
        .map_err(ApiError::from)?;
    Ok(content.map(Value::String).unwrap_or(Value::Null))
}

pub async fn upsert_claude_mcp_server(args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let spec: Value = require_arg(&args, "spec")?;
    let ok = cc_switch_lib::upsert_claude_mcp_server(id, spec)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Bool(ok))
}

pub async fn delete_claude_mcp_server(args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let ok = cc_switch_lib::delete_claude_mcp_server(id)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Bool(ok))
}

pub async fn validate_mcp_command(args: Value) -> Result<Value> {
    let cmd: String = require_arg(&args, "cmd")?;
    let ok = cc_switch_lib::validate_mcp_command(cmd)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Bool(ok))
}
