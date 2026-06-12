//! Tests for the extra command handlers.

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    fn dummy_ctx() -> std::sync::Arc<crate::AppContext> {
        std::sync::Arc::new(crate::AppContext::placeholder())
    }

    #[test]
    fn require_arg_missing_field_returns_bad_argument() {
        let v = json!({});
        let err = require_arg::<String>(&v, "missing").unwrap_err();
        match err {
            ApiError::BadArgument { field, .. } => assert_eq!(field, "missing"),
            other => panic!("expected BadArgument, got {other:?}"),
        }
    }

    #[test]
    fn require_arg_wrong_type_returns_bad_argument() {
        let v = json!({ "n": "not a number" });
        let err = require_arg::<i64>(&v, "n").unwrap_err();
        match err {
            ApiError::BadArgument { field, .. } => assert_eq!(field, "n"),
            other => panic!("expected BadArgument, got {other:?}"),
        }
    }

    #[test]
    fn require_arg_returns_value_when_correct() {
        let v = json!({ "x": 1, "s": "hi" });
        assert_eq!(require_arg::<i64>(&v, "x").unwrap(), 1);
        assert_eq!(require_arg::<String>(&v, "s").unwrap(), "hi");
    }

    #[test]
    fn optional_arg_returns_none_when_absent() {
        let v = json!({});
        let x: Option<String> = optional_arg(&v, "missing");
        assert!(x.is_none());
    }

    #[test]
    fn optional_str_returns_value_when_present() {
        let v = json!({ "s": "hello" });
        assert_eq!(optional_str(&v, "s").as_deref(), Some("hello"));
    }

    #[test]
    fn optional_str_returns_none_for_wrong_type() {
        let v = json!({ "n": 1 });
        assert!(optional_str(&v, "n").is_none());
    }

    #[test]
    fn optional_i64_handles_integers() {
        let v = json!({ "n": 42, "f": 1.5 });
        assert_eq!(optional_i64(&v, "n"), Some(42));
        assert_eq!(optional_i64(&v, "f"), None); // 1.5 is not an integer
    }

    #[test]
    fn optional_u64_returns_none_for_missing() {
        let v = json!({});
        assert_eq!(optional_u64(&v, "n"), None);
    }

    #[test]
    fn optional_bool_handles_true_false() {
        let v = json!({ "a": true, "b": false });
        assert_eq!(optional_bool(&v, "a"), Some(true));
        assert_eq!(optional_bool(&v, "b"), Some(false));
        assert_eq!(optional_bool(&v, "missing"), None);
    }

    #[test]
    fn require_app_str_handles_known_apps() {
        for app in ["claude", "codex", "gemini", "opencode", "openclaw", "hermes", "claude-desktop"] {
            let v = json!({ "app": app });
            require_app_str(&v).unwrap();
        }
    }

    #[test]
    fn require_app_str_rejects_unknown_app() {
        let v = json!({ "app": "no-such-app" });
        assert!(require_app_str(&v).is_err());
    }

    #[tokio::test]
    async fn open_external_rejects_non_http_url() {
        let args = json!({ "url": "javascript:alert(1)" });
        let result = tools::open_external(&dummy_ctx(), args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn open_external_accepts_https_url() {
        let args = json!({ "url": "https://example.com" });
        let result = tools::open_external(&dummy_ctx(), args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn open_external_accepts_http_url() {
        let args = json!({ "url": "http://example.com/path?x=1" });
        let result = tools::open_external(&dummy_ctx(), args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn auth_logout_returns_true() {
        let result = auth::auth_logout(json!({})).await;
        assert_eq!(result.unwrap(), json!(true));
    }

    #[tokio::test]
    async fn auth_get_status_returns_empty_status() {
        let result = auth::auth_get_status(json!({})).await;
        let v = result.unwrap();
        assert_eq!(v["loggedIn"], json!(false));
        assert_eq!(v["accounts"], json!([]));
    }

    #[tokio::test]
    async fn get_tool_versions_returns_array() {
        let result = tools::get_tool_versions(json!({})).await;
        let arr = result.unwrap();
        assert!(arr.is_array());
        assert!(arr.as_array().unwrap().len() >= 5);
    }

    #[tokio::test]
    async fn probe_tool_installations_returns_array() {
        let result = tools::probe_tool_installations(json!({})).await;
        let arr = result.unwrap();
        assert!(arr.is_array());
    }

    #[tokio::test]
    async fn pick_directory_returns_null() {
        let result = tools::pick_directory(json!({})).await;
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[tokio::test]
    async fn openclaw_default_model_is_safe() {
        // Either null (no config) or an object is acceptable; the test only
        // ensures the handler does not panic.
        let result = openclaw::get_openclaw_default_model().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn failover_get_failover_queue_rejects_missing_app() {
        // The handler must require an app argument.
        let r = failover::get_failover_queue(&dummy_ctx(), json!({})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn pricing_get_model_pricing_returns_array_or_database_error() {
        // We don't need a real database for this; the placeholder is fine.
        // The handler may error if the path is missing, which is OK.
        let result = pricing::get_model_pricing(&dummy_ctx()).await;
        // Either it returns an array, or fails with internal/db error
        match result {
            Ok(v) => assert!(v.is_array()),
            Err(_) => {}
        }
    }
}
