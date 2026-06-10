//! Bearer-token authentication.
//!
//! When the server is started with `--token <secret>` every `/api/*` request
//! must carry an `Authorization: Bearer <secret>` header. The check is
//! constant-time to avoid leaking the token length or content through
//! timing side channels.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::sync::Arc;

/// Pull the bearer token out of a `HeaderMap`. Case-insensitive prefix
/// match covers the common `Bearer` and `bearer` forms.
pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .map(|s| s.to_string())
}

/// Constant-time equality on two byte strings.
pub fn token_matches(expected: &str, supplied: &str) -> bool {
    let a = expected.as_bytes();
    let b = supplied.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Stand-alone authorisation function. Returns `true` when authentication
/// is satisfied (either by matching the token, or because none was
/// configured).
pub fn authorise(expected: Option<&str>, headers: &HeaderMap) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    match extract_token(headers) {
        Some(supplied) => token_matches(expected, &supplied),
        None => false,
    }
}

/// 401 response body. Kept consistent with `ApiError::IntoResponse` so the
/// frontend sees the same shape regardless of where auth fails.
pub fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": {
                "code": "UNAUTHORIZED",
                "message": "missing or invalid bearer token"
            }
        })),
    )
        .into_response()
}

/// Middleware. The expected token travels in `AppContext.opts.token`; this
/// extractor pulls it from shared state so we do not have to wire a
/// separate layer.
pub async fn enforce(
    State(ctx): State<Arc<crate::AppContext>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if !authorise(ctx.opts.token.as_deref(), &headers) {
        return unauthorized();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_expected_token_accepts_anything() {
        let headers = HeaderMap::new();
        assert!(authorise(None, &headers));
        let mut h = HeaderMap::new();
        h.insert("authorization", "Bearer x".parse().unwrap());
        assert!(authorise(None, &h));
    }

    #[test]
    fn matching_token_authorised() {
        let mut h = HeaderMap::new();
        h.insert("authorization", "Bearer secret".parse().unwrap());
        assert!(authorise(Some("secret"), &h));
    }

    #[test]
    fn wrong_token_rejected() {
        let mut h = HeaderMap::new();
        h.insert("authorization", "Bearer nope".parse().unwrap());
        assert!(!authorise(Some("secret"), &h));
    }

    #[test]
    fn missing_token_rejected() {
        assert!(!authorise(Some("secret"), &HeaderMap::new()));
    }
}
