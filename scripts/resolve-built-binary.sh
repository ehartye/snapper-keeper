#!/usr/bin/env bash
# Resolves the path to the built snapper-keeper binary after
# `bash scripts/build-local.sh` (or equivalent `pnpm tauri build`).
#
# Outputs the binary path to stdout. Exits non-zero with a diagnostic
# message on stderr if no candidate path matches.
#
# Used by the e2e-process-smoke CI job to set SNK_BINARY_PATH.

set -Eeuo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

OS_RAW="$(uname -s)"

# Note: Tauri macOS .app bundles name the internal binary after
# `productName` (with spaces) — currently "Snapper Keeper". On Windows
# the unpackaged release exe matches the package name with hyphens.
case "$OS_RAW" in
  Darwin)
    candidates=(
      "target/aarch64-apple-darwin/release/bundle/macos/Snapper Keeper.app/Contents/MacOS/Snapper Keeper"
      "target/x86_64-apple-darwin/release/bundle/macos/Snapper Keeper.app/Contents/MacOS/Snapper Keeper"
      "target/release/bundle/macos/Snapper Keeper.app/Contents/MacOS/Snapper Keeper"
    )
    ;;
  MINGW*|MSYS*|CYGWIN*)
    candidates=(
      "target/x86_64-pc-windows-msvc/release/snapper-keeper-app.exe"
      "target/release/snapper-keeper-app.exe"
    )
    ;;
  *)
    echo "resolve-built-binary: unsupported OS: $OS_RAW" >&2
    exit 1
    ;;
esac

for c in "${candidates[@]}"; do
  if [[ -f "$c" && -x "$c" ]]; then
    echo "$c"
    exit 0
  fi
  # On Windows, exes are -x but may not have the +x bit reported via stat;
  # accept -f as sufficient if the extension matches.
  if [[ -f "$c" && "$c" == *.exe ]]; then
    echo "$c"
    exit 0
  fi
done

echo "resolve-built-binary: no built binary found at any candidate path:" >&2
for c in "${candidates[@]}"; do
  echo "  $c" >&2
done
exit 1
