//! Shared error type for the HTTP adapter.
//!
//! All handlers return `Result<T, ApiError>`. `ApiError` is `axum::Json`-friendly
//! and also implements `IntoResponse` so handlers can `?` straight into
//! upstream `AppError`s after a small conversion.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub type Result<T> = std::result::Result<T, ApiError>;

/// One error variant per failure mode the dispatch layer needs to express.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Tauri-style command not registered on the server.
    #[error("unknown command: {0}")]
    UnknownCommand(String),

    /// Caller supplied an argument that the upstream service rejected.
    #[error("invalid argument `{field}`: {message}")]
    BadArgument { field: String, message: String },

    /// Bubbled up from `cc_switch_lib::AppError`.
    #[error("upstream error: {0}")]
    Upstream(#[from] cc_switch_lib::AppError),

    /// I/O failure (most likely: static file lookup).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Anything else (serde, internal panic, ...).
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::UnknownCommand(_) => (StatusCode::NOT_FOUND, "UNKNOWN_COMMAND"),
            ApiError::BadArgument { .. } => (StatusCode::BAD_REQUEST, "BAD_ARGUMENT"),
            ApiError::Upstream(_) => (StatusCode::INTERNAL_SERVER_ERROR, "UPSTREAM_ERROR"),
            ApiError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO_ERROR"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        // The upstream Tauri layer returned `Err(String)`; we want a stable
        // shape for the bridge layer to surface in its `console.error`
        // messages without leaking internal traces.
        let message = self.to_string();
        log::warn!(target: "cc_switch_web", "request failed [{code}]: {message}");
        (
            status,
            Json(json!({
                "error": {
                    "code": code,
                    "message": message,
                }
            })),
        )
            .into_response()
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Internal(format!("serde_json: {err}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// Map an `ApiError` to a process exit code. Used by `main.rs` to set the
/// shell''s exit status without leaking internal error categories.
pub fn exit_code(err: &ApiError) -> u8 {
    match err {
        ApiError::Io(_) => 3,
        ApiError::Upstream(_) | ApiError::Internal(_) => 4,
        ApiError::BadArgument { .. } => 2,
        ApiError::UnknownCommand(_) => 1,
    }
}

impl From<String> for ApiError {
    fn from(err: String) -> Self {
        ApiError::Internal(err)
    }
}

impl From<&str> for ApiError {
    fn from(err: &str) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        ApiError::Internal(format!("sqlite: {err}"))
    }
}
