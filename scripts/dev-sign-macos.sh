#!/usr/bin/env bash
# dev-sign-macos.sh — Sign the debug binary with a stable identifier.
#
# WHY THIS EXISTS
# ---------------
# macOS TCC (Transparency, Consent, and Control) tracks privacy permissions
# (Screen Recording, Accessibility, etc.) by the app's code-signing identity.
# An unsigned binary has no stable identity, so:
#   - CGPreflightScreenCaptureAccess() always returns false.
#   - Granting permission in System Settings has no lasting effect.
#   - The app doesn't even appear in the Settings list reliably.
#
# Ad-hoc signing with a fixed --identifier gives TCC a stable key
# ("com.snapper-keeper.app") that persists across cargo rebuilds, because
# the identifier string — not the binary hash — is used as the TCC key.
#
# USAGE
# -----
# Run once after your first `pnpm --filter @snk/app tauri dev` compile,
# and again any time TCC stops recognising the binary (rare with --identifier).
#
#   ./scripts/dev-sign-macos.sh
#
# Then grant Screen Recording in:
#   System Settings → Privacy & Security → Screen Recording
# and restart the dev build.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
BINARY="$ROOT/target/debug/snapper-keeper-app"
ENTITLEMENTS="$ROOT/app/src-tauri/entitlements.plist"
IDENTIFIER="com.snapper-keeper.app"

if [[ ! -f "$BINARY" ]]; then
  echo "Error: debug binary not found at $BINARY"
  echo "Run 'pnpm --filter @snk/app tauri dev' first (Ctrl+C once the frontend compiles)."
  exit 1
fi

echo "Signing $BINARY"
echo "  identifier : $IDENTIFIER"
echo "  entitlements: $ENTITLEMENTS"

codesign \
  --force \
  --sign - \
  --identifier "$IDENTIFIER" \
  --entitlements "$ENTITLEMENTS" \
  "$BINARY"

echo ""
echo "Done. Now:"
echo "  1. Open System Settings → Privacy & Security → Screen Recording"
echo "  2. If snapper-keeper-app isn't listed yet, restart the dev build once"
echo "     (the sign is preserved even after cargo rebuilds as long as you"
echo "      re-run this script once per binary on first grant)."
echo "  3. Toggle snapper-keeper-app ON, then restart the dev build."
