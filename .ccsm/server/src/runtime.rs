//! Top-level runtime glue. `main.rs` is a thin wrapper around `run()`.

use crate::cli::Resolved;
use crate::error::Result;
use std::sync::Arc;

/// Build the `AppContext` and serve until interrupted.
pub async fn run(opts: Resolved) -> Result<()> {
    init_logging();

    let opts = Arc::new(opts);
    let state = crate::state::build(&opts)?;
    let events = crate::events::EventBus::new();
    let ctx = Arc::new(crate::AppContext::new(state, events, opts.clone()));

    // Friendly banner. Mirrors the spec''s example output (section 3.2).
    log::info!("");
    log::info!("  CCSwitch Mini running at http://{}/", opts.bind_addr);
    if opts.token.is_some() {
        log::info!("  Authentication: bearer token required (Authorization: Bearer ...)");
    } else if opts.bind_addr.ip().is_loopback() {
        log::info!("  Loopback only - open the URL on this machine");
    } else {
        log::info!("  Bound to a non-loopback address - make sure this is intentional");
    }
    log::info!("");

    crate::serve(ctx, opts.bind_addr).await
}

fn init_logging() {
    use env_logger::{Builder, Env};
    let env = Env::default().default_filter_or("info,cc_switch_web=debug");
    Builder::from_env(env)
        .format_timestamp_secs()
        .format_module_path(false)
        .try_init()
        .ok();
}
