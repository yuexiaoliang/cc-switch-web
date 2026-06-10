//! Tauri-style command dispatch.
//!
//! Each `#[tauri::command]` in the upstream tree has a corresponding
//! handler here. The handler does one of three things:
//!
//!   1. Forward to a public upstream service (`ProviderService`,
//!      `ProxyService`) - preferred for anything that has a fully
//!      public type signature.
//!   2. Forward to a re-exported Tauri command function (e.g.
//!      `get_settings`, `save_settings`) that does not actually need
//!      a Tauri runtime - they are plain async functions and the
//!      `#[tauri::command]` attribute is a no-op at call time.
//!   3. Fall through to a thin local shim (a direct DB write or a
//!      minimal implementation) for the cases where the upstream
//!      type is in a private module and we cannot name it from
//!      outside.

use crate::error::{ApiError, Result};
use crate::events::FrontendEvent;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Default, Deserialize)]
pub struct InvokeRequest {
    #[serde(default)]
    pub args: Option<Value>,
}

pub async fn invoke(
    State(ctx): State<Arc<crate::AppContext>>,
    Path(cmd): Path<String>,
    Json(req): Json<InvokeRequest>,
) -> std::result::Result<Json<Value>, ApiError> {
    let started = Instant::now();
    let cmd_name = cmd.clone();
    let args = req.args.unwrap_or(Value::Null);

    let result = dispatch(&cmd, &ctx, args).await;

    let elapsed = started.elapsed();
    match &result {
        Ok(_value) => log::debug!(
            target: "cc_switch_mini.dispatch",
            "{cmd_name} ok in {elapsed:?}"
        ),
        Err(err) => log::warn!(
            target: "cc_switch_mini.dispatch",
            "{cmd_name} failed in {elapsed:?}: {err}"
        ),
    }
    result.map(Json)
}

pub async fn health(State(ctx): State<Arc<crate::AppContext>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "subscribers": ctx.events.receiver_count(),
        "bind": ctx.opts.bind_addr.to_string(),
        "data_dir": ctx.opts.data_dir.display().to_string(),
    }))
}

pub async fn version() -> Json<Value> {
    Json(json!({
        "name": "cc-switch-mini",
        "version": env!("CARGO_PKG_VERSION"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
    }))
}

async fn dispatch(cmd: &str, ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
    match cmd {
        "get_providers" => provider::get_providers(ctx, args).await,
        "get_current_provider" => provider::get_current_provider(ctx, args).await,
        "add_provider" => provider::add_provider(ctx, args).await,
        "update_provider" => provider::update_provider(ctx, args).await,
        "delete_provider" => provider::delete_provider(ctx, args).await,
        "remove_provider_from_live_config" => provider::remove_from_live(ctx, args).await,
        "switch_provider" => provider::switch(ctx, args).await,
        "update_providers_sort_order" => provider::update_sort(ctx, args).await,
        "import_default_config" => provider::import_default(ctx, args).await,

        "get_settings" => settings::get(ctx).await,
        "save_settings" => settings::save(ctx, args).await,

        "start_proxy_server" => proxy::start(ctx).await,
        "stop_proxy_with_restore" => proxy::stop_with_restore(ctx).await,
        "get_proxy_status" => proxy::status(ctx).await,
        "is_proxy_running" => proxy::is_running(ctx).await,
        "get_proxy_config" => proxy::get_config(ctx).await,
        "update_proxy_config" => proxy::update_config(ctx, args).await,

        "stream_check_provider" => stream_check::one(ctx, args).await,
        "stream_check_all_providers" => stream_check::all(ctx, args).await,
        "get_stream_check_config" => stream_check::get_config(ctx).await,
        "save_stream_check_config" => stream_check::save_config(ctx, args).await,

        "open_external" => frontend::open_external(args).await,
        "get_config_dir" => frontend::get_config_dir(ctx, args).await,
        "get_app_config_path" => frontend::get_app_config_path(ctx).await,
        "get_tool_versions" => frontend::get_tool_versions(args).await,

        "get_auto_launch_status" => Ok(json!(false)),
        "set_auto_launch" => Ok(json!(true)),
        "is_portable_mode" => Ok(json!(false)),
        "restart_app" => Ok(json!(true)),
        "check_for_updates" => Ok(json!(true)),
        "update_tray_menu" => Ok(json!(true)),
        "open_app_config_folder" => Ok(json!(true)),
        "open_config_folder" => Ok(json!(true)),
        "open_file_dialog" => Ok(Value::Null),
        "save_file_dialog" => Ok(Value::Null),

        _ => Err(ApiError::UnknownCommand(cmd.to_string())),
    }
}

mod provider {
    use super::*;

    pub async fn get_providers(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let providers = cc_switch_lib::ProviderService::list(&ctx.state, app.clone())
            .map_err(ApiError::from)?;
        Ok(serde_json::to_value(&providers)?)
    }

    pub async fn get_current_provider(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let id =
            cc_switch_lib::ProviderService::current(&ctx.state, app).map_err(ApiError::from)?;
        Ok(Value::String(id))
    }

    pub async fn add_provider(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let provider: cc_switch_lib::Provider = require_arg(&args, "provider")?;
        let add_to_live = optional_arg(&args, "addToLive").unwrap_or(true);
        cc_switch_lib::ProviderService::add(&ctx.state, app, provider, add_to_live)
            .map_err(ApiError::from)?;
        Ok(json!(true))
    }

    pub async fn update_provider(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let provider: cc_switch_lib::Provider = require_arg(&args, "provider")?;
        let original_id: Option<String> = optional_arg(&args, "originalId");
        cc_switch_lib::ProviderService::update(&ctx.state, app, original_id.as_deref(), provider)
            .map_err(ApiError::from)?;
        Ok(json!(true))
    }

    pub async fn delete_provider(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let id: String = require_arg(&args, "id")?;
        cc_switch_lib::ProviderService::delete(&ctx.state, app, &id).map_err(ApiError::from)?;
        Ok(json!(true))
    }

    pub async fn remove_from_live(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let id: String = require_arg(&args, "id")?;
        cc_switch_lib::ProviderService::remove_from_live_config(&ctx.state, app, &id)
            .map_err(ApiError::from)?;
        Ok(json!(true))
    }

    pub async fn switch(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let app_str = app.as_str().to_string();
        let id: String = require_arg(&args, "id")?;
        let result =
            cc_switch_lib::ProviderService::switch(&ctx.state, app, &id).map_err(ApiError::from)?;
        ctx.events.publish(FrontendEvent::ProviderSwitched {
            app_type: app_str,
            provider_id: id,
        });
        Ok(serde_json::to_value(&result)?)
    }

    pub async fn update_sort(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let app_str = app.as_str().to_string();
        let updates: Vec<SortUpdateDto> = require_arg(&args, "updates")?;
        let conn = open_db(&ctx.opts.data_dir)?;
        let tx = conn.unchecked_transaction()?;
        for update in updates {
            tx.execute(
                "UPDATE providers SET sort_index = ?1 \
                 WHERE id = ?2 AND app_type = ?3",
                rusqlite::params![update.sort_index, update.id, app_str],
            )
            .map_err(|e| ApiError::Internal(format!("update sort_index: {e}")))?;
        }
        tx.commit()?;
        Ok(json!(true))
    }

    pub async fn import_default(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let ok = cc_switch_lib::ProviderService::import_default_config(&ctx.state, app)
            .map_err(ApiError::from)?;
        Ok(json!(ok))
    }

    #[derive(Debug, Deserialize)]
    struct SortUpdateDto {
        id: String,
        sort_index: usize,
    }
}

mod settings {
    use super::*;

    pub async fn get(_ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let s = cc_switch_lib::get_settings()
            .await
            .map_err(ApiError::from)?;
        Ok(serde_json::to_value(&s)?)
    }

    pub async fn save(_ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let incoming: cc_switch_lib::AppSettings = require_arg(&args, "settings")?;
        let ok = cc_switch_lib::save_settings(incoming)
            .await
            .map_err(ApiError::from)?;
        Ok(json!(ok))
    }
}

mod proxy {
    use super::*;

    pub async fn start(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let info = ctx
            .state
            .proxy_service
            .start()
            .await
            .map_err(ApiError::from)?;
        Ok(serde_json::to_value(&info)?)
    }

    pub async fn stop_with_restore(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        ctx.state
            .proxy_service
            .stop_with_restore()
            .await
            .map_err(ApiError::from)?;
        Ok(Value::Null)
    }

    pub async fn status(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let s = ctx
            .state
            .proxy_service
            .get_status()
            .await
            .map_err(ApiError::from)?;
        Ok(serde_json::to_value(&s)?)
    }

    pub async fn is_running(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let running = ctx.state.proxy_service.is_running().await;
        Ok(json!(running))
    }

    pub async fn get_config(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let c = ctx
            .state
            .proxy_service
            .get_config()
            .await
            .map_err(ApiError::from)?;
        Ok(serde_json::to_value(&c)?)
    }

    pub async fn update_config(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let patch: Value = require_arg(&args, "config")?;
        let map = patch.as_object().ok_or_else(|| ApiError::BadArgument {
            field: "config".into(),
            message: "config must be a JSON object".into(),
        })?;
        let conn = open_db(&ctx.opts.data_dir)?;
        let tx = conn.unchecked_transaction()?;
        for app in ["claude", "codex", "gemini"] {
            tx.execute(
                "UPDATE proxy_config SET \
                    listen_address   = COALESCE(?1, listen_address), \
                    listen_port      = COALESCE(?2, listen_port), \
                    max_retries      = COALESCE(?3, max_retries), \
                    enable_logging   = COALESCE(?4, enable_logging), \
                    streaming_first_byte_timeout = COALESCE(?5, streaming_first_byte_timeout), \
                    streaming_idle_timeout       = COALESCE(?6, streaming_idle_timeout), \
                    non_streaming_timeout        = COALESCE(?7, non_streaming_timeout) \
                 WHERE app_type = ?8",
                rusqlite::params![
                    map.get("listen_address").and_then(|v| v.as_str()),
                    map.get("listen_port").and_then(|v| v.as_i64()),
                    map.get("max_retries").and_then(|v| v.as_i64()),
                    map.get("enable_logging").and_then(|v| v.as_i64()),
                    map.get("streaming_first_byte_timeout")
                        .and_then(|v| v.as_i64()),
                    map.get("streaming_idle_timeout").and_then(|v| v.as_i64()),
                    map.get("non_streaming_timeout").and_then(|v| v.as_i64()),
                    app,
                ],
            )
            .map_err(|e| ApiError::Internal(format!("update proxy_config: {e}")))?;
        }
        tx.commit()?;
        log::info!("proxy_config patched; restart the proxy to apply");
        Ok(Value::Null)
    }
}

mod stream_check {
    use super::*;
    use std::time::Duration;

    const STREAM_CHECK_CONFIG_KEY: &str = "stream_check_config";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Config {
        pub timeout_secs: u64,
        pub max_retries: u32,
        pub degraded_threshold_ms: u64,
        pub claude_model: String,
        pub codex_model: String,
        pub gemini_model: String,
        #[serde(default = "default_prompt")]
        pub test_prompt: String,
    }

    fn default_prompt() -> String {
        "Who are you?".to_string()
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                timeout_secs: 45,
                max_retries: 2,
                degraded_threshold_ms: 6000,
                claude_model: "claude-haiku-4-5-20251001".to_string(),
                codex_model: "gpt-5.5@low".to_string(),
                gemini_model: "gemini-3.5-flash".to_string(),
                test_prompt: default_prompt(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ProbeResult {
        status: String,
        success: bool,
        message: String,
        response_time_ms: Option<u64>,
        http_status: Option<u16>,
        model_used: String,
        tested_at: i64,
        retry_count: u32,
    }

    fn read_config(conn: &rusqlite::Connection) -> Result<Config> {
        let json: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [STREAM_CHECK_CONFIG_KEY],
                |row| row.get(0),
            )
            .ok();
        match json {
            Some(s) => serde_json::from_str(&s).map_err(ApiError::from),
            None => Ok(Config::default()),
        }
    }

    fn write_config(conn: &rusqlite::Connection, cfg: &Config) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![STREAM_CHECK_CONFIG_KEY, json],
        )
        .map_err(|e| ApiError::Internal(format!("save stream_check_config: {e}")))?;
        Ok(())
    }

    fn resolve_credentials(
        conn: &rusqlite::Connection,
        app: &cc_switch_lib::AppType,
        provider_id: &str,
    ) -> Result<Option<(String, String, String)>> {
        let json: Option<String> = conn
            .query_row(
                "SELECT settings_config FROM providers \
                 WHERE id = ?1 AND app_type = ?2",
                rusqlite::params![provider_id, app.as_str()],
                |row| row.get(0),
            )
            .ok();
        let Some(s) = json else { return Ok(None) };
        let v: Value = serde_json::from_str(&s).map_err(ApiError::from)?;
        let env = v.get("env").and_then(|e| e.as_object());
        let auth = v.get("auth").and_then(|a| a.as_object());
        let base_url = env
            .and_then(|e| e.get("ANTHROPIC_BASE_URL").and_then(|x| x.as_str()))
            .or_else(|| env.and_then(|e| e.get("GOOGLE_GEMINI_BASE_URL").and_then(|x| x.as_str())))
            .map(|s| s.to_string());
        let api_key = env
            .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN").and_then(|x| x.as_str()))
            .or_else(|| env.and_then(|e| e.get("GEMINI_API_KEY").and_then(|x| x.as_str())))
            .or_else(|| auth.and_then(|a| a.get("OPENAI_API_KEY").and_then(|x| x.as_str())))
            .map(|s| s.to_string());
        let name = v
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| provider_id.to_string());
        Ok(match (base_url, api_key) {
            (Some(b), Some(k)) => Some((name, b, k)),
            _ => None,
        })
    }

    pub async fn one(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let provider_id: String = require_arg(&args, "providerId")?;
        // Scope the connection so it is dropped before we hit the network
        // call below. `rusqlite::Connection` is `!Send` by default and
        // holding it across an `.await` would make the future `!Send`,
        // which then breaks the axum `Handler` trait.
        let (name, base_url, api_key, cfg) = {
            let conn = open_db(&ctx.opts.data_dir)?;
            let cfg = read_config(&conn)?;
            match resolve_credentials(&conn, &app, &provider_id)? {
                Some(p) => (p.0, p.1, p.2, cfg),
                None => {
                    return Ok(serde_json::to_value(&ProbeResult {
                        status: "failed".into(),
                        success: false,
                        message: format!("no credentials for provider {provider_id}"),
                        response_time_ms: None,
                        http_status: None,
                        model_used: String::new(),
                        tested_at: chrono::Utc::now().timestamp(),
                        retry_count: 0,
                    })?);
                }
            }
        };
        let result = probe_provider(&name, &base_url, &api_key, &cfg, app.as_str()).await;
        Ok(serde_json::to_value(&result)?)
    }

    pub async fn all(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        // Collect (provider_id, credentials) and the global config up
        // front so we can drop the sqlite connection before the
        // network calls start. See `one` for the full reasoning.
        let mut probes: Vec<(String, String, String, String)> = Vec::new();
        let cfg = {
            let conn = open_db(&ctx.opts.data_dir)?;
            let cfg = read_config(&conn)?;
            let mut stmt = conn
                .prepare("SELECT id FROM providers WHERE app_type = ?1")
                .map_err(|e| ApiError::Internal(format!("query providers: {e}")))?;
            let ids: Vec<String> = stmt
                .query_map([app.as_str()], |row| row.get::<_, String>(0))
                .map_err(|e| ApiError::Internal(format!("read providers: {e}")))?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);
            for id in ids {
                if let Some((name, base_url, api_key)) = resolve_credentials(&conn, &app, &id)? {
                    probes.push((id, name, base_url, api_key));
                }
            }
            cfg
        };
        let mut results = Vec::new();
        for (id, name, base_url, api_key) in probes {
            let r = probe_provider(&name, &base_url, &api_key, &cfg, app.as_str()).await;
            results.push((id, r));
        }
        Ok(serde_json::to_value(&results)?)
    }

    pub async fn get_config(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let conn = open_db(&ctx.opts.data_dir)?;
        let cfg = read_config(&conn)?;
        Ok(serde_json::to_value(&cfg)?)
    }

    pub async fn save_config(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let cfg: Config = require_arg(&args, "config")?;
        let conn = open_db(&ctx.opts.data_dir)?;
        write_config(&conn, &cfg)?;
        Ok(Value::Null)
    }

    async fn probe_provider(
        name: &str,
        base_url: &str,
        api_key: &str,
        cfg: &Config,
        app: &str,
    ) -> ProbeResult {
        let started = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()
            .unwrap_or_default();
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        let model = match app {
            "claude" => &cfg.claude_model,
            "codex" => &cfg.codex_model,
            "gemini" => &cfg.gemini_model,
            _ => &cfg.claude_model,
        };
        let body = json!({
            "model": model,
            "max_tokens": 16,
            "messages": [{"role": "user", "content": &cfg.test_prompt}],
        });
        let resp = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await;
        let elapsed = started.elapsed().as_millis() as u64;
        match resp {
            Ok(r) => {
                let status = r.status().as_u16();
                let ok = r.status().is_success();
                ProbeResult {
                    status: if ok { "operational" } else { "failed" }.into(),
                    success: ok,
                    message: if ok {
                        format!("{name} healthy (HTTP {status})")
                    } else {
                        format!("{name} returned HTTP {status}")
                    },
                    response_time_ms: Some(elapsed),
                    http_status: Some(status),
                    model_used: model.to_string(),
                    tested_at: chrono::Utc::now().timestamp(),
                    retry_count: 0,
                }
            }
            Err(e) => ProbeResult {
                status: "failed".into(),
                success: false,
                message: format!("{name}: {e}"),
                response_time_ms: Some(elapsed),
                http_status: None,
                model_used: model.to_string(),
                tested_at: chrono::Utc::now().timestamp(),
                retry_count: 0,
            },
        }
    }
}

mod frontend {
    use super::*;

    pub async fn open_external(args: Value) -> Result<Value> {
        let url: String = require_arg(&args, "url")?;
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(ApiError::BadArgument {
                field: "url".into(),
                message: "only http(s) URLs are allowed".into(),
            });
        }
        log::info!("open_external: {url}");
        Ok(json!(true))
    }

    pub async fn get_config_dir(ctx: &Arc<crate::AppContext>, args: Value) -> Result<Value> {
        let app = require_app(&args)?;
        let path = match app {
            cc_switch_lib::AppType::Claude => dirs::home_dir()
                .map(|h| h.join(".claude"))
                .unwrap_or_default(),
            cc_switch_lib::AppType::Codex => cc_switch_lib::get_codex_config_path()
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default(),
            cc_switch_lib::AppType::Gemini => dirs::home_dir()
                .map(|h| h.join(".gemini"))
                .unwrap_or_default(),
            cc_switch_lib::AppType::OpenCode => dirs::home_dir()
                .map(|h| h.join(".config").join("opencode"))
                .unwrap_or_default(),
            cc_switch_lib::AppType::OpenClaw => dirs::home_dir()
                .map(|h| h.join(".openclaw"))
                .unwrap_or_default(),
            cc_switch_lib::AppType::Hermes => dirs::home_dir()
                .map(|h| h.join(".hermes"))
                .unwrap_or_default(),
            cc_switch_lib::AppType::ClaudeDesktop => dirs::home_dir()
                .map(|h| h.join(".cc-switch-desktop"))
                .unwrap_or_default(),
        };
        let path_str = if path.as_os_str().is_empty() {
            ctx.opts.data_dir.join(app.as_str()).display().to_string()
        } else {
            path.display().to_string()
        };
        Ok(Value::String(path_str))
    }

    pub async fn get_app_config_path(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let upstream = cc_switch_lib::get_app_config_path()
            .await
            .map_err(ApiError::from)?;
        let path = if upstream.is_empty() {
            ctx.opts.data_dir.join(".cc-switch").join("config.json")
        } else {
            std::path::PathBuf::from(upstream)
        };
        Ok(Value::String(path.display().to_string()))
    }

    pub async fn get_tool_versions(_args: Value) -> Result<Value> {
        Ok(json!([]))
    }
}

fn require_app(args: &Value) -> Result<cc_switch_lib::AppType> {
    let app_str: String = require_arg(args, "app")?;
    cc_switch_lib::AppType::from_str(&app_str).map_err(|e| ApiError::BadArgument {
        field: "app".into(),
        message: e.to_string(),
    })
}

fn require_arg<T: for<'de> Deserialize<'de>>(args: &Value, field: &str) -> Result<T> {
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

fn optional_arg<T: for<'de> Deserialize<'de>>(args: &Value, field: &str) -> Option<T> {
    args.as_object()
        .and_then(|o| o.get(field))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

fn open_db(data_dir: &std::path::Path) -> Result<rusqlite::Connection> {
    let path = data_dir.join(".cc-switch").join("cc-switch.db");
    rusqlite::Connection::open(&path).map_err(|e| ApiError::Internal(format!("open {path:?}: {e}")))
}
