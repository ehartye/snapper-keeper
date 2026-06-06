#!/usr/bin/env bash
#
# release-readiness.sh — single enforced entry point for the spec §10.5
# release gate (issue #44).
#
# The tag-triggered release pipeline (release.yml) builds, signs, and publishes
# but does NOT otherwise re-run the lint / clippy / test / verify gates that
# live in ci.yml for PRs. So a tag pushed to a non-green commit would sail
# through. This script runs every AUTOMATABLE gate against the working tree and
# prints a PASS/FAIL table, then prints the §10.5 MANUAL checklist that a human
# must confirm. The release-readiness job in release.yml runs it with --ci and
# gates the build on it; the manual items are confirmed by the approver at the
# production-release environment gate.
#
# Usage:
#   scripts/release-readiness.sh         # local: automated gates + manual list
#   scripts/release-readiness.sh --ci    # CI: automated gates, gate the build
#
# Exits non-zero if any automated gate fails.
#
# Design note (issue #44): the issue floated "refuse to run unless the script
# was committed within 24h" as the enforcement. Running it IN the pipeline on
# the exact tagged commit is strictly stronger than a freshness heuristic, so we
# do that instead.

set -uo pipefail

CI_MODE=0
[[ "${1:-}" == "--ci" ]] && CI_MODE=1

cd "$(git rev-parse --show-toplevel)"

declare -a RESULTS
overall=0

gate() {
    local name="$1"
    shift
    printf '→ %-26s ' "$name"
    local out
    if out="$("$@" 2>&1)"; then
        printf 'PASS\n'
        RESULTS+=("PASS  $name")
    else
        printf 'FAIL\n'
        echo "----- $name output (tail) -----"
        echo "$out" | tail -25
        echo "-------------------------------"
        RESULTS+=("FAIL  $name")
        overall=1
    fi
}

# A grep that must find NOTHING (used for the plugin-reach-in guard).
forbid_grep() {
    ! grep -rn "$1" "$2" >/dev/null 2>&1
}

echo "================ Automated release-readiness gates ================"
# Rust — mirror ci.yml's rust-test job (the app crate needs a real webview
# runtime to build, so it is excluded here as it is in CI).
gate "rustfmt"                 cargo fmt --all -- --check
gate "clippy"                  cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
gate "rust tests"              cargo test --workspace --exclude snapper-keeper-app
gate "no plugin reach-ins"     forbid_grep "snk_library::plugin::" crates/

# Frontend — mirror ci.yml's lint-typecheck + ts-test jobs.
gate "eslint"                  pnpm -r run lint
gate "typecheck"               pnpm -r run typecheck
gate "frontend tests"          pnpm -r run test

# Release-correctness static guards (not covered by the build matrix).
gate "CSP scheme guard"        bash scripts/verify-csp.sh
gate "doc CLAIM coverage"      bash scripts/verify-docs.sh
gate "TS bindings fresh"       bash scripts/verify-ts-bindings.sh
gate "theme keys aligned"      bash scripts/verify-theme-keys.sh
gate "string-result allowlist" bash scripts/verify-string-result-allowlist.sh

echo
echo "============================ Summary =============================="
printf '  %s\n' "${RESULTS[@]}"

echo
cat <<'EOF'
================= Manual §10.5 checklist (human sign-off) =================
Verify on a real interactive desktop before approving the production-release
environment gate (these are interactive and cannot be automated):

  [ ] Fresh install · first-run wizard · permission grant flow (macOS)
  [ ] Capture: region · window · screen · timed
  [ ] Annotate: all tools · undo · save
  [ ] Clipboard popup against VS Code · browser address bar · Slack · Terminal · Word/Pages
  [ ] Multi-monitor: capture per monitor · correct DPI
  [ ] Sensitive clipboard: 1Password copy → NOT in history
  [ ] Update flow: install old build · check for update · apply · relaunch
  [ ] Sanity: log folder contains no clipboard / OCR text
EOF

echo
if [[ $overall -ne 0 ]]; then
    echo "RESULT: automated gates FAILED — do NOT tag / release until green."
    exit 1
fi

if [[ $CI_MODE -eq 1 ]]; then
    echo "RESULT: automated gates PASS. The manual §10.5 items above are the"
    echo "        approval criteria for the production-release environment gate."
else
    echo "RESULT: automated gates PASS. Complete the manual checklist above,"
    echo "        then tag the release (git tag vX.Y.Z && git push --tags)."
fi
