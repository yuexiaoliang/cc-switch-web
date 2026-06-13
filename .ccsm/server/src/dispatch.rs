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

use crate::commands_extra::{
    auth, claude, deeplink, failover, mcp, omo, openclaw, opencode, pricing, prompt,
    session, skill, sync, tools, usage,
};
use crate::commands_extra::{provider as ext_provider, proxy as ext_proxy};

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

        // Hermes-specific commands (spec 6.1 coverage)
        "get_hermes_model_config" => hermes::get_model_config().await,
        "open_hermes_web_ui" => hermes::open_web_ui(args).await,
        "launch_hermes_dashboard" => hermes::launch_dashboard().await,
        "get_hermes_memory" => hermes::get_memory(args).await,
        "set_hermes_memory" => hermes::set_memory(args).await,
        "get_hermes_memory_limits" => hermes::get_memory_limits().await,
        "set_hermes_memory_enabled" => hermes::set_memory_enabled(args).await,
        "get_hermes_live_provider_ids" => hermes::get_live_provider_ids().await,
        "get_hermes_live_provider" => hermes::get_live_provider(args).await,
        "import_hermes_providers_from_live" => hermes::import_from_live(ctx).await,

        "get_auto_launch_status" => Ok(json!(false)),
        "set_auto_launch" => Ok(json!(true)),
        "is_portable_mode" => Ok(json!(false)),
        "restart_app" => Ok(json!(true)),
        "check_for_updates" => Ok(json!(true)),
        "install_update_and_restart" => Ok(json!(false)),
        "update_tray_menu" => Ok(json!(true)),
        "open_app_config_folder" => Ok(json!(true)),
        "open_config_folder" => Ok(json!(true)),
        "open_file_dialog" => Ok(Value::Null),
        "save_file_dialog" => Ok(Value::Null),

        // Initialization / migration stubs (spec 6.2 no-ops)
        "get_init_error" => Ok(Value::Null),
        "get_migration_result" => Ok(json!(false)),
        "get_skills_migration_result" => Ok(Value::Null),
        "check_env_conflicts" => Ok(json!([])),
        
        "get_claude_code_config_path" => frontend::get_claude_code_config_path(ctx).await,
        
        
        
        
        
        "set_window_theme" => Ok(Value::Null),

        // ---- usage / pricing ----
        "get_usage_summary" => usage::get_usage_summary(ctx, args).await,
        "get_usage_summary_by_app" => usage::get_usage_summary_by_app(ctx, args).await,
        "get_usage_trends" => usage::get_usage_trends(ctx, args).await,
        "get_provider_stats" => usage::get_provider_stats(ctx, args).await,
        "get_model_stats" => usage::get_model_stats(ctx, args).await,
        "get_request_logs" => usage::get_request_logs(ctx, args).await,
        "get_request_detail" => usage::get_request_detail(ctx, args).await,
        "check_provider_limits" => usage::check_provider_limits(ctx, args).await,
        "sync_session_usage" => usage::sync_session_usage(ctx).await,
        "get_usage_data_sources" => usage::get_usage_data_sources(ctx).await,
        "get_model_pricing" => pricing::get_model_pricing(ctx).await,
        "update_model_pricing" => pricing::update_model_pricing(ctx, args).await,
        "delete_model_pricing" => pricing::delete_model_pricing(ctx, args).await,
        "queryProviderUsage" => ext_provider::query_provider_usage(args).await,
        "testUsageScript" => ext_provider::test_usage_script(args).await,
        "get_provider_health" => omo::get_provider_health(args).await,
        "get_subscription_quota" => tools::get_balance(args).await,

        // ---- MCP ----
        "get_mcp_servers" => mcp::get_mcp_servers(ctx).await,
        "get_mcp_config" => mcp::get_mcp_config(ctx, args).await,
        "upsert_mcp_server" => mcp::upsert_mcp_server(ctx, args).await,
        "delete_mcp_server" => mcp::delete_mcp_server(ctx, args).await,
        "toggle_mcp_app" => mcp::toggle_mcp_app(ctx, args).await,
        "set_mcp_enabled" => mcp::set_mcp_enabled(ctx, args).await,
        "import_mcp_from_apps" => mcp::import_mcp_from_apps(ctx).await,
        "upsert_mcp_server_in_config" => mcp::upsert_mcp_server_in_config(ctx, args).await,
        "delete_mcp_server_in_config" => mcp::delete_mcp_server_in_config(ctx, args).await,
        "get_claude_mcp_status" => mcp::get_claude_mcp_status().await,
        "read_claude_mcp_config" => mcp::read_claude_mcp_config().await,
        "upsert_claude_mcp_server" => mcp::upsert_claude_mcp_server(args).await,
        "delete_claude_mcp_server" => mcp::delete_claude_mcp_server(args).await,
        "validate_mcp_command" => mcp::validate_mcp_command(args).await,

        // ---- prompts ----
        "get_prompts" => prompt::get_prompts(ctx, args).await,
        "upsert_prompt" => prompt::upsert_prompt(ctx, args).await,
        "delete_prompt" => prompt::delete_prompt(ctx, args).await,
        "enable_prompt" => prompt::enable_prompt(ctx, args).await,
        "import_prompt_from_file" => prompt::import_prompt_from_file(ctx, args).await,
        "get_current_prompt_file_content" => prompt::get_current_prompt_file_content(args).await,

        // ---- skills ----
        "get_installed_skills" => skill::get_installed_skills(ctx).await,
        "get_skill_backups" => skill::get_skill_backups().await,
        "delete_skill_backup" => skill::delete_skill_backup(args).await,
        "install_skill_unified" => skill::install_skill_unified(ctx, args).await,
        "uninstall_skill_unified" => skill::uninstall_skill_unified(ctx, args).await,
        "restore_skill_backup" => skill::restore_skill_backup(ctx, args).await,
        "toggle_skill_app" => skill::toggle_skill_app(ctx, args).await,
        "scan_unmanaged_skills" => skill::scan_unmanaged_skills(ctx).await,
        "import_skills_from_apps" => skill::import_skills_from_apps(ctx, args).await,
        "discover_available_skills" => skill::discover_available_skills(ctx).await,
        "check_skill_updates" => skill::check_skill_updates(ctx).await,
        "update_skill" => skill::update_skill(ctx, args).await,
        "migrate_skill_storage" => skill::migrate_skill_storage(ctx, args).await,
        "search_skills_sh" => skill::search_skills_sh(args).await,
        "get_skills" => skill::get_skills(ctx).await,
        "get_skills_for_app" => skill::get_skills_for_app(ctx, args).await,
        "install_skill" => skill::install_skill(ctx, args).await,
        "install_skill_for_app" => skill::install_skill_for_app(ctx, args).await,
        "uninstall_skill" => skill::uninstall_skill(ctx, args).await,
        "uninstall_skill_for_app" => skill::uninstall_skill_for_app(ctx, args).await,
        "get_skill_repos" => skill::get_skill_repos(ctx).await,
        "add_skill_repo" => skill::add_skill_repo(ctx, args).await,
        "remove_skill_repo" => skill::remove_skill_repo(ctx, args).await,
        "install_skills_from_zip" => skill::install_skills_from_zip(ctx, args).await,

        // ---- providers (extended) ----
        "get_universal_providers" => ext_provider::get_universal_providers(ctx).await,
        "get_universal_provider" => ext_provider::get_universal_provider(ctx, args).await,
        "upsert_universal_provider" => ext_provider::upsert_universal_provider(ctx, args).await,
        "delete_universal_provider" => ext_provider::delete_universal_provider(ctx, args).await,
        "sync_universal_provider" => ext_provider::sync_universal_provider(ctx, args).await,
        "sync_current_providers_live" => ext_provider::sync_current_providers_live(ctx).await,
        "get_custom_endpoints" => ext_provider::get_custom_endpoints(ctx, args).await,
        "add_custom_endpoint" => ext_provider::add_custom_endpoint(ctx, args).await,
        "remove_custom_endpoint" => ext_provider::remove_custom_endpoint(ctx, args).await,
        "update_endpoint_last_used" => ext_provider::update_endpoint_last_used(ctx, args).await,
        "read_live_provider_settings" => ext_provider::read_live_provider_settings(args).await,
        "import_default_config" => ext_provider::import_default_config(ctx, args).await,
        "test_api_endpoints" => ext_provider::test_api_endpoints(args).await,
        "fetch_models_for_config" => ext_provider::fetch_models_for_config(args).await,
        "get_claude_common_config_snippet" => ext_provider::get_claude_common_config_snippet(args).await,
        "set_claude_common_config_snippet" => ext_provider::set_claude_common_config_snippet(args).await,
        "get_common_config_snippet" => ext_provider::get_common_config_snippet(args).await,
        "set_common_config_snippet" => ext_provider::set_common_config_snippet(args).await,
        "apply_claude_plugin_config" => ext_provider::apply_claude_plugin_config(args).await,
        "apply_claude_onboarding_skip" => ext_provider::apply_claude_onboarding_skip(args).await,
        "clear_claude_onboarding_skip" => ext_provider::clear_claude_onboarding_skip(args).await,
        "ensure_claude_desktop_official_provider" => ext_provider::ensure_claude_desktop_official_provider(ctx, args).await,

        // ---- proxy (extended) ----
        "get_global_proxy_config" => ext_proxy::get_global_proxy_config(ctx).await,
        "update_global_proxy_config" => ext_proxy::update_global_proxy_config(ctx, args).await,
        "set_global_proxy_url" => ext_proxy::set_global_proxy_url(ctx, args).await,
        "get_proxy_config_for_app" => ext_proxy::get_proxy_config_for_app(ctx, args).await,
        "update_proxy_config_for_app" => ext_proxy::update_proxy_config_for_app(ctx, args).await,
        "set_proxy_takeover_for_app" => ext_proxy::set_proxy_takeover_for_app(ctx, args).await,
        "is_live_takeover_active" => ext_proxy::is_live_takeover_active(ctx).await,
        "get_proxy_takeover_status" => ext_proxy::get_proxy_takeover_status(ctx).await,
        "switch_proxy_provider" => ext_proxy::switch_proxy_provider(ctx, args).await,
        "get_circuit_breaker_config" => ext_proxy::get_circuit_breaker_config(ctx, args).await,
        "update_circuit_breaker_config" => ext_proxy::update_circuit_breaker_config(ctx, args).await,
        "get_circuit_breaker_stats" => ext_proxy::get_circuit_breaker_stats(ctx, args).await,
        "reset_circuit_breaker" => ext_proxy::reset_circuit_breaker(ctx, args).await,
        "get_default_cost_multiplier" => ext_proxy::get_default_cost_multiplier(ctx, args).await,
        "set_default_cost_multiplier" => ext_proxy::set_default_cost_multiplier(ctx, args).await,
        "get_pricing_model_source" => ext_proxy::get_pricing_model_source(ctx, args).await,
        "set_pricing_model_source" => ext_proxy::set_pricing_model_source(ctx, args).await,

        // ---- failover ----
        "get_failover_queue" => failover::get_failover_queue(ctx, args).await,
        "add_to_failover_queue" => failover::add_to_failover_queue(ctx, args).await,
        "remove_from_failover_queue" => failover::remove_from_failover_queue(ctx, args).await,
        "get_auto_failover_enabled" => failover::get_auto_failover_enabled(ctx, args).await,
        "set_auto_failover_enabled" => failover::set_auto_failover_enabled(ctx, args).await,
        "get_available_providers_for_failover" => failover::get_available_providers_for_failover(ctx, args).await,

        // ---- omo ----
        "read_omo_local_file" => omo::read_omo_local_file().await,
        "read_omo_slim_local_file" => omo::read_omo_slim_local_file().await,
        "get_current_omo_provider_id" => omo::get_current_omo_provider_id(ctx).await,
        "get_current_omo_slim_provider_id" => omo::get_current_omo_slim_provider_id(ctx).await,
        "disable_current_omo" => omo::disable_current_omo(ctx).await,
        "disable_current_omo_slim" => omo::disable_current_omo_slim(ctx).await,
        "get_optimizer_config" => omo::get_optimizer_config(args).await,
        "set_optimizer_config" => omo::set_optimizer_config(args).await,
        "get_rectifier_config" => omo::get_rectifier_config(args).await,
        "set_rectifier_config" => omo::set_rectifier_config(args).await,

        // ---- openclaw ----
        "get_openclaw_live_provider_ids" => openclaw::get_openclaw_live_provider_ids().await,
        "get_openclaw_live_provider" => openclaw::get_openclaw_live_provider(args).await,
        "import_openclaw_providers_from_live" => openclaw::import_openclaw_providers_from_live(ctx).await,
        "scan_openclaw_config_health" => openclaw::scan_openclaw_config_health().await,
        "get_openclaw_default_model" => openclaw::get_openclaw_default_model().await,
        "set_openclaw_default_model" => openclaw::set_openclaw_default_model(args).await,
        "get_openclaw_model_catalog" => openclaw::get_openclaw_model_catalog().await,
        "set_openclaw_model_catalog" => openclaw::set_openclaw_model_catalog(args).await,
        "get_openclaw_agents_defaults" => openclaw::get_openclaw_agents_defaults().await,
        "set_openclaw_agents_defaults" => openclaw::set_openclaw_agents_defaults(args).await,
        "get_openclaw_env" => openclaw::get_openclaw_env().await,
        "set_openclaw_env" => openclaw::set_openclaw_env(args).await,
        "get_openclaw_tools" => openclaw::get_openclaw_tools().await,
        "set_openclaw_tools" => openclaw::set_openclaw_tools(args).await,

        // ---- opencode ----
        "get_opencode_live_provider_ids" => opencode::get_opencode_live_provider_ids().await,
        "import_opencode_providers_from_live" => opencode::import_opencode_providers_from_live(ctx).await,

        // ---- sessions ----
        "list_sessions" => session::list_sessions(ctx).await,
        "get_session_messages" => session::get_session_messages(ctx, args).await,
        "delete_session" => session::delete_session(ctx, args).await,
        "delete_sessions" => session::delete_sessions(ctx, args).await,
        "launch_session_terminal" => session::launch_session_terminal(ctx, args).await,

        // ---- deeplink ----
        "parse_deeplink" => deeplink::parse_deeplink(args).await,
        "merge_deeplink_config" => deeplink::merge_deeplink_config(args).await,
        "import_from_deeplink_unified" => deeplink::import_from_deeplink_unified(ctx, args).await,

        // ---- claude ----
        "get_claude_desktop_status" => claude::get_claude_desktop_status().await,
        "get_claude_desktop_default_routes" => claude::get_claude_desktop_default_routes().await,

        // ---- tools ----
        "open_provider_terminal" => tools::open_provider_terminal(ctx, args).await,
        "open_workspace_directory" => tools::open_workspace_directory(ctx, args).await,
        "pick_directory" => tools::pick_directory(args).await,
        "open_zip_file_dialog" => tools::open_zip_file_dialog(args).await,
        "run_tool_lifecycle_action" => tools::run_tool_lifecycle_action(args).await,
        "probe_tool_installations" => tools::probe_tool_installations(args).await,
        "export_config_to_file" => tools::export_config_to_file(args).await,
        "import_config_from_file" => tools::import_config_from_file(args).await,
        "get_balance" => tools::get_balance(args).await,
        "get_codex_oauth_quota" => tools::get_codex_oauth_quota(args).await,
        "get_codex_oauth_models" => tools::get_codex_oauth_models(args).await,
        "get_coding_plan_quota" => tools::get_coding_plan_quota(args).await,
        "get_log_config" => tools::get_log_config(ctx).await,
        "set_log_config" => tools::set_log_config(ctx, args).await,
        "set_app_config_dir_override" => tools::set_app_config_dir_override(ctx, args).await,
        "get_app_config_dir_override" => tools::get_app_config_dir_override().await,

        // ---- db backup / sync ----
        "create_db_backup" => sync::create_db_backup(ctx).await,
        "list_db_backups" => sync::list_db_backups(ctx).await,
        "delete_db_backup" => sync::delete_db_backup(ctx, args).await,
        "restore_db_backup" => sync::restore_db_backup(ctx, args).await,
        "rename_db_backup" => sync::rename_db_backup(ctx, args).await,
        "s3_sync_save_settings" => sync::s3_sync_save_settings(ctx, args).await,
        "s3_test_connection" => sync::s3_test_connection(args).await,
        "s3_sync_upload" => sync::s3_sync_upload(args).await,
        "s3_sync_download" => sync::s3_sync_download(args).await,
        "s3_sync_fetch_remote_info" => sync::s3_sync_fetch_remote_info(args).await,
        "webdav_sync_save_settings" => sync::webdav_sync_save_settings(ctx, args).await,
        "webdav_test_connection" => sync::webdav_test_connection(args).await,
        "webdav_sync_upload" => sync::webdav_sync_upload(args).await,
        "webdav_sync_download" => sync::webdav_sync_download(args).await,
        "webdav_sync_fetch_remote_info" => sync::webdav_sync_fetch_remote_info(args).await,

        // ---- auth / oauth stubs (no OAuth manager on headless) ----
        "auth_get_status" => auth::auth_get_status(args).await,
        "auth_start_login" => auth::auth_start_login(args).await,
        "auth_poll_for_account" => auth::auth_poll_for_account(args).await,
        "auth_list_accounts" => auth::auth_list_accounts(args).await,
        "auth_logout" => auth::auth_logout(args).await,
        "auth_remove_account" => auth::auth_remove_account(args).await,
        "auth_set_default_account" => auth::auth_set_default_account(args).await,
        "copilot_logout" => auth::copilot_logout(args).await,
        "copilot_remove_account" => auth::copilot_remove_account(args).await,
        "copilot_set_default_account" => auth::copilot_set_default_account(args).await,
        "import_claude_desktop_providers_from_claude" => auth::import_claude_desktop_providers_from_claude(args).await,

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

    pub async fn get_claude_code_config_path(
        ctx: &Arc<crate::AppContext>,
    ) -> Result<Value> {
        let path = dirs::home_dir()
            .map(|h| h.join(".claude").join("config.json"))
            .unwrap_or_else(|| ctx.opts.data_dir.join(".claude").join("config.json"));
        Ok(Value::String(path.display().to_string()))
    }
}

mod hermes {
    use super::*;

    fn hermes_err(e: impl std::fmt::Display) -> ApiError {
        ApiError::Internal(format!("hermes_config: {e}"))
    }

    pub async fn get_model_config() -> Result<Value> {
        let cfg = cc_switch_lib::hermes_config::get_model_config().map_err(hermes_err)?;
        Ok(cfg.map_or(Value::Null, |v| serde_json::to_value(&v).unwrap_or(Value::Null)))
    }

    pub async fn open_web_ui(args: Value) -> Result<Value> {
        // Headless server: no browser to open. Log the requested path so
        // the operator can navigate there manually.
        let path: Option<String> = optional_arg(&args, "path");
        let port = std::env::var("HERMES_WEB_PORT")
            .ok()
            .and_then(|raw| raw.trim().parse::<u16>().ok())
            .unwrap_or(9119);
        let target = match path.as_deref() {
            Some(p) if p.starts_with('/') => format!("http://127.0.0.1:{port}{p}"),
            Some(p) if !p.is_empty() => format!("http://127.0.0.1:{port}/{p}"),
            _ => format!("http://127.0.0.1:{port}/"),
        };
        log::info!("open_hermes_web_ui: {target} (headless server — open manually)");
        Ok(Value::Null)
    }

    pub async fn launch_dashboard() -> Result<Value> {
        // Headless server: no preferred-terminal launcher available.
        log::info!("launch_hermes_dashboard: run `hermes dashboard` manually on this host");
        Ok(Value::Null)
    }

    pub async fn get_memory(args: Value) -> Result<Value> {
        let kind: cc_switch_lib::hermes_config::MemoryKind = require_arg(&args, "kind")?;
        let content = cc_switch_lib::hermes_config::read_memory(kind).map_err(hermes_err)?;
        Ok(Value::String(content))
    }

    pub async fn set_memory(args: Value) -> Result<Value> {
        let kind: cc_switch_lib::hermes_config::MemoryKind = require_arg(&args, "kind")?;
        let content: String = require_arg(&args, "content")?;
        cc_switch_lib::hermes_config::write_memory(kind, &content).map_err(hermes_err)?;
        Ok(Value::Null)
    }

    pub async fn get_memory_limits() -> Result<Value> {
        let limits = cc_switch_lib::hermes_config::read_memory_limits().map_err(hermes_err)?;
        Ok(serde_json::to_value(&limits).unwrap_or(Value::Null))
    }

    pub async fn set_memory_enabled(args: Value) -> Result<Value> {
        let kind: cc_switch_lib::hermes_config::MemoryKind = require_arg(&args, "kind")?;
        let enabled: bool = require_arg(&args, "enabled")?;
        let outcome = cc_switch_lib::hermes_config::set_memory_enabled(kind, enabled)
            .map_err(hermes_err)?;
        Ok(serde_json::to_value(&outcome).unwrap_or(Value::Null))
    }

    pub async fn get_live_provider_ids() -> Result<Value> {
        let providers = cc_switch_lib::hermes_config::get_providers().map_err(hermes_err)?;
        let ids: Vec<String> = providers.keys().cloned().collect();
        Ok(serde_json::to_value(&ids)?)
    }

    pub async fn get_live_provider(args: Value) -> Result<Value> {
        let id: String = require_arg(&args, "providerId")?;
        let provider = cc_switch_lib::hermes_config::get_provider(&id).map_err(hermes_err)?;
        Ok(provider.map_or(Value::Null, |v| v))
    }

    pub async fn import_from_live(ctx: &Arc<crate::AppContext>) -> Result<Value> {
        let providers = cc_switch_lib::hermes_config::get_providers().map_err(hermes_err)?;
        if providers.is_empty() {
            return Ok(json!(0));
        }
        let existing_ids = ctx
            .state
            .db
            .get_provider_ids("hermes")
            .map_err(hermes_err)?;
        let mut imported = 0usize;
        for (name, config) in providers {
            if name.trim().is_empty() {
                log::warn!("Skipping Hermes provider with empty name");
                continue;
            }
            if existing_ids.contains(&name) {
                log::debug!("Hermes provider '{name}' already exists, skipping");
                continue;
            }
            let mut provider =
                cc_switch_lib::Provider::with_id(name.clone(), name.clone(), config, None);
            provider.meta = Some(cc_switch_lib::ProviderMeta {
                live_config_managed: Some(true),
                ..Default::default()
            });
            if let Err(e) = ctx.state.db.save_provider("hermes", &provider) {
                log::warn!("Failed to import Hermes provider '{name}': {e}");
                continue;
            }
            imported += 1;
            log::info!("Imported Hermes provider '{name}' from live config");
        }
        Ok(json!(imported))
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
