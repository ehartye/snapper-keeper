#!/usr/bin/env bash
# Smoke-test the platform signing toolchain against a throwaway stub binary
# BEFORE touching real release artifacts. If any step fails, the sign job
# aborts and no real artifact gets signed.
#
# Usage: smoke-sign-roundtrip.sh <windows|macos>
#
# Required env (varies by platform):
#   All:     TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD
#   Windows: AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET
#   macOS:   APPLE_SIGNING_IDENTITY (+ build.keychain unlocked beforehand)

set -euo pipefail

PLATFORM="${1:?usage: smoke-sign-roundtrip.sh <windows|macos>}"

# Cheap pubkey-diff first — verify-pubkey job already ran upstream, but it's
# cheap and fails fast before we burn Azure/Apple signing calls.
./scripts/verify-pubkey.sh

cargo build -p snk-smoke-target --release

case "$PLATFORM" in
  windows)
    cp target/release/snk-smoke-target.exe ./smoke.exe
    sign code artifact-signing \
      -ase https://eus.codesigning.azure.net \
      -asa HartyeTech \
      -ascp snapper-keeper \
      -v Information \
      ./smoke.exe
    signtool verify /pa /v ./smoke.exe
    ;;
  macos)
    cp target/release/snk-smoke-target ./smoke
    codesign --sign "$APPLE_SIGNING_IDENTITY" --options runtime --timestamp ./smoke
    codesign --verify --strict --verbose=2 ./smoke
    ;;
  *)
    echo "::error::Unknown platform: $PLATFORM"
    exit 2
    ;;
esac

rm -f ./smoke ./smoke.exe
echo "Smoke roundtrip passed for $PLATFORM."
