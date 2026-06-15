//! Application state initialisation.
//!
//! Wraps `cc_switch_lib::AppState` in an `Arc` and applies any environment
//! overrides the user passed on the command line (`--config-dir`).
//!
//! ## Directory layout
//!
//! cc-switch-mini uses the upstream Tauri app layout verbatim. The database
//! and all host-tool configs live at the same paths as the desktop build, so
//! users can switch between the two without data migration:
//!
//! - Database: `~/.cc-switch/cc-switch.db`
//! - Hermes config: `~/.hermes/config.yaml`
//! - Claude config: `~/.claude/`
//! - Codex config: `~/.codex/`
//! - Gemini config: `~/.gemini/`

use crate::cli::Resolved;
use crate::error::{ApiError, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Build the upstream `AppState` and apply CLI overrides.
pub fn build(opts: &Resolved) -> Result<Arc<cc_switch_lib::AppState>> {
    if let Some(dir) = &opts.config_dir {
        apply_config_dir_override(dir);
    }

    let db = cc_switch_lib::Database::init().map_err(ApiError::from)?;
    let state = cc_switch_lib::AppState::new(Arc::new(db));
    log::info!(
        "state initialised; app config dir = {:?}; config dir override = {:?}",
        app_config_dir(),
        opts.config_dir,
    );
    Ok(Arc::new(state))
}

fn apply_config_dir_override(dir: &PathBuf) {
    // `dirs::config_dir()` reads XDG_CONFIG_HOME on Linux, APPDATA on
    // Windows. Setting these *before* the first call from upstream
    // functions is enough to relocate Claude / Codex / Gemini live
    // configs.
    #[cfg(target_os = "linux")]
    std::env::set_var("XDG_CONFIG_HOME", dir);
    #[cfg(target_os = "macos")]
    std::env::set_var("XDG_CONFIG_HOME", dir);
    #[cfg(windows)]
    std::env::set_var("APPDATA", dir);
}

/// The upstream application config directory (`~/.cc-switch`).
/// Tests can set `CC_SWITCH_TEST_HOME` to relocate this path.
pub fn app_config_dir() -> PathBuf {
    home_dir().join(".cc-switch")
}

/// The upstream home directory (`~` by default).
/// Tests can set `CC_SWITCH_TEST_HOME` to relocate this path.
pub fn home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CC_SWITCH_TEST_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs::home_dir().unwrap_or_else(|| {
        log::warn!("无法获取用户主目录，回退到当前目录");
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    })
}
