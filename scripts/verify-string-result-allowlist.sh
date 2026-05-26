#!/usr/bin/env bash

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

ALLOWLIST="crates/.string-result-allowlist"
ACTUAL="$(mktemp)"
EXPECTED="$(mktemp)"
trap 'rm -f "$ACTUAL" "$EXPECTED"' EXIT

{
    grep -rnE 'Result<.*, String>' crates/*/src/plugin.rs crates/*/src/commands.rs || true
} | sort > "$ACTUAL"

{ grep -vE '^\s*(#|$)' "$ALLOWLIST" || true; } | sort > "$EXPECTED"

if ! diff -u "$EXPECTED" "$ACTUAL"; then
    echo "::error::Result<_, String> signatures in crates/*/src/{plugin,commands}.rs differ from crates/.string-result-allowlist"
    exit 1
fi

echo "verify-string-result-allowlist: OK"
