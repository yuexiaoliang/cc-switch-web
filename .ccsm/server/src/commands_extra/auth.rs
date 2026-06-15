//! Auth / OAuth stubs.
//!
//! The upstream `auth` and `copilot` commands talk to OAuth providers
//! that need a desktop browser / WebView to complete the device flow.
//! The headless server has no WebView, so we accept the calls and
//! return a deterministic empty/failure response that the frontend can
//! surface in the UI.

use super::{ApiError, Result, Value};

pub async fn auth_logout(_args: Value) -> Result<Value> {
    log::info!("auth_logout: no OAuth manager on headless server");
    Ok(Value::Bool(true))
}

pub async fn auth_remove_account(args: Value) -> Result<Value> {
    let _id: String = super::require_arg(&args, "accountId")?;
    log::info!("auth_remove_account: no OAuth manager on headless server");
    Ok(Value::Bool(true))
}

pub async fn auth_set_default_account(args: Value) -> Result<Value> {
    let _id: String = super::require_arg(&args, "accountId")?;
    log::info!("auth_set_default_account: no OAuth manager on headless server");
    Ok(Value::Bool(true))
}

pub async fn copilot_logout(_args: Value) -> Result<Value> {
    log::info!("copilot_logout: no OAuth manager on headless server");
    Ok(Value::Bool(true))
}

pub async fn copilot_remove_account(args: Value) -> Result<Value> {
    let _id: String = super::require_arg(&args, "accountId")?;
    log::info!("copilot_remove_account: no OAuth manager on headless server");
    Ok(Value::Bool(true))
}

pub async fn copilot_set_default_account(args: Value) -> Result<Value> {
    let _id: String = super::require_arg(&args, "accountId")?;
    log::info!("copilot_set_default_account: no OAuth manager on headless server");
    Ok(Value::Bool(true))
}

pub async fn import_claude_desktop_providers_from_claude(_args: Value) -> Result<Value> {
    // Claude Desktop is a desktop app; headless server returns 0.
    Ok(serde_json::to_value(0u64)?)
}

// Required to keep ApiError referenced so the import doesn't trigger
// unused warnings.
#[allow(dead_code)]
fn _force_use(_: ApiError) {}

pub async fn auth_get_status(_args: Value) -> Result<Value> {
    // Headless server has no AuthManager; report a deterministic empty status.
    Ok(serde_json::json!({
        "loggedIn": false,
        "accounts": [],
        "defaultAccountId": null,
    }))
}

pub async fn auth_start_login(_args: Value) -> Result<Value> {
    Err(ApiError::Internal("auth_start_login is not supported on the headless server; configure credentials in ~/.claude or via the upstream desktop app".into()))
}

pub async fn auth_poll_for_account(_args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "auth_poll_for_account is not supported on the headless server".into(),
    ))
}

pub async fn auth_list_accounts(_args: Value) -> Result<Value> {
    Ok(serde_json::json!([]))
}
