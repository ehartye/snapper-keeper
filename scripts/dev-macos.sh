#!/usr/bin/env bash
# dev-macos.sh — macOS dev launcher for snapper-keeper.
#
# WHY THIS EXISTS
# ---------------
# macOS TCC tracks Screen Recording permission by code-signing identity.
# An unsigned binary has no stable identity, so CGPreflightScreenCaptureAccess()
# always returns false and permission grants don't stick.
#
# This script:
#   1. Builds the Rust binary (cargo build)
#   2. Ad-hoc signs it with a stable --identifier so TCC uses "com.snapper-keeper.app"
#      as the key instead of the binary hash — the grant survives cargo rebuilds
#   3. Launches tauri dev (frontend-only hot-reload keeps the signed binary intact)
#
# NOTE: tauri dev runs with --no-watch (Rust file watcher disabled).
# Frontend (Vite) still hot-reloads normally. For Rust changes, Ctrl+C
# and re-run this script.
#
# FIRST-TIME SETUP
# ----------------
# Run this script, then:
#   System Settings → Privacy & Security → Screen Recording → enable snapper-keeper-app

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
BINARY="$ROOT/target/debug/snapper-keeper-app"
ENTITLEMENTS="$ROOT/app/src-tauri/entitlements.plist"
IDENTIFIER="com.snapper-keeper.app"

echo "→ Building Rust binary..."
cargo build --manifest-path "$ROOT/app/src-tauri/Cargo.toml"

echo "→ Signing with identifier '$IDENTIFIER'..."
codesign \
  --force \
  --sign - \
  --identifier "$IDENTIFIER" \
  --entitlements "$ENTITLEMENTS" \
  "$BINARY"

echo "→ Launching tauri dev (Rust watch disabled — re-run this script after Rust changes)..."
cd "$ROOT"
exec pnpm --filter @snk/app tauri dev --no-watch
