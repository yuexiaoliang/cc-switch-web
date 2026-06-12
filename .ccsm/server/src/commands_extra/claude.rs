//! Claude-specific commands: desktop config, onboarding skip, plugin config.

use super::{ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn get_claude_desktop_status() -> Result<Value> {
    // Check if the claude-desktop app is installed (no GUI probe — just
    // verify the binary path exists on disk).
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::home_dir);
    let path = home.map(|h| {
        [
            h.join("Library/Application Support/Claude"),
            h.join(".config/Claude"),
        ]
    });
    let installed = path
        .map(|paths| paths.iter().any(|p| p.exists()))
        .unwrap_or(false);
    Ok(serde_json::json!({
        "installed": installed,
        "cliAvailable": false,
    }))
}

pub async fn get_claude_desktop_default_routes() -> Result<Value> {
    // Upstream returns a hard-coded list of default routes for Claude
    // Desktop's provider set. We do not have access to that list from
    // outside; return an empty array so the frontend can fall back to its
    // own defaults.
    Ok(Value::Array(Vec::new()))
}

pub async fn get_claude_code_config_path(_ctx: &Arc<AppContext>) -> Result<Value> {
    let upstream = cc_switch_lib::get_claude_code_config_path()
        .await
        .map_err(ApiError::from)?;
    Ok(Value::String(upstream))
}
