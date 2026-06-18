//! cc-switch-web binary entry point.
//!
//! All the heavy lifting lives in `lib.rs`; this file is intentionally
//! tiny so the same logic can be embedded in integration tests.

use cc_switch_web_server::{cli, error, runtime};
use clap::Parser;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let raw = cli::Cli::parse();
    let opts = match cli::Resolved::resolve(raw) {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("cc-switch-web: failed to resolve options: {err}");
            return std::process::ExitCode::from(2);
        }
    };

    match runtime::run(opts).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cc-switch-web: {err}");
            std::process::ExitCode::from(error::exit_code(&err))
        }
    }
}
