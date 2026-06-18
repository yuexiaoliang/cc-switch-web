# AGENTS.md

This file is for AI agents and human contributors working on this
project. After reading it you should be able to:
1. Find the code you need
2. Run the build and tests
3. Know which areas are protected (do not modify)
4. Know how to do the common tasks

## TL;DR

- The repository is a fork of [cc-switch](https://github.com/farion1231/cc-switch); all new code lives under `.ccsm/`.
- **Do not modify `src/` or `src-tauri/`** (except as part of the sync-upstream flow).
- Backend is an Axum service in `.ccsm/server/`; frontend is a TypeScript mock in `.ccsm/bridge/`; the two are wired together via pnpm `overrides`.
- To produce a release artifact: `pnpm run build:renderer && cargo build --release -p cc-switch-web-server`.
- Full design is in [`cc-switch-web.md`](./cc-switch-web.md); user docs in [`README.md`](./README.md); implementation details in [`.ccsm/DEVELOPMENT.md`](./.ccsm/DEVELOPMENT.md).

## Repository layout

```
cc-switch-web/
  src/                       # upstream frontend (Vite + React), do not modify
  src-tauri/                 # upstream Rust lib (cc_switch_lib), do not modify
  .ccsm/                     # this project's additions, all changes go here
    bridge/                  # TypeScript mocks that stand in for @tauri-apps/api
      api/                   #   core / event / window / app / path
      plugin-{dialog,process,store,updater}/
    server/                  # Axum HTTP service
      src/
        lib.rs               # router + serve loop
        main.rs              # CLI entry
        dispatch.rs          # 25 P0 commands + 10 stubs
        events.rs            # /api/events (SSE)
        auth.rs              # bearer-token middleware
        cli.rs               # clap parser
        state.rs             # bootstrap upstream AppState at startup
        error.rs             # ApiError + IntoResponse
        runtime.rs           # startup banner + serve
        static_files.rs      # embedded dist/ + SPA fallback
    scripts/                 # install.sh / sync-upstream.sh / check-coverage.sh
    DEVELOPMENT.md           # implementation details and known trade-offs
  Cargo.toml                 # workspace root (new; src-tauri/Cargo.toml untouched)
  package.json               # root package.json: pnpm.overrides are added here
  vite.config.ts             # left as-is
  .github/workflows/ci.yml   # 4 CI jobs
  pnpm-workspace.yaml        # left as-is
```

## Development workflow

### One-time setup

```bash
# 1. Install dependencies (pnpm-lock.yaml is committed)
pnpm install --frozen-lockfile

# 2. Verify the toolchain
pnpm typecheck
cargo check -p cc-switch-web-server
cargo test -p cc-switch-web-server
```

### Day-to-day loop

```bash
# Frontend: Vite HMR (run in a separate terminal; grabs port 3000)
pnpm run dev:renderer
# vite.config.ts hard-codes port: 3000. If the backend also wants 3000,
# change vite.config.ts's server.port or run the backend on a different port.

# Backend: cargo run, auto-restarts on .ccsm/server/src/** changes
cargo run -p cc-switch-web-server -- --port 3000
# Note: dev:renderer takes 3000, so run the backend on e.g. 3001 first.

# To use the Vite dev server for the frontend and bridge to the backend,
# add a proxy to vite.config.ts that forwards /api/* to the backend:
#   server: { proxy: { "/api": "http://localhost:3001" } }
# Then `cargo run -- --port 3001` + `pnpm run dev:renderer` (3000) both run.
# Or skip dev mode and just use a production build (below).
```

### Production build

```bash
pnpm run build:renderer    # writes dist/
cargo build --release -p cc-switch-web-server
./target/release/cc-switch-web --port 3000
# Open http://localhost:3000/
```

### Syncing upstream

```bash
# One-time: add the remote
git remote add upstream https://github.com/farion1231/cc-switch.git

# Each sync
./.ccsm/scripts/sync-upstream.sh
# Then run check-coverage.sh to see if the dispatch table has gaps
./.ccsm/scripts/check-coverage.sh
```

## Common tasks

### Add a new P0 command (spec 6.1)

1. In `.ccsm/server/src/dispatch.rs`, add an arm to the `dispatch()` match:
   ```rust
   "new_command" => provider::new_command(ctx, args).await,
   ```
2. In the matching `mod provider / settings / proxy / ...` inside `dispatch.rs`, add `pub async fn new_command(...)`. Prefer calling an upstream service (if the types are public); otherwise talk to the db directly via rusqlite.
3. Run `cargo check -p cc-switch-web-server` to confirm it compiles.
4. Run `bash .ccsm/scripts/check-coverage.sh` to confirm coverage.
5. Add a minimal unit test in `#[cfg(test)] mod tests` at the bottom of `dispatch.rs`.

### Add a new stub (spec 6.2)

Add an arm to the `dispatch()` match that returns a fixed value, e.g.:
```rust
"my_new_stub" => Ok(json!(true)),
```
Do not implement it — spec 6.2 explicitly requires no-op stubs.

### Modify the frontend bridge

Edit `.ccsm/bridge/<package>/<file>.ts` directly. The `exports` field in `package.json` controls the subpath mapping; if you change the file layout, update it too.

### After syncing upstream, a service signature in `src-tauri/` changed

1. Upstream signatures are usually backwards-compatible; only fix it when `cargo check` errors.
2. If `dispatch.rs` was using a type that was public before and is now private, fall back to a direct db-write shim in `.ccsm/server/src/` (see `proxy::update_config` or `stream_check::*` for the pattern).
3. Run `bash .ccsm/scripts/check-coverage.sh` to confirm every frontend `invoke` still has a handler.

## Conventions

- **Cargo workspace members**: `src-tauri` and `.ccsm/server`. `src-tauri` is not a separate workspace member — it is itself the package.
- **Binary name**: `cc-switch-web` (release); the package name is `cc-switch-web-server` (to avoid colliding with upstream's `cc-switch`).
- **Versioning**: tracks the upstream tag; the workspace `package.version` is the single source of truth. cc-switch-web-only changes use SemVer build metadata (`3.16.2+ccs.1`).
- **Error handling**: every handler returns `Result<Value, ApiError>`. `ApiError` in `.ccsm/server/src/error.rs` already implements `IntoResponse`, so `?` works.
- **Logging**: use the `log` crate (not `println!`); tune verbosity with `RUST_LOG=info,cc_switch_web=debug`.
- **Do not add dependencies to `src-tauri/Cargo.toml`** — that would pollute upstream. `.ccsm/server/Cargo.toml` is free to grow.

## Common pitfalls

1. **`rusqlite::Connection` is `!Send`**: it must be dropped before any `.await`. See the `stream_check::one` pattern — wrap all db operations in a `{ let conn = ...; ... }` block, let `conn` drop at the end of the block, then `.await` the network call. Otherwise the axum `Handler` trait is not satisfied.
2. **`ProviderSortUpdate` / `ProxyConfig` / `StreamCheckConfig` are private types**: write a db shim, do not try to re-export them.
3. **`AppType` is not `Copy`**: call `app.as_str().to_string()` up front to take ownership of the string, then pass `app` to the service (the service moves it).
4. **Frontend bridge uses pnpm `overrides`**: do not `pnpm add @tauri-apps/api`; edit `package.json`'s `overrides` field so the local bridge replaces it.
5. **`dist/` must exist**: `include_dir!("$CARGO_MANIFEST_DIR/../../dist")` requires the directory at compile time; CI uses `mkdir -p dist` as a placeholder. After cloning locally, run `pnpm run build:renderer` first.
6. **`[]` inside bash scripts**: `sync-upstream.sh` rewrites `package.json` with Python, where `pkg["dependencies"][k] = ...` has square brackets that bash misinterprets as array assignment. Use a quoted heredoc `<<'PY'`, not a bare heredoc.

## Deeper references

| Want to know... | Look at |
| --- | --- |
| Why this project exists | [`cc-switch-web.md`](./cc-switch-web.md) |
| How to install / use | [`README.md`](./README.md) |
| Known trade-offs / implementation details | [`.ccsm/DEVELOPMENT.md`](./.ccsm/DEVELOPMENT.md) |
| What CI runs / where it runs | [`.github/workflows/ci.yml`](./.github/workflows/ci.yml) |
| How sync-upstream resolves conflicts | Header comment in [`.ccsm/scripts/sync-upstream.sh`](./.ccsm/scripts/sync-upstream.sh) |
| What the dispatch table is missing | Run `bash .ccsm/scripts/check-coverage.sh` |

## Validation checklist (run after changing code)

Minimal validation (local):
```bash
pnpm typecheck
cargo check -p cc-switch-web-server
cargo test -p cc-switch-web-server
bash .ccsm/scripts/check-coverage.sh
```

> **`check-coverage.sh` exit code**: the script exits 1 when any frontend `invoke` lacks a dispatch handler. The Auth / Copilot / MCP / Skills / ... commands listed in spec 6.3 are **expected** to be MISSING — add them only when you actually implement those features. The PASS / FAIL judgment is the line "every frontend invoke is covered" in the script's output — spec 6.1's 25 P0 commands must be fully covered, 6.2's 10 stubs must be present, and 6.3's "out-of-scope" list showing up in MISSING is normal.

> **`pnpm test:unit` known issue**: upstream's `tests/integration/App.test.tsx` has 2 cases that fail when the full suite runs in parallel due to MSW state pollution (`"covers basic provider flows"` and `"shows toast when auto sync fails in background"`). Both pass when the file is run in isolation (`pnpm test:unit tests/integration/App.test.tsx` — 4/4 pass). It is an upstream test-suite bug, unrelated to the bridge. Workaround for now: `pnpm test:unit --exclude="**/integration/App.test.tsx"`.

Full CI mirror (before release):
```bash
# fmt + clippy
cargo fmt --check -p cc-switch-web-server
cargo clippy -p cc-switch-web-server --lib -- -D warnings
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings

# types + tests
pnpm test:unit
pnpm format:check
```

CI actually runs 4 jobs (frontend / upstream-tauri / server / bash-syntax), each about a minute. They can be run in parallel locally.




