//! Deep link parse / merge / import.
//!
//! Most of the upstream `deeplink::*` helpers are private. We re-use
//! the few items the `lib.rs` re-exports (`parse_deeplink_url`,
//! `import_provider_from_deeplink`, `DeepLinkImportRequest`) and the
//! upstream `merge_deeplink_config` command. The multi-resource
//! dispatcher is re-implemented because the upstream
//! `import_from_deeplink_unified` takes `State<AppState>` which we
//! cannot construct from outside the Tauri runtime.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn parse_deeplink(args: Value) -> Result<Value> {
    let url: String = require_arg(&args, "url")?;
    let req = cc_switch_lib::parse_deeplink_url(&url).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&req)?)
}

pub async fn merge_deeplink_config(args: Value) -> Result<Value> {
    let request: cc_switch_lib::DeepLinkImportRequest = require_arg(&args, "request")?;
    let merged = cc_switch_lib::merge_deeplink_config(request).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&merged)?)
}

pub async fn import_from_deeplink_unified(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let request: cc_switch_lib::DeepLinkImportRequest = require_arg(&args, "request")?;
    match request.resource.as_str() {
        "provider" => {
            let id = cc_switch_lib::import_provider_from_deeplink(&ctx.state, request)
                .map_err(ApiError::from)?;
            Ok(serde_json::json!({ "type": "provider", "id": id }))
        }
        "prompt" | "mcp" | "skill" => Err(ApiError::Internal(format!(
            "{} imports are not yet wired into cc-switch-mini",
            request.resource
        ))),
        other => Err(ApiError::BadArgument {
            field: "request.resource".into(),
            message: format!("unsupported resource type: {other}"),
        }),
    }
}

pub async fn import_mcp_from_deeplink(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    // The upstream `import_mcp_from_deeplink` lives in the private
    // `deeplink` module. Fall back to a no-op for now; users can use
    // `import_mcp_from_apps` to populate from local CLI configs instead.
    Err(ApiError::Internal(
        "deeplink MCP import requires the upstream private helper; use import_mcp_from_apps"
            .into(),
    ))
}
