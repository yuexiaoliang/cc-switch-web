//! Skills management.
//!
//! The public `SkillService` is re-exported at the crate root, but the
//! per-command response types (`DiscoverableSkill`, `SkillRepo`,
//! `SkillUninstallResult`, ...) are not. We therefore take JSON inputs
//! and serialise the service responses to `serde_json::Value`. Calls
//! that need to construct typed inputs (e.g. `install_skill_unified`)
//! are routed through the upstream command re-export where possible.
//!
//! For operations where neither path is reachable (the upstream
//! command takes `State<SkillServiceState>` and the service methods
//! require private input types), we provide a no-op success so the
//! UI can be exercised end-to-end.

use super::{require_arg, ApiError, AppContext, Result, Value};
use cc_switch_lib::SkillService;
use std::sync::Arc;

pub async fn get_installed_skills(ctx: &Arc<AppContext>) -> Result<Value> {
    let skills = SkillService::get_all_installed(&ctx.state.db).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&skills)?)
}

pub async fn get_skill_backups() -> Result<Value> {
    let backups = SkillService::list_backups().map_err(ApiError::from)?;
    Ok(serde_json::to_value(&backups)?)
}

pub async fn delete_skill_backup(args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "backupId")?;
    SkillService::delete_backup(&id).map_err(ApiError::from)?;
    Ok(Value::Bool(true))
}

pub async fn install_skill_unified(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    // The upstream command takes `State<SkillServiceState>` and the
    // service method requires a private `DiscoverableSkill` input.
    // Users should install skills via the upstream desktop app, then
    // pull the metadata from `get_installed_skills`.
    log::info!("install_skill_unified: not supported on headless server");
    Err(ApiError::Internal(
        "skill install via the headless server is not yet supported; use the upstream Tauri app for now".into(),
    ))
}

pub async fn uninstall_skill_unified(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let result = SkillService::uninstall(&ctx.state.db, &id).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn restore_skill_backup(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "restore_skill_backup requires the upstream private helper".into(),
    ))
}

pub async fn toggle_skill_app(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let id: String = require_arg(&args, "id")?;
    let app = super::require_app_str(&args)?;
    let enabled: bool = require_arg(&args, "enabled")?;
    SkillService::toggle_app(&ctx.state.db, &id, &app, enabled).map_err(ApiError::from)?;
    Ok(Value::Bool(true))
}

pub async fn scan_unmanaged_skills(ctx: &Arc<AppContext>) -> Result<Value> {
    let unmanaged = SkillService::scan_unmanaged(&ctx.state.db).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&unmanaged)?)
}

pub async fn import_skills_from_apps(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "import_skills_from_apps requires the upstream private helper".into(),
    ))
}

pub async fn discover_available_skills(_ctx: &Arc<AppContext>) -> Result<Value> {
    Err(ApiError::Internal(
        "discover_available_skills requires the upstream private helper".into(),
    ))
}

pub async fn check_skill_updates(_ctx: &Arc<AppContext>) -> Result<Value> {
    Err(ApiError::Internal(
        "check_skill_updates requires the upstream private helper".into(),
    ))
}

pub async fn update_skill(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "update_skill requires the upstream private helper".into(),
    ))
}

pub async fn migrate_skill_storage(ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    // The public `migrate_skills_to_ssot` helper re-routes any legacy
    // skill storage into the new SSOT. We expose that as the only
    // migration path; the typed `SkillStorageLocation` argument is not
    // reachable from outside.
    let n = cc_switch_lib::migrate_skills_to_ssot(&ctx.state.db).map_err(ApiError::from)?;
    Ok(serde_json::json!({ "migrated": n }))
}

pub async fn search_skills_sh(_args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "search_skills_sh requires the upstream private helper".into(),
    ))
}

pub async fn get_skills(_ctx: &Arc<AppContext>) -> Result<Value> {
    Err(ApiError::Internal(
        "get_skills requires the upstream private helper".into(),
    ))
}

pub async fn get_skills_for_app(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "get_skills_for_app requires the upstream private helper".into(),
    ))
}

pub async fn install_skill(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "install_skill requires the upstream private helper".into(),
    ))
}

pub async fn install_skill_for_app(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "install_skill_for_app requires the upstream private helper".into(),
    ))
}

pub async fn uninstall_skill(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    let directory: String = require_arg(&args, "directory")?;
    let skills = SkillService::get_all_installed(&ctx.state.db).map_err(ApiError::from)?;
    let skill = skills
        .into_iter()
        .find(|s| s.directory.eq_ignore_ascii_case(&directory))
        .ok_or_else(|| ApiError::Internal(format!("installed skill not found: {directory}")))?;
    let result = SkillService::uninstall(&ctx.state.db, &skill.id).map_err(ApiError::from)?;
    Ok(serde_json::to_value(&result)?)
}

pub async fn uninstall_skill_for_app(ctx: &Arc<AppContext>, args: Value) -> Result<Value> {
    uninstall_skill(ctx, args).await
}

pub async fn get_skill_repos(_ctx: &Arc<AppContext>) -> Result<Value> {
    Err(ApiError::Internal(
        "get_skill_repos requires the upstream private helper".into(),
    ))
}

pub async fn add_skill_repo(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "add_skill_repo requires the upstream private helper".into(),
    ))
}

pub async fn remove_skill_repo(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "remove_skill_repo requires the upstream private helper".into(),
    ))
}

pub async fn install_skills_from_zip(_ctx: &Arc<AppContext>, _args: Value) -> Result<Value> {
    Err(ApiError::Internal(
        "install_skills_from_zip requires the upstream private helper".into(),
    ))
}
