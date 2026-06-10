#!/usr/bin/env bash
# Sync the upstream cc-switch repository into this fork.
#
# Usage:
#   ./.ccsm/scripts/sync-upstream.sh
#
# This implements section 7.1 of the spec: a full `git merge upstream/main`
# with deterministic conflict resolution. The intent is to keep the fork
# always equivalent to upstream plus the `.ccsm/` overlay.

set -euo pipefail

UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-upstream}"
UPSTREAM_BRANCH="${UPSTREAM_BRANCH:-main}"
WORKTREE="$(git rev-parse --show-toplevel)"

say() { printf '\033[1;34m[ccsm-sync]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[ccsm-sync]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[ccsm-sync]\033[0m %s\n' "$*" >&2; exit 1; }

# --- 1. Sanity checks ---------------------------------------------------------
[ -d "$WORKTREE/.git" ] || die "not a git repository: $WORKTREE"
[ -d "$WORKTREE/.ccsm" ] || die ".ccsm/ not found - this script is for the cc-switch-mini fork"

if ! git remote get-url "$UPSTREAM_REMOTE" >/dev/null 2>&1; then
  say "adding $UPSTREAM_REMOTE remote (https://github.com/farion1231/cc-switch.git)"
  git remote add "$UPSTREAM_REMOTE" "https://github.com/farion1231/cc-switch.git"
fi

# --- 2. Fetch + merge --------------------------------------------------------
say "fetching $UPSTREAM_REMOTE/$UPSTREAM_BRANCH"
git fetch "$UPSTREAM_REMOTE" "$UPSTREAM_BRANCH"

LOCAL="$(git rev-parse HEAD)"
REMOTE="$(git rev-parse "$UPSTREAM_REMOTE/$UPSTREAM_BRANCH")"
if [ "$LOCAL" = "$REMOTE" ]; then
  say "already up to date"
  exit 0
fi

say "merging $REMOTE"
if git merge --no-edit "$REMOTE"; then
  say "clean merge - nothing to do"
  exit 0
fi

# --- 3. Resolve conflicts using section 7.2 -----------------------------------
say "merge produced conflicts; applying cc-switch-mini resolution policy"

# These are upstream-owned and we want the latest version verbatim.
for path in src src-tauri; do
  if git diff --name-only --diff-filter=U | grep -q "^$path/"; then
    say "  taking upstream version of $path/"
    git checkout --theirs -- "$path/" || true
    git add -- "$path/" || true
  fi
done

# package.json: keep upstream's, restore our overrides block. We use
# `jq` (when present) so the JSON is parsed cleanly; otherwise we fall
# back to a Python one-liner.
if git diff --name-only --diff-filter=U | grep -q "^package.json$"; then
  say "  accepting upstream package.json (will re-apply our overrides)"
  git checkout --theirs -- package.json
  git add package.json
  if command -v jq >/dev/null 2>&1; then
    jq -s '
      .[0] as $upstream
      | $upstream
      | .pnpm.overrides = {
          "@tauri-apps/api":               "file:./.ccsm/bridge/api",
          "@tauri-apps/plugin-dialog":      "file:./.ccsm/bridge/plugin-dialog",
          "@tauri-apps/plugin-process":     "file:./.ccsm/bridge/plugin-process",
          "@tauri-apps/plugin-store":       "file:./.ccsm/bridge/plugin-store",
          "@tauri-apps/plugin-updater":     "file:./.ccsm/bridge/plugin-updater"
        }
      | .dependencies["@tauri-apps/api"]               = "file:./.ccsm/bridge/api"
      | .dependencies["@tauri-apps/plugin-dialog"]      = "file:./.ccsm/bridge/plugin-dialog"
      | .dependencies["@tauri-apps/plugin-process"]     = "file:./.ccsm/bridge/plugin-process"
      | .dependencies["@tauri-apps/plugin-store"]       = "file:./.ccsm/bridge/plugin-store"
      | .dependencies["@tauri-apps/plugin-updater"]     = "file:./.ccsm/bridge/plugin-updater"
      | .scripts["build:server"]        = "cargo build --release -p cc-switch-mini-server"
      | .scripts["ccsm:check-coverage"] = "bash .ccsm/scripts/check-coverage.sh"
      | .scripts["ccsm:sync-upstream"]   = "bash .ccsm/scripts/sync-upstream.sh"
    ' package.json > package.json.new && mv package.json.new package.json
  else
    # Python fallback - bracket-quote to avoid bash array assignment.
    python3 - <<'PY'
import json
import pathlib

p = pathlib.Path("package.json")
pkg = json.loads(p.read_text())
overrides = {
    "@tauri-apps/api": "file:./.ccsm/bridge/api",
    "@tauri-apps/plugin-dialog": "file:./.ccsm/bridge/plugin-dialog",
    "@tauri-apps/plugin-process": "file:./.ccsm/bridge/plugin-process",
    "@tauri-apps/plugin-store": "file:./.ccsm/bridge/plugin-store",
    "@tauri-apps/plugin-updater": "file:./.ccsm/bridge/plugin-updater",
}
pkg.setdefault("pnpm", {})["overrides"] = overrides
for k in list(overrides):
    pkg["dependencies"][k] = overrides[k]
pkg["scripts"].setdefault("build:server", "cargo build --release -p cc-switch-mini-server")
pkg["scripts"].setdefault("ccsm:check-coverage", "bash .ccsm/scripts/check-coverage.sh")
pkg["scripts"].setdefault("ccsm:sync-upstream", "bash .ccsm/scripts/sync-upstream.sh")
p.write_text(json.dumps(pkg, indent=2) + "\n")
PY
  fi
  git add package.json
fi

# .ccsm/ is ours - always keep our version.
if git diff --name-only --diff-filter=U | grep -q "^\.ccsm/"; then
  say "  keeping our .ccsm/ overlay"
  git checkout --ours -- .ccsm/
  git add -- .ccsm/
fi

# Cargo.toml at the root was added by us; if upstream added one, prefer ours.
if git diff --name-only --diff-filter=U | grep -q "^Cargo.toml$"; then
  if grep -q "cc-switch-mini-server" Cargo.toml 2>/dev/null; then
    say "  keeping our root Cargo.toml (workspace member)"
    git checkout --ours -- Cargo.toml
    git add Cargo.toml
  fi
fi

# Anything else with conflicts: warn and stop.
remaining="$(git diff --name-only --diff-filter=U || true)"
if [ -n "$remaining" ]; then
  warn "still has conflicts:"
  echo "$remaining" | sed "s/^/  /"
  die "please resolve manually and run \`git commit\` to finish the merge"
fi

git -c user.name="cc-switch-mini sync" \
    -c user.email="cc-switch-mini@users.noreply.github.com" \
    commit --no-edit
say "merge complete - run \`git push\` to publish"
