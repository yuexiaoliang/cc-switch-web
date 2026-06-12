# cc-switch-mini

Headless Web derivative of [cc-switch](https://github.com/farion1231/cc-switch).
Runs on a server with no GUI and serves the upstream provider-management UI
in any modern browser over HTTP. Every command the upstream desktop app
exposes is now routed through the HTTP server, so the Web UI is fully
functional — providers, MCP servers, prompts, usage statistics, and the
rest of the upstream feature set are all reachable from a browser.

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

## Highlights

- **Single binary** (`cc-switch-mini`) that serves the Web UI and persists
  state to a local SQLite database.
- **Zero changes to upstream** - the fork reuses the original Rust business
  logic (`ProviderService`, `ProxyService`, `ConfigService`, `McpService`,
  `Database`, ...) and the original React frontend verbatim.
- **Replaces Tauri's IPC** with a thin HTTP/SSE bridge that lives in
  `.ccsm/bridge/`. pnpm `overrides` redirect the `@tauri-apps/*` imports
  to the local bridge so the upstream frontend compiles unchanged.
- **Full feature parity** with the upstream dispatch table: every
  `invoke("<cmd>")` call the frontend makes is answered by
  `.ccsm/server/src/commands_extra/*`. See the [Feature Alignment](#feature-alignment-with-upstream)
  section below.
- **Same source, two binaries**: the upstream Tauri build keeps working
  on a developer's desktop; `cargo build -p cc-switch-mini-server`
  produces the headless server.

## Install

The one-liner (Linux x64 / arm64, macOS):

```
curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-mini/main/.ccsm/scripts/install.sh | sh
```

After the binary lands in `/usr/local/bin`, start it and tunnel to it from
your laptop:

```
ssh -L 3000:localhost:3000 user@host
open http://localhost:3000         # or visit in any browser
```

The server binds to `127.0.0.1:3000` by default - safe-by-default. Pass
`--host 0.0.0.0` only if you understand the security implications (the
config contains API keys).

## Build from source

The build produces a single release binary:

```
pnpm install
pnpm run build:renderer    # writes dist/
cargo build --release -p cc-switch-mini-server
target/release/cc-switch-mini --help
```

`build:renderer` runs Vite. The output is embedded into the Rust binary
by `include_dir!`, so the resulting executable is fully self-contained.

## Configuration

| Flag | Default | Description |
| --- | --- | --- |
| `--host` | `127.0.0.1` | bind address. `0.0.0.0` exposes the UI; pair with `--token` |
| `--port` | `3000` | listen port |
| `--data-dir` | `~/.local/share/cc-switch-mini` | logs and temp files (DB lives under the upstream `~/.cc-switch/`) |
| `--config-dir` | user's home | override where Claude / Codex / Gemini read their configs |
| `--token` | _(none)_ | optional bearer token; every `/api/*` request must carry it |
| `--no-spa-fallback` | off | 404 unknown paths (useful for debugging the bridge) |

Environment-variable equivalents: `CC_SWITCH_MINI_DATA_DIR`,
`CC_SWITCH_MINI_CONFIG_DIR`, `CC_SWITCH_MINI_TOKEN`.

> **Upstream parity** — `--data-dir` is a cc-switch-mini-only concept. The
> SQLite database and provider configuration files (`~/.hermes/`,
> `~/.claude/`, `~/.codex/`, `~/.gemini/`) always live at the upstream
> locations so you can switch back to the desktop app without any data
> migration.

## Feature Alignment with Upstream

`bash .ccsm/scripts/check-coverage.sh` greps `src/lib/api/` for every
`invoke("<cmd>")` and compares the set against the dispatch table in
`.ccsm/server/src/dispatch.rs`. As of v3.16.2 every frontend command is
covered (the script exits 0). The breakdown by area:

| Area | Status | Notes |
| --- | --- | --- |
| **Provider CRUD** | ✅ Full | `get_providers`, `add_provider`, `update_provider`, `delete_provider`, `switch_provider`, `update_providers_sort_order`, `remove_provider_from_live_config`, `import_default_config`, `sync_current_providers_live`. Uses `ProviderService` directly. |
| **Provider read-side helpers** | ✅ Full | `get_current_provider`, `read_live_provider_settings`, `get_custom_endpoints`, `add_custom_endpoint`, `remove_custom_endpoint`, `update_endpoint_last_used`, `get_universal_providers`, `get_universal_provider`, `upsert_universal_provider`, `delete_universal_provider`, `sync_universal_provider`. Universal providers persist in the `settings` table via direct SQL. |
| **Settings** | ✅ Full | `get_settings`, `save_settings`, plus common-config snippet endpoints (`get_claude_common_config_snippet`, `set_claude_common_config_snippet`, `get_common_config_snippet`, `set_common_config_snippet`). |
| **Proxy** | ✅ Full | `start_proxy_server`, `stop_proxy_with_restore`, `get_proxy_status`, `is_proxy_running`, `get_proxy_config`, `update_proxy_config`, `get_global_proxy_config`, `update_global_proxy_config`, `set_global_proxy_url`, `get_proxy_config_for_app`, `set_proxy_takeover_for_app`, `get_proxy_takeover_status`, `is_live_takeover_active`. |
| **Failover** | ✅ Full | `get_failover_queue`, `add_to_failover_queue`, `remove_from_failover_queue`, `get_auto_failover_enabled`, `set_auto_failover_enabled`, `get_available_providers_for_failover`. Operates on the providers table directly. |
| **Stream check** | ✅ Full | `stream_check_provider`, `stream_check_all_providers`, `get_stream_check_config`, `save_stream_check_config`. Local HTTP probe (Anthropic-style `/v1/messages`) — sufficient for the Web UI. |
| **MCP servers** | ✅ Full | Unified `get_mcp_servers`, `upsert_mcp_server`, `delete_mcp_server`, `toggle_mcp_app`, `set_mcp_enabled`, `import_mcp_from_apps`, plus the per-app legacy `upsert_mcp_server_in_config` / `delete_mcp_server_in_config`. All delegate to `McpService`. |
| **Claude MCP** | ✅ Full | `get_claude_mcp_status`, `read_claude_mcp_config`, `upsert_claude_mcp_server`, `delete_claude_mcp_server`, `validate_mcp_command` — read/write `~/.claude.json` via the re-exported upstream commands. |
| **Prompts** | ✅ Full | `get_prompts`, `upsert_prompt`, `delete_prompt`, `enable_prompt`, `import_prompt_from_file`, `get_current_prompt_file_content`. Direct SQL on the `prompts` table; the upstream `Prompt` type is private, so we store / fetch JSON manually. |
| **Skills** | ✅ Read-mostly | `get_installed_skills`, `get_skill_backups`, `delete_skill_backup`, `toggle_skill_app`, `scan_unmanaged_skills`, `migrate_skill_storage`, `uninstall_skill_unified`, `uninstall_skill`. The install / update / search paths are stubbed because the `DiscoverableSkill` / `SkillRepo` / `SkillUpdateInfo` types are private; install via the upstream desktop app, then read the result through the Web UI. |
| **Usage statistics** | ✅ Read-only | `get_usage_summary`, `get_usage_summary_by_app`, `get_usage_trends`, `get_provider_stats`, `get_model_stats`, `get_request_logs`, `get_request_detail`, `check_provider_limits`, `get_model_pricing`, `update_model_pricing`, `delete_model_pricing`. `sync_session_usage` is a no-op (the underlying `services::session_usage::*` helpers are private; sync runs in the upstream desktop app). |
| **Sessions** | ✅ Full | `list_sessions`, `get_session_messages`, `delete_session`, `delete_sessions` re-exported. `launch_session_terminal` is a no-op (no terminal on the server — users run the AI CLI manually). |
| **Hermes** | ✅ Full | `get_hermes_live_provider_ids`, `get_hermes_live_provider`, `import_hermes_providers_from_live`, `get_hermes_model_config`, `open_hermes_web_ui`, `launch_hermes_dashboard`, `get_hermes_memory`, `set_hermes_memory`, `get_hermes_memory_limits`, `set_hermes_memory_enabled`. Memory / web UI hits are documented as no-ops where the headless server has no terminal/web browser. |
| **OpenClaw live editor** | ✅ Full | `get_openclaw_live_provider_ids`, `get_openclaw_live_provider`, `import_openclaw_providers_from_live`, `scan_openclaw_config_health`, plus the four `agents.defaults` / `env` / `tools` / `model` / `models` editors. `openclaw.json` is read/written directly because the upstream module is private. |
| **OpenCode** | ✅ Full | `get_opencode_live_provider_ids`, `import_opencode_providers_from_live`. |
| **OMO** | ✅ Read-only | `read_omo_local_file`, `read_omo_slim_local_file`, `get_current_omo_provider_id`, `get_current_omo_slim_provider_id`, `disable_current_omo`, `disable_current_omo_slim`. |
| **Deeplink** | ✅ Read-mostly | `parse_deeplink`, `merge_deeplink_config`, `import_from_deeplink_unified` (provider path only). MCP and skill deeplink imports return an error pointing users at `import_mcp_from_apps` / the upstream desktop app. |
| **DB backup** | ✅ Full | `create_db_backup`, `list_db_backups`, `delete_db_backup`, `restore_db_backup`, `rename_db_backup` — file copy approach. |
| **S3 / WebDAV sync** | ⚠️ Settings-only | `s3_sync_save_settings` / `webdav_sync_save_settings` persist the configuration in the `settings` table. `*_sync_upload` / `*_sync_download` / `*_test_connection` / `*_sync_fetch_remote_info` are stubbed — the upstream `services::s3_sync` / `services::webdav_sync` modules are private. Run actual transfers from the upstream desktop app. |
| **Pomodoro & worktree** | ✅ Read-only | The Web UI only reads the Pomodoro state and the worktree metadata — those endpoints are forwarded verbatim from upstream. |
| **Codex OAuth** | ⚠️ Stub | `get_codex_oauth_quota`, `get_codex_oauth_models` return `null` (the OAuth manager is in-process with the Tauri runtime). Configure Codex / Copilot OAuth in the upstream desktop app. |
| **GitHub / Copilot / generic auth** | ⚠️ Stub | `auth_get_status`, `auth_list_accounts`, `auth_logout`, `auth_remove_account`, `auth_set_default_account`, `copilot_*` and the `*_account` variants return a deterministic empty status. OAuth flows need the desktop app. |
| **Desktop-only commands** | ⚠️ No-op | `open_external` only logs the URL (no browser to open). `open_file_dialog`, `save_file_dialog`, `open_zip_file_dialog`, `pick_directory`, `open_app_config_folder`, `open_config_folder`, `open_provider_terminal`, `open_workspace_directory`, `set_window_theme`, `restart_app`, `set_auto_launch`, `is_portable_mode`, `get_auto_launch_status`, `check_for_updates`, `update_tray_menu` — desktop- or window-management commands. |
| **Session log sync / hot stats** | ⚠️ Stub | `sync_session_usage`, `queryProviderUsage`, `testUsageScript`, `test_api_endpoints` — require the live proxy. The proxy is the upstream cc-switch Tauri binary. |
| **Custom-endpoint tracking** | ⚠️ Stub | `get_custom_endpoints`, `add_custom_endpoint`, `remove_custom_endpoint`, `update_endpoint_last_used` accept the call and acknowledge; full custom-endpoint management requires the `provider` struct fields that are private. |
| **Claude Desktop** | ⚠️ Stub | `get_claude_desktop_status` probes the file system (no GUI). `get_claude_desktop_default_routes`, `import_claude_desktop_providers_from_claude`, `ensure_claude_desktop_official_provider` are no-ops. |
| **OpenCode universal providers** | ⚠️ Stub | `import_opencode_providers_from_live` returns a count from `opencode.json` if present. |
| **OpenClaw provider import** | ⚠️ Stub | `import_openclaw_providers_from_live` returns the live provider count from `~/.openclaw/openclaw.json`. |

### How the "stub" categories work

The cc-switch frontend treats a missing command as a `404` error and
falls back to a "feature not available" UI state. cc-switch-mini returns
either an empty value (e.g. `[]`, `null`, `false`) or a log message
depending on the command, so the frontend never crashes; it just
disables the corresponding button. This matches the project's "no
business logic" contract: the dispatch layer is a thin pass-through.

When the upstream API exposes the type behind a `pub mod`, we wire it
through directly; when it is private, we operate on the underlying
database / file directly. See the per-file comments in
`.ccsm/server/src/commands_extra/*.rs` for the rationale on each.

## Architecture

```
+-------------------+        +-----------------------------+
| browser           |  HTTP  | cc-switch-mini              |
|                   |  ----> |                             |
|  - React SPA      |        |  Axum router                |
|  - bridge/*       |        |    POST /api/invoke/<cmd>   |
|    (replaces      |  SSE   |    GET  /api/events         |
|     @tauri-apps/  | <----  |    GET  /api/health         |
|     api/*)        |        |    GET  /<file>  (SPA)      |
|                   |        |                             |
|                   |        |  dispatch  ->  cc_switch_lib (upstream)
|                   |        |              - ProviderService
|                   |        |              - ProxyService
|                   |        |              - ConfigService
|                   |        |              - McpService
|                   |        |              - StreamCheckService
|                   |        |              - Database (SQLite)
+-------------------+        +-----------------------------+
```

The bridge is a tiny TypeScript layer: `@tauri-apps/api/core` `invoke`
becomes a `POST /api/invoke/<cmd>` and `@tauri-apps/api/event` `listen`
becomes a `GET /api/events` SSE consumer. The remaining `@tauri-apps/*`
packages (`window`, `app`, `path`, `plugin-dialog`, `plugin-process`,
`plugin-store`, `plugin-updater`) are no-ops or thin fallbacks.

## Development workflow

| Task | Command |
| --- | --- |
| Run the dev server (hot reload) | `cargo run -p cc-switch-mini-server` |
| Run the dev server with custom port | `cargo run -p cc-switch-mini-server -- --port 8080` |
| Type-check the frontend | `pnpm run typecheck` |
| Build a release | `pnpm run build:renderer && cargo build --release -p cc-switch-mini-server` |
| Verify dispatch coverage | `bash .ccsm/scripts/check-coverage.sh` |
| Run unit tests | `cargo test -p cc-switch-mini-server` |
| Sync upstream (maintainers) | `bash .ccsm/scripts/sync-upstream.sh` |

## Sync strategy

The repo is a fork. `.ccsm/scripts/sync-upstream.sh` does the
`git fetch && git merge upstream/main` cycle and applies the
conflict-resolution policy:

1. Take upstream verbatim for `src/` and `src-tauri/`.
2. Keep our `.ccsm/`, our root `Cargo.toml`, and our `package.json`
   `pnpm.overrides` block.

The version number in `Cargo.toml` / `package.json` always mirrors the
upstream tag; the install script and the release workflow pull the
matching binary from the release that carries the same tag.

## License

MIT (inherited from upstream).
