#!/usr/bin/env bash
# dev-macos.sh — Build a bundled macOS .app, ad-hoc sign it, and launch via
#                Launch Services.
#
# WHY THIS EXISTS
# ---------------
# macOS TCC tracks Screen Recording permission by code-signing identity.
# Raw binaries (cargo build output) and tauri dev (unpackaged) have no stable
# bundle identity, so:
#   - CGPreflightScreenCaptureAccess() always returns false
#   - Permission grants don't stick across runs
#   - The app may not appear reliably in System Settings → Privacy & Security
#
# Building a proper .app bundle and ad-hoc signing it with the app's bundle ID
# as the signing identifier gives TCC a stable key.  Launch Services registers
# the bundle, making the permission grant durable.
#
# For UI iteration (hot-reload), use:
#   pnpm --filter @snk/app tauri dev
#
# For capture validation (bundled runtime, stable TCC identity), use:
#   pnpm dev:mac-capture   (which calls this script)
#
# FIRST-TIME SETUP
# ----------------
# Run this script, then:
#   System Settings → Privacy & Security → Screen Recording → enable Snapper Keeper

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
ENTITLEMENTS="$ROOT/app/src-tauri/entitlements.plist"
IDENTIFIER="com.snapper-keeper.app"
PRODUCT_NAME="Snapper Keeper"

ARCH_RAW="$(uname -m)"
case "$ARCH_RAW" in
  arm64)  TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *)
    echo "dev-macos: unsupported architecture: $ARCH_RAW" >&2
    exit 1
    ;;
esac

BUNDLE_PATH="$ROOT/target/$TARGET/debug/bundle/macos/${PRODUCT_NAME}.app"
EXECUTABLE_PATH="$BUNDLE_PATH/Contents/MacOS/snapper-keeper-app"

echo "→ Building bundled macOS app (debug, unsigned)..."
(
  cd "$ROOT/app"
  pnpm exec tauri build \
    --debug \
    --target "$TARGET" \
    --bundles app \
    --no-sign \
    --config '{"bundle":{"createUpdaterArtifacts":false}}'
)

echo "→ Ad-hoc signing app bundle with identifier '$IDENTIFIER'..."
codesign \
  --force \
  --deep \
  --sign - \
  --identifier "$IDENTIFIER" \
  --entitlements "$ENTITLEMENTS" \
  "$BUNDLE_PATH"

echo ""
echo "==================================================================="
echo "  Bundle info"
echo "==================================================================="
printf '  Bundle path:  %s\n' "$BUNDLE_PATH"
printf '  Executable:   %s\n' "$EXECUTABLE_PATH"
printf '  Bundle ID:    %s\n' "$IDENTIFIER"
echo ""
echo "  Signature:"
codesign -dv "$BUNDLE_PATH" 2>&1 | sed 's/^/    /'

echo ""
echo "→ Launching via Launch Services..."
open -n "$BUNDLE_PATH"
