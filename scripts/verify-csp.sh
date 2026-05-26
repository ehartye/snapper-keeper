#!/usr/bin/env bash
# Fail loudly if the CSP in app/src-tauri/tauri.conf.json is missing the
# http:// variants of asset.localhost / ipc.localhost.
#
# Background: Tauri 2 on Windows + WebView2 uses http:// for the asset
# and ipc loopback protocols in PACKAGED builds. Dev mode (Vite-served)
# doesn't enforce the configured CSP at all, so a CSP that only lists
# https:// variants passes silently in dev and breaks in production —
# captures render as black (images blocked) and IPC custom-protocol
# falls back to postMessage (the IPC fallback masks the bug, the asset
# fallback doesn't exist).
#
# See:
#   - PR #141 commit cc970ae (fix(security): allow http:// asset.localhost ...)
#   - CLAUDE.md "Tauri 2 gotchas" section
#
# This script is an automated guard against regressions where someone
# "simplifies" the CSP back to a single scheme. It is intentionally
# a literal-string check — Tauri's protocol URL conventions are
# version-dependent, so a precise check beats a clever one.
#
# Required tools on PATH: jq.
# No network access. No side effects.

set -euo pipefail

CONF="app/src-tauri/tauri.conf.json"

if [[ ! -f "$CONF" ]]; then
  echo "::error::expected $CONF to exist" >&2
  exit 1
fi

CSP="$(jq -r '.app.security.csp // empty' "$CONF")"

if [[ -z "$CSP" ]]; then
  echo "::error file=$CONF::app.security.csp is missing or empty"
  exit 1
fi

# Required directive substrings. If Tauri's protocol URL conventions
# change in a future minor version (https → http or back), this list
# is the canonical place to update — paired with a CSP edit.
required=(
  "http://asset.localhost"
  "https://asset.localhost"
  "http://ipc.localhost"
  "https://ipc.localhost"
)

missing=()
for token in "${required[@]}"; do
  if [[ "$CSP" != *"$token"* ]]; then
    missing+=("$token")
  fi
done

if (( ${#missing[@]} > 0 )); then
  echo "::error file=$CONF::CSP is missing required protocol-loopback variants:"
  for token in "${missing[@]}"; do
    echo "::error file=$CONF::  - $token"
  done
  echo "" >&2
  echo "Tauri 2 + WebView2 packaged builds use http:// for asset.localhost" >&2
  echo "and ipc.localhost; dev mode hides this because Vite skips CSP." >&2
  echo "CSP must allow BOTH schemes or installer builds will render images" >&2
  echo "as black (asset protocol) and fall back IPC to postMessage." >&2
  echo "" >&2
  echo "Current CSP:" >&2
  echo "  $CSP" >&2
  exit 1
fi

echo "CSP OK: includes http:// + https:// variants for asset.localhost + ipc.localhost."
