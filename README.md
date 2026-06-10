# cc-switch-mini

Headless Web derivative of [cc-switch](https://github.com/farion1231/cc-switch).
Runs on a server with no GUI and serves the upstream provider-management UI
in any modern browser over HTTP.

This README is the operational cheat sheet. The full design notes
(architecture decisions, dispatch-table rationale, upstream-merge
policy, and the local workarounds the upstream-private types forced)
live in [`.ccsm/DEVELOPMENT.md`](./.ccsm/DEVELOPMENT.md).

> **Desktop users: use the original [cc-switch](https://github.com/farion1231/cc-switch).**
> This headless derivative is a stop-gap for server deployments. Its lifecycle
> is expected to end once the upstream project ships an official server-side
> version.

## Highlights

- **Single binary** (`cc-switch-mini`) that serves the Web UI and persists
  state to a local SQLite database.
- **Zero changes to upstream** - the fork reuses the original Rust business
  logic (`ProviderService`, `ProxyService`, `ConfigService`, `Database`...)
  and the original React frontend verbatim.
- **Replaces Tauri's IPC** with a thin HTTP/SSE bridge that lives in
  `.ccsm/bridge/`. pnpm `overrides` redirect the `@tauri-apps/*` imports
  to the local bridge so the upstream frontend compiles unchanged.
- **Same source, two binaries**: the upstream Tauri build keeps working
  on a developer''s desktop; `cargo build -p cc-switch-mini-server`
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
| `--data-dir` | `~/.local/share/cc-switch-mini` | SQLite + settings location |
| `--config-dir` | user''s home | override where Claude / Codex / Gemini read their configs |
| `--token` | _(none)_ | optional bearer token; every `/api/*` request must carry it |
| `--no-spa-fallback` | off | 404 unknown paths (useful for debugging the bridge) |

Environment-variable equivalents: `CC_SWITCH_MINI_DATA_DIR`,
`CC_SWITCH_MINI_CONFIG_DIR`, `CC_SWITCH_MINI_TOKEN`.

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
|                   |        |              - ConfigService / settings
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
| Build a release | `pnpm run build` |
| Verify dispatch coverage | `pnpm run ccsm:check-coverage` |
| Sync upstream (maintainers) | `pnpm run ccsm:sync-upstream` |
| Run the test suite | `cargo test -p cc-switch-mini-server` |

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

