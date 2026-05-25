#!/usr/bin/env bash
# Fail loudly if TAURI_SIGNING_PRIVATE_KEY does not match the pubkey embedded
# in app/src-tauri/tauri.conf.json plugins.updater.pubkey.
#
# Approach: sign a known canary string with the private key, verify the
# signature with the embedded public key. If verify succeeds, the keys match.
# Tauri 2 has no `tauri signer sign --print-pub-key` flag, so the canary
# roundtrip is the robust pattern.
#
# Required env:
#   TAURI_SIGNING_PRIVATE_KEY          (base64 minisign private key)
#   TAURI_SIGNING_PRIVATE_KEY_PASSWORD (password used at key generation)
#
# Required tools on PATH: minisign, jq, base64.
#
# Side effects: writes/deletes temp files under TMPDIR. No network access.

set -euo pipefail

: "${TAURI_SIGNING_PRIVATE_KEY:?TAURI_SIGNING_PRIVATE_KEY must be set}"
: "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:?TAURI_SIGNING_PRIVATE_KEY_PASSWORD must be set}"

WORK=$(mktemp -d)
trap "rm -rf '$WORK'" EXIT

echo "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d > "$WORK/priv.key"
echo "snapper-keeper-pubkey-drift-canary" > "$WORK/canary.txt"

# Sign the canary with the secret-held private key.
# minisign reads the password from stdin when -W is set or piped.
echo "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" | minisign -S -s "$WORK/priv.key" \
  -m "$WORK/canary.txt" -x "$WORK/canary.sig" >/dev/null

# Verify the canary with the embedded public key.
jq -r .plugins.updater.pubkey app/src-tauri/tauri.conf.json | base64 -d > "$WORK/pub.key"

if ! minisign -V -p "$WORK/pub.key" -m "$WORK/canary.txt" -x "$WORK/canary.sig" >/dev/null 2>&1; then
  echo "::error file=app/src-tauri/tauri.conf.json,line=109::Pubkey drift: TAURI_SIGNING_PRIVATE_KEY does not match plugins.updater.pubkey. Rotate one to match the other before tagging."
  exit 1
fi

echo "Pubkey OK: TAURI_SIGNING_PRIVATE_KEY matches tauri.conf.json plugins.updater.pubkey."
