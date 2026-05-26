#!/usr/bin/env bash
#
# verify-theme-keys.sh — assert TypeScript theme registry keys match
# Rust tray-icon dispatch arms.
#
# Per #67: theme registration was fanned out across multiple sites
# (app/src/lib/theme.ts THEMES + app/src-tauri/src/main.rs tray_icon_for
# match arms + per-theme PNG const declarations). Adding a new theme
# in TS without the corresponding Rust additions would result in the
# theme silently using the default tray icon. Manual sync is brittle;
# this script enforces it via CI.
#
# ## What it checks
#
# 1. Extract THEME_FAMILIES keys from app/src/lib/theme.ts.
# 2. Extract match-arm strings + tray-icon const names from
#    app/src-tauri/src/main.rs.
# 3. Assert the set of TS keys equals the set of Rust-supported
#    families (explicit match arms + the default key, which is "holo").
#
# If a new theme key is added to TS but not Rust (or vice versa),
# fail with a clear message indicating which side is missing.
#
# ## Usage
#
#   ./scripts/verify-theme-keys.sh

set -euo pipefail

THEME_TS="app/src/lib/theme.ts"
MAIN_RS="app/src-tauri/src/main.rs"

# Theme key used as the default in Rust's tray_icon_for. Must match
# the `_` arm of the match expression.
DEFAULT_RUST_KEY="holo"

extract_ts_keys() {
    # Extract the keys of the THEME_FAMILIES object literal. They appear
    # as `<key>: {` or `'<key>': {` at indentation 2 inside the const.
    # Use awk to scope the search to inside THEME_FAMILIES = { ... }.
    awk '
        /^export const THEME_FAMILIES = {/ { in_block = 1; next }
        in_block && /^} as const/ { exit }
        in_block && match($0, /^  '"'"'?([a-z-]+)'"'"'?: {/, arr) { print arr[1] }
    ' "$THEME_TS" | sort -u
}

extract_rust_match_arms() {
    # Extract `"foo" =>` patterns inside tray_icon_for's match expression.
    awk '
        /fn tray_icon_for/ { in_fn = 1 }
        in_fn && /match family/ { in_match = 1; next }
        in_match && /^[[:space:]]*}/ { exit }
        in_match && match($0, /"([a-z-]+)"[[:space:]]*=>/, arr) { print arr[1] }
    ' "$MAIN_RS" | sort -u
}

main() {
    if [[ ! -f "$THEME_TS" ]]; then
        echo "::error::verify-theme-keys: missing $THEME_TS"
        exit 1
    fi
    if [[ ! -f "$MAIN_RS" ]]; then
        echo "::error::verify-theme-keys: missing $MAIN_RS"
        exit 1
    fi

    local ts_keys
    ts_keys=$(extract_ts_keys)
    if [[ -z "$ts_keys" ]]; then
        echo "::error::verify-theme-keys: extracted ZERO keys from $THEME_TS — extraction may be broken"
        exit 1
    fi

    local rust_keys_explicit
    rust_keys_explicit=$(extract_rust_match_arms)

    # Rust supports the explicit arms PLUS the default. The default key
    # is hardcoded to "holo" here; if Rust's match changes its default
    # behavior, this script needs to evolve.
    local rust_keys_supported
    rust_keys_supported=$(printf '%s\n%s\n' "$rust_keys_explicit" "$DEFAULT_RUST_KEY" | sort -u)

    echo "verify-theme-keys: TS keys ($(echo "$ts_keys" | wc -l | tr -d ' ')):"
    echo "$ts_keys" | sed 's/^/  - /'
    echo ""
    echo "verify-theme-keys: Rust-supported keys ($(echo "$rust_keys_supported" | wc -l | tr -d ' '), including default \"$DEFAULT_RUST_KEY\"):"
    echo "$rust_keys_supported" | sed 's/^/  - /'

    # Diff: every TS key must appear in Rust.
    local missing_in_rust
    missing_in_rust=$(comm -23 <(echo "$ts_keys") <(echo "$rust_keys_supported"))

    # And vice-versa: every Rust key must appear in TS (avoids stale
    # match arms for themes that have been removed from TS).
    local missing_in_ts
    missing_in_ts=$(comm -23 <(echo "$rust_keys_supported") <(echo "$ts_keys"))

    if [[ -n "$missing_in_rust" ]]; then
        echo ""
        echo "::error::verify-theme-keys: TS theme(s) without a Rust tray-icon match arm:"
        echo "$missing_in_rust" | sed 's/^/  - /'
        echo ""
        echo "Add the missing key as a match arm in $MAIN_RS::tray_icon_for"
        echo "AND a matching TRAY_<UPPER>_PNG const + icons/sk-<key>.png file."
        exit 1
    fi

    if [[ -n "$missing_in_ts" ]]; then
        echo ""
        echo "::error::verify-theme-keys: Rust match arm(s) without a corresponding TS theme:"
        echo "$missing_in_ts" | sed 's/^/  - /'
        echo ""
        echo "Either add the missing key to THEME_FAMILIES in $THEME_TS,"
        echo "or remove the stale Rust match arm + const + icon file."
        exit 1
    fi

    echo ""
    echo "verify-theme-keys: OK — TS and Rust theme keys agree."
}

main "$@"
