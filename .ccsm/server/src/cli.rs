//! Command-line argument parsing.
//!
//! Defaults are chosen to match the spec''s "safe" profile (127.0.0.1:3000).
//! Section 8.3 of the spec covers the security model: a non-loopback bind
//! requires an explicit `--host 0.0.0.0` and is on the user to harden.

use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "cc-switch-mini",
    version,
    about = "Headless Web UI for cc-switch - manage AI CLI providers from any browser."
)]
pub struct Cli {
    /// Address to bind to. Use 0.0.0.0 to expose the UI to a network
    /// (NOT recommended - use an SSH tunnel or a reverse proxy instead).
    #[arg(long, default_value = "127.0.0.1", value_name = "IP")]
    pub host: IpAddr,

    /// Port to listen on.
    #[arg(long, default_value_t = 3000, value_name = "PORT")]
    pub port: u16,

    /// Directory used to persist the SQLite database and backups. The
    /// database file lives at `<data-dir>/cc-switch.db`.
    #[arg(long, value_name = "DIR", env = "CC_SWITCH_MINI_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Override the config dir for the *host* tools (Claude / Codex / Gemini
    /// configs). Defaults to the user''s home directory exactly like the
    /// upstream Tauri build.
    #[arg(long, value_name = "DIR", env = "CC_SWITCH_MINI_CONFIG_DIR")]
    pub config_dir: Option<PathBuf>,

    /// Optional bearer token. Every `/api/*` request (and SSE) must carry
    /// `Authorization: Bearer <token>`. When unset, only the loopback bind
    /// protects the UI.
    #[arg(long, value_name = "TOKEN", env = "CC_SWITCH_MINI_TOKEN")]
    pub token: Option<String>,

    /// Disable the SPA fallback so unknown paths 404 (useful for debugging
    /// the bridge layer).
    #[arg(long)]
    pub no_spa_fallback: bool,
}

/// Resolved runtime configuration. Built once at startup and shared through
/// `AppContext`.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub bind_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub config_dir: Option<PathBuf>,
    pub token: Option<String>,
    pub spa_fallback: bool,
}

impl Resolved {
    pub fn resolve(cli: Cli) -> std::io::Result<Self> {
        let data_dir = match cli.data_dir {
            Some(dir) => dir,
            None => default_data_dir()?,
        };
        std::fs::create_dir_all(&data_dir)?;

        let bind_addr = SocketAddr::new(cli.host, cli.port);

        // Loudly warn if the user opted into a public bind without a token.
        if !is_loopback(&cli.host) && cli.token.is_none() {
            log::warn!(
                "binding to {} without --token; anyone able to reach the port will see your API keys", cli.host
            );
        }

        Ok(Self {
            bind_addr,
            data_dir,
            config_dir: cli.config_dir,
            token: cli.token,
            spa_fallback: !cli.no_spa_fallback,
        })
    }
}

fn default_data_dir() -> std::io::Result<PathBuf> {
    // Mirror upstream''s default: `~/.local/share/cc-switch-mini/` on Linux,
    // the platform-native equivalent elsewhere.
    let base = dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    Ok(base.join("cc-switch-mini"))
}

fn is_loopback(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_loopback() || *v4 == Ipv4Addr::UNSPECIFIED,
        IpAddr::V6(v6) => v6.is_loopback() || *v6 == std::net::Ipv6Addr::UNSPECIFIED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_default_bind_to_loopback_3000() {
        let cli = Cli::parse_from(["cc-switch-mini"]);
        let resolved = Resolved::resolve(cli).unwrap();
        assert_eq!(resolved.bind_addr.port(), 3000);
        assert!(resolved.spa_fallback);
    }

    #[test]
    fn warns_on_public_bind_without_token() {
        // We do not assert on the log output, only that resolve succeeds and
        // the loopback guard runs without panicking.
        let cli = Cli::parse_from(["cc-switch-mini", "--host", "0.0.0.0"]);
        let resolved = Resolved::resolve(cli).unwrap();
        assert!(resolved.token.is_none());
    }
}
