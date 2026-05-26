#!/usr/bin/env bash

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

cargo test --workspace --exclude snapper-keeper-app export_bindings -- --include-ignored

if [[ -n "$(git status --porcelain -- packages/)" ]]; then
    git --no-pager diff -- packages/
    echo "::error::Generated TS bindings are out of date. Run cargo test --workspace --exclude snapper-keeper-app export_bindings -- --include-ignored and commit packages/*/src/generated/errors.ts."
    exit 1
fi

echo "verify-ts-bindings: OK"
