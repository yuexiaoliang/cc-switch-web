//! Application state initialisation.
//!
//! Wraps `cc_switch_lib::AppState` in an `Arc` and applies any environment
//! overrides the user passed on the command line (`--data-dir`,
//! `--config-dir`).
//!
//! ## How `--data-dir` is wired in
//!
//! The upstream `Database::init()` reads its location from
//! `config::get_app_config_dir()`, which itself falls through
//! `config::get_home_dir()`. `get_home_dir` honours an `CC_SWITCH_TEST_HOME`
//! env var (originally a test escape hatch). We set that var to
//! `<data-dir>` at startup so the upstream layout (`<home>/.cc-switch/...`)
//! is preserved but relocated under the user's data dir.
//!
//! In other words: `--data-dir /var/lib/cc-switch-mini` puts the SQLite
//! database at `/var/lib/cc-switch-mini/.cc-switch/cc-switch.db`, matching
//! the directory layout the upstream Tauri build produces on a desktop
//! install. The trailing `.cc-switch` is intentional - it keeps a stable
//! shape we can reason about across the spec, the install script, and
//! future migrations.

use crate::cli::Resolved;
use crate::error::{ApiError, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Build the upstream `AppState` and apply CLI overrides.
pub fn build(opts: &Resolved) -> Result<Arc<cc_switch_lib::AppState>> {
    // 1. `--data-dir` -> override the home dir used by `get_app_config_dir`.
    apply_data_dir_override(&opts.data_dir);

    // 2. `--config-dir` -> override the host-tools' config dir. We cannot
    //    reach into the upstream `app_store` module from here, so we
    //    re-implement the relevant subset: set XDG_CONFIG_HOME / APPDATA
    //    so `dirs::config_dir()` returns our override. The change is
    //    best-effort; if the upstream does not read those env vars
    //    (e.g. they pin to `dirs::config_dir()` at compile time), the
    //    override is a no-op.
    if let Some(dir) = &opts.config_dir {
        apply_config_dir_override(dir);
    }

    let db = cc_switch_lib::Database::init().map_err(ApiError::from)?;
    let state = cc_switch_lib::AppState::new(Arc::new(db));
    log::info!(
        "state initialised; data dir = {}, config dir override = {:?}",
        opts.data_dir.display(),
        opts.config_dir,
    );
    Ok(Arc::new(state))
}

fn apply_data_dir_override(data_dir: &PathBuf) {
    // The upstream `config::get_home_dir` consults `CC_SWITCH_TEST_HOME`
    // first. We piggy-back on that contract: the database file will end up
    // at `<data_dir>/.cc-switch/cc-switch.db`.
    std::env::set_var("CC_SWITCH_TEST_HOME", data_dir);
    // Also pre-create the directory so the user can spot the layout
    // before the first write.
    let _ = std::fs::create_dir_all(data_dir);
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
