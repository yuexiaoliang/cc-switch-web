#!/usr/bin/env bash
# cc-switch-web one-line installer.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-web/main/.ccsm/scripts/install.sh | sh
#
# Environment variables (all optional):
#   CCSM_VERSION         - release tag to install (default: latest)
#   CCSM_INSTALL_DIR     - target binary directory (default: /usr/local/bin)
#   CCSM_GITHUB_REPO     - override the source repository (default: yuexiaoliang/cc-switch-web)
#   CCSM_NO_SERVICE      - set to 1 to skip the systemd unit / launchd plist registration
#
# The script:
#   1. Detects the host platform (linux x64 / arm64).
#   2. Downloads the matching release tarball.
#   3. Verifies the SHA-256 checksum against the manifest.
#   4. Installs the binary to $CCSM_INSTALL_DIR (sudo if not writable).
#   5. Prints a one-line `ssh -L` hint to remind the user about the
#      safe SSH-tunnel access pattern.

# The shebang says bash (it gives us arrays, `[[ ]]`, etc. for free), but
# the script body is intentionally POSIX-sh compatible so it also runs
# under `dash` (the default `/bin/sh` on Debian/Ubuntu). In particular:
#   * `set -eu` instead of `set -euo pipefail` - downstream checks (empty
#     $VERSION, missing $expected, mismatched checksum) cover pipe failures
#     without needing pipefail, which dash does not implement.
#   * `local` only appears inside functions, never inside `(...)` subshells.
# This is the fix for the "sh: 21: set: Illegal option -o pipefail" error
# users hit when running `curl ... | sh` on a fresh Ubuntu box.
set -eu

GITHUB_REPO="${CCSM_GITHUB_REPO:-yuexiaoliang/cc-switch-web}"
INSTALL_DIR="${CCSM_INSTALL_DIR:-/usr/local/bin}"
BIN_NAME="cc-switch-web"
SKIP_SERVICE="${CCSM_NO_SERVICE:-0}"

say() { printf '\033[1;34m[ccsm]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[ccsm]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[ccsm]\033[0m %s\n' "$*" >&2; exit 1; }

# --- 1. Resolve platform ------------------------------------------------------
detect_target() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os" in
    linux)  TARGET_OS="linux" ;;
    darwin) TARGET_OS="macos" ;;
    *) die "unsupported OS: $os (cc-switch-web targets Linux servers; macOS works but is unofficial)" ;;
  esac
  case "$arch" in
    x86_64|amd64) TARGET_ARCH="x86_64" ;;
    aarch64|arm64) TARGET_ARCH="aarch64" ;;
    *) die "unsupported architecture: $arch" ;;
  esac
  TARGET="${TARGET_OS}-${TARGET_ARCH}"
  # No `-apple` suffix here on purpose: the release workflow publishes assets
  # named `cc-switch-web-{linux,macos}-{x86_64,aarch64}.tar.xz` (no `-apple`).
  # Earlier versions of this script appended `-apple` (a Rust target-triple
  # convention) which then 404'd against the actual release artifacts.
}

# --- 2. Resolve version -------------------------------------------------------
resolve_version() {
  if [ -n "${CCSM_VERSION:-}" ]; then
    VERSION="$CCSM_VERSION"
  else
    say "resolving latest release from $GITHUB_REPO"
    VERSION="$(curl -fsSL "https://api.github.com/repos/$GITHUB_REPO/releases/latest" \
      | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' \
      | head -n1)"
    [ -n "$VERSION" ] || die "could not determine latest release tag (set CCSM_VERSION to override)"
  fi
  say "installing version $VERSION"
}

# --- 3. Download + verify -----------------------------------------------------
download_and_verify() {
  local archive="cc-switch-web-${TARGET}.tar.xz"
  local url="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${archive}"
  local sums_url="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/SHA256SUMS"
  # NB: `workdir` is intentionally *not* `local`. The EXIT trap we install
  # below is shell-global, not function-scoped, so the variable it expands
  # must outlive this function. With `set -eu` + `local`, the trap would
  # see an unset $workdir when it eventually fires.
  workdir="$(mktemp -d)"
  trap 'rm -rf "$workdir"' EXIT

  say "downloading $archive"
  curl -fsSL -o "$workdir/$archive" "$url" \
    || die "download failed - is the release published? ($url)"

  say "downloading SHA256SUMS"
  if ! curl -fsSL -o "$workdir/SHA256SUMS" "$sums_url"; then
    warn "checksum manifest missing - skipping verification (NOT recommended)"
  else
    verify_checksum "$workdir" "$archive"
  fi

  say "extracting"
  tar -xJf "$workdir/$archive" -C "$workdir"
  # The release workflow stages the tarball with a top-level
  # `cc-switch-web/` directory holding the binary (plus README/LICENSE/
  # CHANGELOG). After extraction the binary lives at
  # `$workdir/cc-switch-web/cc-switch-web`. Accept the legacy flat layout
  # too in case a future workflow flattens it.
  if   [ -f "$workdir/$BIN_NAME/$BIN_NAME" ]; then BIN_PATH="$workdir/$BIN_NAME/$BIN_NAME"
  elif [ -f "$workdir/$BIN_NAME" ];           then BIN_PATH="$workdir/$BIN_NAME"
  else die "tarball did not contain $BIN_NAME (looked in $workdir/$BIN_NAME/ and $workdir/$BIN_NAME)"
  fi
}

# Verify a downloaded archive's SHA-256 against SHA256SUMS. Kept as a
# function (not a `(...)` subshell) so `local` is valid in POSIX sh.
verify_checksum() {
  local workdir="$1"
  local archive="$2"
  local sums_file="$workdir/SHA256SUMS"
  local archive_path="$workdir/$archive"
  local expected actual
  expected="$(grep "  $archive" "$sums_file" | awk '{print $1}')"
  [ -n "$expected" ] || die "checksum for $archive not found in SHA256SUMS"
  actual="$(sha256sum "$archive_path" | awk '{print $1}')"
  if [ "$expected" != "$actual" ]; then
    die "checksum mismatch: expected $expected got $actual"
  fi
  say "checksum OK"
}

# --- 4. Install ---------------------------------------------------------------
install_binary() {
  # $EUID is a bash/zsh-ism; use `id -u` (POSIX) to detect root.
  if [ -w "$INSTALL_DIR" ] || [ "$(id -u)" -eq 0 ]; then
    install -m 0755 "$BIN_PATH" "$INSTALL_DIR/$BIN_NAME"
  else
    say "$INSTALL_DIR is not writable; using sudo"
    sudo install -m 0755 "$BIN_PATH" "$INSTALL_DIR/$BIN_NAME"
  fi
  say "installed to $INSTALL_DIR/$BIN_NAME"
}

# --- 5. Optional service ------------------------------------------------------
register_service() {
  if [ "$SKIP_SERVICE" = "1" ]; then
    say "skipping service registration (CCSM_NO_SERVICE=1)"
    return
  fi
  if ! command -v systemctl >/dev/null 2>&1; then
    say "systemd not detected - skipping service registration"
    return
  fi

  local unit=/etc/systemd/system/cc-switch-web.service
  say "writing systemd unit at $unit"
  sudo tee "$unit" >/dev/null <<EOF
[Unit]
Description=cc-switch-web headless server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$BIN_NAME
Restart=on-failure
RestartSec=5
User=${SUDO_USER:-${USER}}
Environment=RUST_LOG=info
# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

  sudo systemctl daemon-reload
  sudo systemctl enable --now cc-switch-web.service
  say "service started; check with: systemctl status cc-switch-web"
}

# --- main ---------------------------------------------------------------------
detect_target
resolve_version
download_and_verify
install_binary
register_service

cat <<EOF

cc-switch-web $VERSION installed.

Quick start:
  cc-switch-web                    # serves on http://127.0.0.1:3000
  ssh -L 3000:localhost:3000 user@host  # tunnel from your laptop
  open http://localhost:3000

Service (systemd):
  systemctl status cc-switch-web
  journalctl -u cc-switch-web -f

Customise the data directory or port via CLI flags - run \`cc-switch-web --help\`.
EOF
