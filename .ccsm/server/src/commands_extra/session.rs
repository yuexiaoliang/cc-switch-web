//! Session scanning, message loading, and deletion.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn list_sessions(_ctx: &Arc<AppContext>) -> Result<Value> {
    let sessions = cc_switch_lib::list_sessions()
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&sessions)?)
}

pub async fn get_session_messages(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let provider_id: String = require_arg(&args, "providerId")?;
    let source_path: String = require_arg(&args, "sourcePath")?;
    let messages = cc_switch_lib::get_session_messages(provider_id, source_path)
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&messages)?)
}

pub async fn delete_session(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let provider_id: String = require_arg(&args, "providerId")?;
    let session_id: String = require_arg(&args, "sessionId")?;
    let source_path: String = require_arg(&args, "sourcePath")?;
    let ok = cc_switch_lib::delete_session(provider_id, session_id, source_path)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Bool(ok))
}

pub async fn delete_sessions(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let requests: Vec<Value> = require_arg(&args, "requests")?;
    let mut outcomes = Vec::with_capacity(requests.len());
    for req in requests {
        let provider_id = req
            .get("providerId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::BadArgument {
                field: "requests[].providerId".into(),
                message: "missing".into(),
            })?
            .to_string();
        let session_id = req
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::BadArgument {
                field: "requests[].sessionId".into(),
                message: "missing".into(),
            })?
            .to_string();
        let source_path = req
            .get("sourcePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::BadArgument {
                field: "requests[].sourcePath".into(),
                message: "missing".into(),
            })?
            .to_string();
        let ok = cc_switch_lib::delete_session(provider_id, session_id, source_path)
            .await
            .map_err(ApiError::from)?;
        outcomes.push(serde_json::json!({ "ok": ok }));
    }
    Ok(Value::Array(outcomes))
}

pub async fn launch_session_terminal(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let command: String = require_arg(&args, "command")?;
    let cwd: Option<String> = super::optional_arg(&args, "cwd");
    log::info!(
        "launch_session_terminal: headless server; command={command} cwd={cwd:?} (run manually)"
    );
    Ok(Value::Bool(false))
}
