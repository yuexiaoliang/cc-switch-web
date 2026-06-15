# cc-switch-mini — development notes

This document captures the non-obvious decisions made while implementing
the cc-switch-mini overlay. It complements the user-facing `README.md`.

## Workspace layout

```
cc-switch-mini/
  Cargo.toml                 # workspace root: members = [src-tauri, .ccsm/server]
  src/                       # upstream frontend, untouched
  src-tauri/                 # upstream Rust library, untouched
  .ccsm/
    bridge/                  # TypeScript mocks for @tauri-apps/*
      api/                   #   - core, event, window, app, path
      plugin-dialog/
      plugin-process/
      plugin-store/
      plugin-updater/
    server/                  # Axum HTTP adapter
      Cargo.toml
      src/
        main.rs              # thin CLI entry
        lib.rs               # router + serve loop
        cli.rs               # clap parser
        state.rs             # upstream AppState bootstrap
        dispatch.rs          # POST /api/invoke/<cmd>
        events.rs            # GET  /api/events (SSE)
        static_files.rs      # GET  /<path> (SPA + embedded dist/)
        auth.rs              # bearer token middleware
        error.rs             # ApiError + IntoResponse
        runtime.rs           # logging + banner + serve()
    scripts/                 # install.sh, sync-upstream.sh, check-coverage.sh
    DEVELOPMENT.md           # this file
```

## Frontend bridge mechanism

`@tauri-apps/api/core::invoke` becomes `POST /api/invoke/<cmd>` with
`{"args": { ... }}` as the body. `@tauri-apps/api/event::listen` becomes a
single `EventSource` on `/api/events`; a shared listener set dispatches
incoming events to per-call handlers.

pnpm `overrides` (in the root `package.json`) redirect each
`@tauri-apps/*` import to a local `file:./.ccsm/bridge/<name>` directory.
That directory is itself a tiny npm package — its `package.json` carries
the original name and a `exports` field mapping each subpath to a
`./<file>.ts`. Vite resolves through this transparently.

## Server build

The single binary `cc-switch-mini` is produced by
`cargo build --release -p cc-switch-mini-server`. It depends on the
upstream `cc-switch` lib and embeds the Vite output via
`include_dir!("../../dist")`.

To produce a release artifact:

1. `pnpm install`
2. `pnpm run build:renderer`   # writes dist/
3. `cargo build --release -p cc-switch-mini-server`
4. `target/release/cc-switch-mini` is the deliverable.

## Dispatch table

Every Tauri command the frontend calls has a handler in
`.ccsm/server/src/dispatch.rs`. Each handler does one of:

1. Forward to a public upstream service (`ProviderService`,
   `ProxyService`).
2. Forward to a re-exported Tauri command function
   (`get_settings`, `save_settings`, `get_app_config_path`).
3. Hit a thin local shim — a direct DB write or a minimal
   implementation. Used when the upstream type is in a private
   module and cannot be named from outside.

`pnpm run ccsm:check-coverage` greps `src/lib/api/` for every
`invoke("<cmd>")` and compares the set against the dispatch table
matchers in `dispatch.rs`. Run it as part of the CI gate.

## Known workarounds

The spec asks for "no business logic" in the dispatch layer. The
upstream `cc_switch_lib` was not designed to be consumed from
outside its Tauri runtime, so a few public APIs are out of reach
even via the `pub use commands::*;` re-export:

- `services::provider::ProviderSortUpdate` lives behind a private
  module. `update_providers_sort_order` is implemented as a direct
  `UPDATE providers SET sort_index = ?1` instead of calling the
  service method.
- `proxy::types::ProxyConfig` is private. `update_proxy_config`
  patches the `proxy_config` table directly via SQL.
- `services::stream_check::{StreamCheckConfig, StreamCheckResult,
  HealthStatus}` are private. `stream_check_*` commands use a
  self-contained mirror struct and a minimal HTTP probe; the
  upstream retry / Copilot-auth / proxy-only-filtering logic is
  not replicated.
- The `Database` DAO methods (e.g. `get_stream_check_config`) are
  declared in `pub(crate) mod dao`. The cc-switch-mini server opens
  its own `rusqlite::Connection` to the same file and queries the
  `settings` table directly. This works because the
  `Database::init()` call at startup already created the schema.

These workarounds are documented at each dispatch site with a short
comment so future maintainers can swap them out once the upstream
re-exports the relevant types.

## Concurrency note

`rusqlite::Connection` is `!Send` by default. The dispatch handlers
that need a connection open it inside a block, do all the queries
synchronously, and drop the connection **before** any `.await`. This
is what keeps the `Handler` future `Send`-bound for axum 0.7. The
`stream_check::one` and `stream_check::all` handlers are the most
prominent example — see their implementation for the pattern.

## CLI flags

| Flag | Env var | Default | Notes |
| --- | --- | --- | --- |
| `--host` | – | `127.0.0.1` | `0.0.0.0` to expose the UI; pair with `--token` |
| `--port` | – | `3000` | listen port |
| `--config-dir` | `CC_SWITCH_MINI_CONFIG_DIR` | – | override where Claude / Codex / Gemini read their live configs |
| `--token` | `CC_SWITCH_MINI_TOKEN` | – | bearer token; every `/api/*` request must carry `Authorization: Bearer <token>` |
| `--no-spa-fallback` | – | off | disable `index.html` fallback for unknown paths |

cc-switch-mini uses the upstream Tauri app directory layout verbatim. The upstream re-exports only a few `config` helpers, so the cc-switch-mini path computations are duplicated for the apps where the helper is unreachable (e.g. `get_claude_config_dir`). Tests can relocate the home directory via the `CC_SWITCH_TEST_HOME` env var.

## Auth model

`--token` enables a middleware (`auth::enforce`) on every `/api/*`
route. SSE also checks the token (browser EventSource cannot
attach custom headers, so the bridge falls back to a `?token=`
query string on the SSE URL when the header is missing). When the
server binds to a loopback address, the spec considers
authentication optional — `--token` defaults to off and the
middleware is a no-op.

## Versioning

The workspace version (root `Cargo.toml` `workspace.package.version`,
plus `package.json` `version`) mirrors the upstream tag. When a
cc-switch-mini-only change ships between upstream tags, we use
build-metadata in the SemVer (e.g. `3.16.2+ccs.1`).

## Tests

- `cargo test -p cc-switch-mini-server` exercises the auth helper,
  CLI parser, and a couple of small unit checks in `dispatch.rs`.
- The full integration suite (the upstream `src-tauri/tests/`) is
  untouched and remains the source of truth for the business
  logic.

