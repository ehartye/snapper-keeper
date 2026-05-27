#!/usr/bin/env bash

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

ALLOWLIST="crates/.string-result-allowlist"

if [[ ! -f "$ALLOWLIST" ]]; then
    echo "::error::Allowlist file $ALLOWLIST is missing. Create it (may be empty) to enforce the Result<_, String> policy."
    exit 1
fi

ACTUAL="$(mktemp /tmp/snk_actual.XXXXXX)"
EXPECTED="$(mktemp /tmp/snk_expected.XXXXXX)"
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
