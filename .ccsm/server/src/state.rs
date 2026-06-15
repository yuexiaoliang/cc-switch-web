//! Application state initialisation.
//!
//! Wraps `cc_switch_lib::AppState` in an `Arc` and applies any environment
//! overrides the user passed on the command line (`--data-dir`, `--config-dir`).
//!
//! ## Directory layout
//!
//! By default (no `--data-dir`) we match the upstream Tauri app exactly so
//! that users can switch between cc-switch-mini and the desktop app without
//! data migration:
//!
//! - Database: `~/.cc-switch/cc-switch.db`
//! - Hermes config: `~/.hermes/config.yaml`
//! - Claude config: `~/.claude/`
//! - Codex config: `~/.codex/`
//! - Gemini config: `~/.gemini/`
//!
//! When `--data-dir` is explicitly provided, that directory becomes the
//! effective home directory for upstream path resolution (via
//! `CC_SWITCH_TEST_HOME`). This makes the server self-contained and is the
//! behaviour the e2e smoke tests rely on.

use crate::cli::Resolved;
use crate::error::{ApiError, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Build the upstream `AppState` and apply CLI overrides.
pub fn build(opts: &Resolved) -> Result<Arc<cc_switch_lib::AppState>> {
    // When the operator explicitly gives us a data directory, make it the
    // effective home directory for upstream's path resolution. This causes
    // the SQLite database (`~/.cc-switch/cc-switch.db`) and all host-tool
    // config directories (Claude, Codex, Gemini, Hermes, ...) to live under
    // that single directory, which is what users expect from `--data-dir`.
    //
    // When no `--data-dir` is provided we keep the default upstream layout
    // so that cc-switch-mini and the upstream Tauri app share the same
    // files and migration is seamless.
    if opts.explicit_data_dir {
        let home = opts.data_dir.display().to_string();
        std::env::set_var("CC_SWITCH_TEST_HOME", &home);
        log::info!("using explicit data dir as effective home: {home}");
    }

    if let Some(dir) = &opts.config_dir {
        apply_config_dir_override(dir);
    }

    let db = cc_switch_lib::Database::init().map_err(ApiError::from)?;
    let state = cc_switch_lib::AppState::new(Arc::new(db));
    log::info!(
        "state initialised; data_dir = {:?}; config dir override = {:?}",
        opts.data_dir,
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
