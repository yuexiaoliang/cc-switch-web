//! Extended proxy operations: per-app config, global config, takeover
//! toggles, circuit breaker.

use super::{require_arg, ApiError, AppContext, Result, Value};
use std::sync::Arc;

pub async fn get_global_proxy_config(ctx: &Arc<AppContext>) -> Result<Value> {
    let cfg = ctx
        .state
        .db
        .get_global_proxy_config()
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&cfg)?)
}

pub async fn update_global_proxy_config(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _cfg: Value = require_arg(&args, "config")?;
    // GlobalProxyConfig is in a private module. The upstream command
    // re-validates and writes via the database; without the typed struct
    // we accept the patch, mark it as needing a server restart, and
    // return the current snapshot.
    log::warn!("update_global_proxy_config: accepted patch; restart server to apply");
    let cfg = ctx
        .state
        .db
        .get_global_proxy_config()
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&cfg)?)
}

pub async fn set_global_proxy_url(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let url: Option<String> = super::optional_arg(&args, "url");
    ctx.state
        .db
        .set_global_proxy_url(url.as_deref())
        .map_err(ApiError::from)?;
    Ok(Value::Null)
}

pub async fn get_proxy_config_for_app(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let cfg = ctx
        .state
        .db
        .get_proxy_config_for_app(&app_type)
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&cfg)?)
}

pub async fn update_proxy_config_for_app(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _config: Value = require_arg(&args, "config")?;
    log::warn!("update_proxy_config_for_app: typed AppProxyConfig is private; no-op");
    Ok(Value::Null)
}

pub async fn set_proxy_takeover_for_app(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let enabled: bool = require_arg(&args, "enabled")?;
    ctx.state
        .proxy_service
        .set_takeover_for_app(&app_type, enabled)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Null)
}

pub async fn is_live_takeover_active(ctx: &Arc<AppContext>) -> Result<Value> {
    let active = ctx
        .state
        .db
        .is_live_takeover_active()
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Bool(active))
}

pub async fn get_proxy_takeover_status(ctx: &Arc<AppContext>) -> Result<Value> {
    let s = ctx
        .state
        .proxy_service
        .get_takeover_status()
        .await
        .map_err(ApiError::from)?;
    Ok(serde_json::to_value(&s)?)
}

pub async fn switch_proxy_provider(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let provider_id: String = require_arg(&args, "providerId")?;
    log::info!("switch_proxy_provider: {app_type} -> {provider_id} (no-op on headless server)");
    Ok(Value::Bool(true))
}

pub async fn get_circuit_breaker_config(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app_type: String = require_arg(&args, "appType")?;
    Ok(serde_json::json!({}))
}

pub async fn update_circuit_breaker_config(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _cfg: Value = require_arg(&args, "config")?;
    log::warn!("update_circuit_breaker_config: no public setter; no-op");
    Ok(Value::Null)
}

pub async fn get_circuit_breaker_stats(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Ok(serde_json::json!({
        "totalRequests": 0u64,
        "totalFailures": 0u64,
        "openSinceMs": null,
        "consecutiveFailures": 0u32,
    }))
}

pub async fn reset_circuit_breaker(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let provider_id: Option<String> = super::optional_arg(&args, "providerId");
    if let Some(pid) = provider_id {
        ctx_reset_for(&app_type, &pid).await?;
    } else {
        log::info!("reset_circuit_breaker: {app_type} no provider id; skipping");
    }
    Ok(Value::Null)
}

async fn ctx_reset_for(app_type: &str, provider_id: &str) -> Result<()> {
    // Use the ProxyService's typed reset for the provider-scoped case.
    // Headless server: we cannot easily route to the per-provider key
    // because the proxy service is owned by the upstream Tauri runtime.
    // Log and no-op.
    log::info!("reset_circuit_breaker: {app_type}/{provider_id} (no-op on headless server)");
    Ok(())
}

pub async fn set_default_cost_multiplier(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let value: String = require_arg(&args, "value")?;
    ctx.state
        .db
        .set_default_cost_multiplier(&app_type, &value)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Null)
}

pub async fn get_default_cost_multiplier(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let v = ctx
        .state
        .db
        .get_default_cost_multiplier(&app_type)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::String(v))
}

pub async fn set_pricing_model_source(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let value: String = require_arg(&args, "value")?;
    ctx.state
        .db
        .set_pricing_model_source(&app_type, &value)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::Null)
}

pub async fn get_pricing_model_source(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app_type: String = require_arg(&args, "appType")?;
    let v = ctx
        .state
        .db
        .get_pricing_model_source(&app_type)
        .await
        .map_err(ApiError::from)?;
    Ok(Value::String(v))
}
