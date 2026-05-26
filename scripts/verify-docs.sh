#!/usr/bin/env bash
#
# verify-docs.sh — assert every documented claim has at least one
# matching VERIFIED tag somewhere in the codebase.
#
# Per #59: doc/code drift is a recurring pattern. Three of four
# perspectives in the 2026-05-24 prerelease review independently
# identified specific instances. This script is the systemic fix —
# catches the next instance in CI before the doc and code diverge.
#
# ## Tag taxonomy
#
# In docs (Markdown):
#   <!-- CLAIM: <doc>/<claim-id> -->
#
# In code (Rust // or TypeScript //):
#   VERIFIED: <doc>/<claim-id>
#
# The pair "<doc>/<claim-id>" must match exactly. Multiple VERIFIED
# tags for the same CLAIM are allowed (e.g. the same claim is backed
# by multiple test paths). A CLAIM with NO matching VERIFIED tag
# fails the script. Orphan VERIFIED tags (no matching CLAIM) are
# reported as warnings but don't fail.
#
# ## Usage
#
#   ./scripts/verify-docs.sh
#
# Returns 0 on success, non-zero on any CLAIM with missing coverage.
# CI runs this in the verify-docs job.

set -euo pipefail

# Collect CLAIMs from doc files. Searches PRIVACY.md, README.md, and
# the design spec for `<!-- CLAIM: ... -->` patterns.
DOC_PATHS=(
    "PRIVACY.md"
    "README.md"
    "docs/superpowers/specs/2026-05-20-snapper-keeper-design.md"
)

# Collect VERIFIED tags from code. Rust + TypeScript files.
CODE_GLOBS=(
    "crates/**/*.rs"
    "app/**/*.rs"
    "app/**/*.ts"
    "app/**/*.tsx"
    "packages/**/*.ts"
    "packages/**/*.tsx"
)

extract_claims() {
    local found=()
    for f in "${DOC_PATHS[@]}"; do
        if [[ -f "$f" ]]; then
            # `<!-- CLAIM: foo/bar -->` -> capture `foo/bar`
            while IFS= read -r id; do
                [[ -n "$id" ]] && found+=("$id")
            done < <(
                grep -oE 'CLAIM:[[:space:]]+[^[:space:]>]+' "$f" 2>/dev/null \
                    | sed -E 's/^CLAIM:[[:space:]]+//' \
                    || true
            )
        fi
    done
    printf '%s\n' "${found[@]:-}" | sort -u
}

extract_verified() {
    local found=()
    # Use git ls-files to respect .gitignore + only check tracked files.
    # Filter to the supported extensions.
    while IFS= read -r f; do
        if [[ -f "$f" ]]; then
            # `VERIFIED: foo/bar` -> capture `foo/bar`
            while IFS= read -r id; do
                [[ -n "$id" ]] && found+=("$id")
            done < <(
                grep -oE 'VERIFIED:[[:space:]]+[^[:space:]*]+' "$f" 2>/dev/null \
                    | sed -E 's/^VERIFIED:[[:space:]]+//' \
                    || true
            )
        fi
    done < <(git ls-files '*.rs' '*.ts' '*.tsx' 2>/dev/null || true)
    printf '%s\n' "${found[@]:-}" | sort -u
}

main() {
    echo "verify-docs: extracting CLAIMs from docs..."
    local claims
    claims=$(extract_claims)
    if [[ -z "$claims" ]]; then
        echo "verify-docs: no CLAIMs found in docs. Add <!-- CLAIM: <id> --> markers to PRIVACY.md / README / design spec to enable verification."
        echo "verify-docs: this is a non-blocking warning during the bootstrap period."
        exit 0
    fi

    echo "verify-docs: found $(echo "$claims" | wc -l | tr -d ' ') CLAIM(s):"
    echo "$claims" | sed 's/^/  - /'

    echo ""
    echo "verify-docs: extracting VERIFIED tags from code..."
    local verified
    verified=$(extract_verified)
    if [[ -n "$verified" ]]; then
        echo "verify-docs: found $(echo "$verified" | wc -l | tr -d ' ') VERIFIED tag(s):"
        echo "$verified" | sed 's/^/  - /'
    else
        echo "verify-docs: no VERIFIED tags found."
    fi

    echo ""
    echo "verify-docs: checking coverage..."
    local missing=()
    while IFS= read -r claim; do
        [[ -z "$claim" ]] && continue
        if ! echo "$verified" | grep -qxF "$claim"; then
            missing+=("$claim")
        fi
    done <<< "$claims"

    if (( ${#missing[@]} > 0 )); then
        echo ""
        echo "::error::verify-docs: ${#missing[@]} CLAIM(s) without matching VERIFIED tag:"
        printf '  - %s\n' "${missing[@]}"
        echo ""
        echo "Add a corresponding 'VERIFIED: <id>' comment to a Rust or TypeScript file"
        echo "that backs each claim, then re-run this script."
        exit 1
    fi

    # Orphan check: VERIFIED tags without a matching CLAIM (warnings, not errors).
    local orphans=()
    while IFS= read -r v; do
        [[ -z "$v" ]] && continue
        if ! echo "$claims" | grep -qxF "$v"; then
            orphans+=("$v")
        fi
    done <<< "$verified"

    if (( ${#orphans[@]} > 0 )); then
        echo ""
        echo "::warning::verify-docs: ${#orphans[@]} orphan VERIFIED tag(s) (no matching CLAIM):"
        printf '  - %s\n' "${orphans[@]}"
        echo ""
        echo "These tags reference claims not present in any doc. Either add a"
        echo "<!-- CLAIM: <id> --> to the relevant doc, or remove the tag."
    fi

    echo ""
    echo "verify-docs: OK — all $(echo "$claims" | wc -l | tr -d ' ') CLAIM(s) have matching VERIFIED tag(s)."
}

main "$@"
