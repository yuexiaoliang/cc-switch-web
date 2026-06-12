//! Application state initialisation.
//!
//! Wraps `cc_switch_lib::AppState` in an `Arc` and applies any environment
//! overrides the user passed on the command line (`--config-dir`).
//!
//! ## Design: Match upstream behavior exactly
//!
//! To ensure seamless migration between cc-switch-mini and the upstream
//! Tauri app, we use the EXACT same directory layout:
//!
//! - Database: `~/.cc-switch/cc-switch.db`
//! - Hermes config: `~/.hermes/config.yaml`
//! - Claude config: `~/.claude/`
//! - Codex config: `~/.codex/`
//! - Gemini config: `~/.gemini/`
//!
//! We do NOT set `CC_SWITCH_TEST_HOME` or relocate any config directories.
//! The `--data-dir` option is kept for backwards compatibility but only
//! affects where logs and temporary files are stored (if any).
//!
//! This ensures that users can switch between cc-switch-mini and the
//! upstream Tauri app without any data migration.

use crate::cli::Resolved;
use crate::error::{ApiError, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Build the upstream `AppState` and apply CLI overrides.
pub fn build(opts: &Resolved) -> Result<Arc<cc_switch_lib::AppState>> {
    // Match upstream behavior exactly: do NOT set CC_SWITCH_TEST_HOME.
    // This ensures all config files (Hermes, Claude, Codex, Gemini) and
    // the database are stored in their standard locations:
    // - Database: ~/.cc-switch/cc-switch.db
    // - Hermes: ~/.hermes/
    // - Claude: ~/.claude/
    // - Codex: ~/.codex/
    // - Gemini: ~/.gemini/
    //
    // The --config-dir option can still override host tool configs if needed.
    if let Some(dir) = &opts.config_dir {
        apply_config_dir_override(dir);
    }

    let db = cc_switch_lib::Database::init().map_err(ApiError::from)?;
    let state = cc_switch_lib::AppState::new(Arc::new(db));
    log::info!(
        "state initialised; using upstream directory layout; config dir override = {:?}",
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
