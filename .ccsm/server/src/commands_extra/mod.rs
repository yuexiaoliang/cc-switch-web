//! Extended dispatch handlers for cc-switch-web.
//!
//! This module contains the additional Tauri-command handlers that are not
//! part of the original 25 P0 commands in `dispatch.rs`. Each handler is a
//! thin wrapper that calls the upstream service / database layer and
//! serialises the result to `serde_json::Value` for the bridge layer.
//!
//! Handlers here are organised by feature area (one sub-module per area).
//! `dispatch.rs` routes commands to the matching sub-module function.

use crate::error::{ApiError, Result};
use serde_json::Value;
use std::str::FromStr;

use super::AppContext;

pub mod auth;
pub mod claude;
pub mod codex_history;
pub mod deeplink;
pub mod failover;
pub mod mcp;
pub mod omo;
pub mod openclaw;
pub mod opencode;
pub mod pricing;
pub mod prompt;
pub mod provider;
pub mod proxy;
pub mod session;
pub mod skill;
pub mod sync;
pub mod tools;
pub mod usage;

// ---------- shared helpers (re-used across sub-modules) ----------

pub fn require_arg<T: for<'de> serde::Deserialize<'de>>(args: &Value, field: &str) -> Result<T> {
    let value =
        args.as_object()
            .and_then(|o| o.get(field))
            .ok_or_else(|| ApiError::BadArgument {
                field: field.to_string(),
                message: "missing required argument".into(),
            })?;
    serde_json::from_value(value.clone()).map_err(|e| ApiError::BadArgument {
        field: field.to_string(),
        message: e.to_string(),
    })
}

pub fn optional_arg<T: for<'de> serde::Deserialize<'de>>(args: &Value, field: &str) -> Option<T> {
    args.as_object()
        .and_then(|o| o.get(field))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

pub fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub fn optional_i64(args: &Value, field: &str) -> Option<i64> {
    args.get(field).and_then(|v| v.as_i64())
}

pub fn optional_u64(args: &Value, field: &str) -> Option<u64> {
    args.get(field).and_then(|v| v.as_u64())
}

pub fn optional_bool(args: &Value, field: &str) -> Option<bool> {
    args.get(field).and_then(|v| v.as_bool())
}

pub fn require_app_str(args: &Value) -> Result<cc_switch_lib::AppType> {
    let s: String = require_arg(args, "app")?;
    cc_switch_lib::AppType::from_str(&s).map_err(|e| ApiError::BadArgument {
        field: "app".into(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests;
