//! Extended provider operations: universal providers, custom endpoints,
//! live config, common-config snippets.

use super::{require_arg, ApiError, AppContext, Result, Value};
use cc_switch_lib::ProviderService;
use reqwest::StatusCode;
use serde_json::{Map, Value as JsonValue};
use std::sync::Arc;

const UNIVERSAL_KEY: &str = "universal_providers";

fn open_db() -> Result<rusqlite::Connection> {
    let path = crate::state::app_config_dir().join("cc-switch.db");
    rusqlite::Connection::open(&path).map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}

fn read_universal(_ctx: &Arc<AppContext>) -> Result<Map<String, JsonValue>> {
    let conn = open_db()?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [UNIVERSAL_KEY],
            |row| row.get(0),
        )
        .ok();
    match raw {
        Some(s) => serde_json::from_str(&s)
            .map_err(|e| ApiError::Internal(format!("parse {UNIVERSAL_KEY}: {e}"))),
        None => Ok(Map::new()),
    }
}

fn write_universal(_ctx: &Arc<AppContext>, map: &Map<String, JsonValue>) -> Result<()> {
    let conn = open_db()?;
    let json = serde_json::to_string(map)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![UNIVERSAL_KEY, json],
    )
    .map_err(|e| ApiError::Internal(format!("write {UNIVERSAL_KEY}: {e}")))?;
    Ok(())
}

pub async fn get_universal_providers(ctx: &Arc<AppContext>) -> Result<Value> {
    let map = read_universal(ctx)?;
    Ok(Value::Object(map))
}

pub async fn get_universal_provider(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let map = read_universal(ctx)?;
    Ok(map.get(&id).cloned().unwrap_or(Value::Null))
}

pub async fn upsert_universal_provider(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let mut provider: Value = require_arg(&args, "provider")?;
    let mut map = read_universal(ctx)?;
    let id = provider
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadArgument {
            field: "provider.id".into(),
            message: "missing".into(),
        })?
        .to_string();
    if let Some(obj) = provider.as_object_mut() {
        obj.entry("updatedAt".to_string())
            .or_insert_with(|| Value::from(chrono::Utc::now().timestamp()));
    }
    map.insert(id, provider);
    write_universal(ctx, &map)?;
    Ok(Value::Bool(true))
}

pub async fn delete_universal_provider(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let mut map = read_universal(ctx)?;
    let removed = map.remove(&id).is_some();
    if removed {
        write_universal(ctx, &map)?;
    }
    Ok(Value::Bool(removed))
}

pub async fn sync_universal_provider(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _id: String = require_arg(&args, "id")?;
    log::info!("sync_universal_provider: headless server; not auto-syncing");
    Ok(Value::Bool(true))
}

pub async fn sync_current_providers_live(ctx: &Arc<AppContext>) -> Result<Value> {
    ProviderService::sync_current_to_live(&ctx.state).map_err(ApiError::from)?;
    Ok(Value::Bool(true))
}

pub async fn get_custom_endpoints(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _provider_id: String = require_arg(&args, "providerId")?;
    Ok(Value::Array(Vec::new()))
}

pub async fn add_custom_endpoint(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _provider_id: String = require_arg(&args, "providerId")?;
    let _endpoint: String = require_arg(&args, "endpoint")?;
    let _note: Option<String> = super::optional_arg(&args, "note");
    log::info!("add_custom_endpoint: no public API; no-op");
    Ok(Value::Null)
}

pub async fn remove_custom_endpoint(_ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _provider_id: String = require_arg(&args, "providerId")?;
    let _endpoint: String = require_arg(&args, "endpoint")?;
    log::info!("remove_custom_endpoint: no public API; no-op");
    Ok(Value::Bool(true))
}

pub async fn update_endpoint_last_used(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Ok(Value::Null)
}

pub async fn read_live_provider_settings(args: Value) -> Result<Value> {
    let app = super::require_app_str(&args)?;
    let v = ProviderService::read_live_settings(app).map_err(ApiError::from)?;
    Ok(v)
}

pub async fn import_default_config(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let app = super::require_app_str(&args)?;
    let ok = ProviderService::import_default_config(&ctx.state, app).map_err(ApiError::from)?;
    Ok(Value::Bool(ok))
}

pub async fn test_api_endpoints(args: Value) -> Result<Value> {
    let urls: Vec<String> = super::require_arg(&args, "urls")?;
    let timeout_secs: u64 = super::optional_u64(&args, "timeoutSecs").unwrap_or(8);
    let timeout = std::time::Duration::from_secs(timeout_secs.clamp(2, 30));

    if urls.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| ApiError::Internal(format!("reqwest: {e}")))?;

    let mut results = Vec::new();
    for url in urls {
        let trimmed = url.trim().to_string();
        if trimmed.is_empty() {
            results.push(serde_json::json!({
                "url": url,
                "latency": None::<u64>,
                "status": None::<u16>,
                "error": "URL 不能为空"
            }));
            continue;
        }
        match reqwest::Url::parse(&trimmed) {
            Ok(parsed) => {
                let start = std::time::Instant::now();
                let entry = match client.get(parsed).timeout(timeout).send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        serde_json::json!({
                            "url": trimmed,
                            "latency": start.elapsed().as_millis() as u64,
                            "status": status,
                            "error": None::<String>
                        })
                    }
                    Err(err) => {
                        let status = err.status().map(|s| s.as_u16());
                        let error_message = if err.is_timeout() {
                            "请求超时".to_string()
                        } else if err.is_connect() {
                            "连接失败".to_string()
                        } else {
                            err.to_string()
                        };
                        serde_json::json!({
                            "url": trimmed,
                            "latency": None::<u64>,
                            "status": status,
                            "error": error_message
                        })
                    }
                };
                results.push(entry);
            }
            Err(err) => {
                results.push(serde_json::json!({
                    "url": trimmed,
                    "latency": None::<u64>,
                    "status": None::<u16>,
                    "error": format!("URL 无效: {err}")
                }));
            }
        }
    }
    Ok(Value::Array(results))
}

pub async fn query_provider_usage(_args: Value) -> Result<Value> {
    // cc-switch-web does not yet execute usage scripts (requires a JS runtime).
    // Return a valid UsageResult shape so the UI does not treat the response
    // as a failure; the empty data array means no usage badge is shown.
    Ok(serde_json::json!({
        "success": true,
        "data": Vec::<serde_json::Value>::new(),
    }))
}

pub async fn test_usage_script(_args: Value) -> Result<Value> {
    Ok(serde_json::json!({
        "success": false,
        "error": "用量查询脚本在 cc-switch-web 中尚未支持"
    }))
}

pub async fn fetch_models_for_config(args: Value) -> Result<Value> {
    let base_url: String = super::require_arg(&args, "baseUrl")?;
    let api_key: String = super::optional_arg(&args, "apiKey").unwrap_or_default();
    let is_full_url: bool = super::optional_arg(&args, "isFullUrl").unwrap_or(false);
    let models_url: Option<String> = super::optional_arg(&args, "modelsUrl");
    let custom_user_agent: Option<String> = super::optional_arg(&args, "customUserAgent");

    let models = fetch_models(
        &base_url,
        &api_key,
        is_full_url,
        models_url.as_deref(),
        custom_user_agent.as_deref(),
    )
    .await?;
    Ok(serde_json::to_value(&models)?)
}

/// Fetched model entry returned to the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchedModel {
    id: String,
    owned_by: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ModelsResponse {
    data: Option<Vec<ModelEntry>>,
}

#[derive(Debug, serde::Deserialize)]
struct ModelEntry {
    id: String,
    owned_by: Option<String>,
}

const FETCH_TIMEOUT_SECS: u64 = 15;
const ERROR_BODY_MAX_CHARS: usize = 512;

const KNOWN_COMPAT_SUFFIXES: &[&str] = &[
    "/api/claudecode",
    "/api/anthropic",
    "/apps/anthropic",
    "/api/coding",
    "/claudecode",
    "/anthropic",
    "/step_plan",
    "/coding",
    "/claude",
];

async fn fetch_models(
    base_url: &str,
    api_key: &str,
    is_full_url: bool,
    models_url_override: Option<&str>,
    user_agent: Option<&str>,
) -> Result<Vec<FetchedModel>> {
    if api_key.is_empty() {
        return Err(ApiError::BadArgument {
            field: "apiKey".into(),
            message: "API Key is required to fetch models".into(),
        });
    }

    let candidates = build_models_url_candidates(base_url, is_full_url, models_url_override)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| ApiError::Internal(format!("reqwest: {e}")))?;
    let mut last_err: Option<String> = None;

    for url in &candidates {
        log::debug!("[ModelFetch] Trying endpoint: {url}");
        let mut request = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"));
        if let Some(ua) = user_agent {
            request = request.header(reqwest::header::USER_AGENT, ua);
        }
        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => return Err(ApiError::Internal(format!("Request failed: {e}"))),
        };

        let status = response.status();

        if status.is_success() {
            let resp: ModelsResponse = response
                .json()
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to parse response: {e}")))?;

            let mut models: Vec<FetchedModel> = resp
                .data
                .unwrap_or_default()
                .into_iter()
                .map(|m| FetchedModel {
                    id: m.id,
                    owned_by: m.owned_by,
                })
                .collect();

            models.sort_by(|a, b| a.id.cmp(&b.id));
            return Ok(models);
        }

        if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
            let body = truncate_body(response.text().await.unwrap_or_default());
            last_err = Some(format!("HTTP {status}: {body}"));
            continue;
        }

        let body = truncate_body(response.text().await.unwrap_or_default());
        return Err(ApiError::Internal(format!("HTTP {status}: {body}")));
    }

    Err(ApiError::Internal(format!(
        "All candidates failed: {}",
        last_err.unwrap_or_else(|| "no candidates".to_string())
    )))
}

fn build_models_url_candidates(
    base_url: &str,
    is_full_url: bool,
    models_url_override: Option<&str>,
) -> Result<Vec<String>> {
    if let Some(raw) = models_url_override {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(vec![trimmed.to_string()]);
        }
    }

    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(ApiError::BadArgument {
            field: "baseUrl".into(),
            message: "Base URL is empty".into(),
        });
    }

    let mut candidates: Vec<String> = Vec::new();

    if is_full_url {
        if let Some(idx) = trimmed.find("/v1/") {
            candidates.push(format!("{}/v1/models", &trimmed[..idx]));
        } else if let Some(idx) = trimmed.rfind('/') {
            let root = &trimmed[..idx];
            if root.contains("://") && root.len() > root.find("://").unwrap() + 3 {
                candidates.push(format!("{root}/v1/models"));
            }
        }
        if candidates.is_empty() {
            return Err(ApiError::BadArgument {
                field: "baseUrl".into(),
                message: "Cannot derive models endpoint from full URL".into(),
            });
        }
        return Ok(candidates);
    }

    if ends_with_version_segment(trimmed) {
        candidates.push(format!("{trimmed}/models"));
        if !trimmed.ends_with("/v1") {
            candidates.push(format!("{trimmed}/v1/models"));
        }
    } else {
        candidates.push(format!("{trimmed}/v1/models"));
    }

    if let Some(stripped) = strip_compat_suffix(trimmed) {
        let root = stripped.trim_end_matches('/');
        if !root.is_empty() && root.contains("://") {
            candidates.push(format!("{root}/v1/models"));
            candidates.push(format!("{root}/models"));
        }
    }

    let mut unique: Vec<String> = Vec::with_capacity(candidates.len());
    for url in candidates {
        if !unique.iter().any(|u| u == &url) {
            unique.push(url);
        }
    }

    Ok(unique)
}

fn truncate_body(body: String) -> String {
    if body.chars().count() <= ERROR_BODY_MAX_CHARS {
        body
    } else {
        let mut s: String = body.chars().take(ERROR_BODY_MAX_CHARS).collect();
        s.push('…');
        s
    }
}

fn strip_compat_suffix(base_url: &str) -> Option<&str> {
    for suffix in KNOWN_COMPAT_SUFFIXES {
        if base_url.ends_with(*suffix) {
            return Some(&base_url[..base_url.len() - suffix.len()]);
        }
    }
    None
}

fn ends_with_version_segment(url: &str) -> bool {
    let last = url.rsplit('/').next().unwrap_or("");
    last.strip_prefix('v')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

pub async fn get_claude_common_config_snippet(_args: Value) -> Result<Value> {
    // The upstream `get_claude_common_config_snippet` takes `State<AppState>`.
    // On the headless server we return null/empty so the UI shows nothing
    // in the snippet editor until the user explicitly saves a value.
    Ok(Value::Null)
}

pub async fn set_claude_common_config_snippet(args: Value) -> Result<Value> {
    let _snippet: String = require_arg(&args, "snippet")?;
    let _enabled: Option<bool> = super::optional_arg(&args, "enabled");
    Ok(Value::Bool(true))
}

pub async fn get_common_config_snippet(_args: Value) -> Result<Value> {
    // Accept both `app` and `appType`; upstream uses `appType`.
    Ok(Value::Null)
}

pub async fn set_common_config_snippet(args: Value) -> Result<Value> {
    let _app = super::require_app_str(&args)?;
    let _snippet: String = require_arg(&args, "snippet")?;
    let _enabled: Option<bool> = super::optional_arg(&args, "enabled");
    Ok(Value::Bool(true))
}

pub async fn apply_claude_plugin_config(args: Value) -> Result<Value> {
    let _config: Value = require_arg(&args, "config")?;
    log::info!("apply_claude_plugin_config: no-op on headless server");
    Ok(Value::Bool(true))
}

pub async fn apply_claude_onboarding_skip(_args: Value) -> Result<Value> {
    log::info!("apply_claude_onboarding_skip: no-op on headless server");
    Ok(Value::Bool(true))
}

pub async fn clear_claude_onboarding_skip(_args: Value) -> Result<Value> {
    log::info!("clear_claude_onboarding_skip: no-op on headless server");
    Ok(Value::Bool(true))
}

pub async fn ensure_claude_desktop_official_provider(
    _ctx: &Arc<AppContext>,
    _args: Value,
) -> Result<Value> {
    log::info!("ensure_claude_desktop_official_provider: no-op on headless server");
    Ok(Value::Bool(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_with_version_segment_detects_v1() {
        assert!(ends_with_version_segment("https://x.com/v1"));
        assert!(ends_with_version_segment(
            "https://example.com/api/coding/paas/v4"
        ));
        assert!(ends_with_version_segment("https://example.com/v10"));
    }

    #[test]
    fn ends_with_version_segment_rejects_non_version() {
        assert!(!ends_with_version_segment("https://x.com/api"));
        assert!(!ends_with_version_segment("https://x.com/vX"));
        assert!(!ends_with_version_segment("https://x.com/models"));
        assert!(!ends_with_version_segment("https://api.siliconflow.cn"));
    }

    #[test]
    fn build_candidates_plain_root() {
        let c = build_models_url_candidates("https://api.siliconflow.cn", false, None).unwrap();
        assert_eq!(c, vec!["https://api.siliconflow.cn/v1/models"]);
    }

    #[test]
    fn build_candidates_trailing_slash() {
        let c = build_models_url_candidates("https://api.example.com/", false, None).unwrap();
        assert_eq!(c, vec!["https://api.example.com/v1/models"]);
    }

    #[test]
    fn build_candidates_with_v1() {
        let c = build_models_url_candidates("https://api.example.com/v1", false, None).unwrap();
        assert_eq!(c, vec!["https://api.example.com/v1/models"]);
    }

    #[test]
    fn build_candidates_zhipu_coding_paas_v4() {
        let c =
            build_models_url_candidates("https://open.bigmodel.cn/api/coding/paas/v4", false, None)
                .unwrap();
        assert_eq!(
            c,
            vec![
                "https://open.bigmodel.cn/api/coding/paas/v4/models",
                "https://open.bigmodel.cn/api/coding/paas/v4/v1/models",
            ]
        );
    }

    #[test]
    fn build_candidates_full_url() {
        let c = build_models_url_candidates(
            "https://proxy.example.com/v1/chat/completions",
            true,
            None,
        )
        .unwrap();
        assert_eq!(c, vec!["https://proxy.example.com/v1/models"]);
    }

    #[test]
    fn build_candidates_override_returns_single() {
        let c = build_models_url_candidates(
            "https://api.deepseek.com/anthropic",
            false,
            Some("https://api.deepseek.com/models"),
        )
        .unwrap();
        assert_eq!(c, vec!["https://api.deepseek.com/models"]);
    }

    #[test]
    fn build_candidates_deepseek_strip_anthropic() {
        let c =
            build_models_url_candidates("https://api.deepseek.com/anthropic", false, None).unwrap();
        assert_eq!(
            c,
            vec![
                "https://api.deepseek.com/anthropic/v1/models",
                "https://api.deepseek.com/v1/models",
                "https://api.deepseek.com/models",
            ]
        );
    }

    #[test]
    fn build_candidates_longer_suffix_wins() {
        let c = build_models_url_candidates("https://api.z.ai/api/anthropic", false, None).unwrap();
        assert_eq!(
            c,
            vec![
                "https://api.z.ai/api/anthropic/v1/models",
                "https://api.z.ai/v1/models",
                "https://api.z.ai/models",
            ]
        );
    }

    #[test]
    fn build_candidates_empty_base_url_fails() {
        assert!(build_models_url_candidates("", false, None).is_err());
    }
}
