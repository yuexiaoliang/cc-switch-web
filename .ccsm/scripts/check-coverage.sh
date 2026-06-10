#!/usr/bin/env bash
# Detect frontend `invoke` calls that the dispatch layer does not handle.
#
# This is the dev-time guard the spec calls out in section 7.3. Run it
# from the repository root:
#
#   pnpm run ccsm:check-coverage
#
# It greps `src/lib/api/` for every Tauri command invoked from the
# frontend, then compares the set against the dispatch table compiled
# into `.ccsm/server/src/dispatch.rs`. The script exits 1 if the
# frontend calls a command that the server does not recognise.

set -euo pipefail

WORKTREE="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
INVOKED_TSV="$(mktemp)"
DISPATCH_TSV="$(mktemp)"
trap "rm -f '$INVOKED_TSV' '$DISPATCH_TSV'" EXIT

# 1. Collect every `invoke("<cmd>"` argument under src/lib/api/.
grep -rhoE 'invoke\("[a-zA-Z0-9_]+"' "$WORKTREE/src/lib/api" \
  | sed -E 's/^invoke\("([^"]+)"$/\1/' \
  | sort -u > "$INVOKED_TSV"

# 2. Collect every "<cmd>" => / arm in the dispatch match block.
sed -n '/^async fn dispatch/,/^}/p' "$WORKTREE/.ccsm/server/src/dispatch.rs" \
  | grep -oE '"[a-zA-Z0-9_]+"\s*=>' \
  | sed -E 's/^"([^"]+)"\s*=>$/\1/' \
  | sort -u > "$DISPATCH_TSV"

echo "[ccsm-coverage] frontend invokes $(wc -l < "$INVOKED_TSV") commands:"
sed "s/^/  /" "$INVOKED_TSV"

echo "[ccsm-coverage] dispatch handles $(wc -l < "$DISPATCH_TSV") commands:"
sed "s/^/  /" "$DISPATCH_TSV"

echo
echo "[ccsm-coverage] coverage report:"

# Invoked but not handled - regression.
missed="$(comm -23 "$INVOKED_TSV" "$DISPATCH_TSV")"
if [ -n "$missed" ]; then
  echo "  MISSING in dispatch.rs (frontend calls them but server returns 404):"
  echo "$missed" | sed "s/^/    /"
  status=1
else
  echo "  every frontend invoke is covered"
  status=0
fi

# Handled but never invoked - dead code (warning only).
unused="$(comm -13 "$INVOKED_TSV" "$DISPATCH_TSV")"
if [ -n "$unused" ]; then
  echo
  echo "  UNUSED in dispatch.rs (no frontend caller; review before removing):"
  echo "$unused" | sed "s/^/    /"
fi

exit "$status"
