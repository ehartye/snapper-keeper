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

case "$OS_RAW" in
  Darwin)
    # The .app bundle dir is well-known but Tauri may name the internal
    # binary differently across versions. Discover by globbing the .app
    # under any release-bundle/macos dir, then picking the first
    # executable file inside Contents/MacOS/.
    shopt -s nullglob
    app_candidates=(
      target/aarch64-apple-darwin/release/bundle/macos/*.app
      target/x86_64-apple-darwin/release/bundle/macos/*.app
      target/release/bundle/macos/*.app
    )
    for app in "${app_candidates[@]}"; do
      [[ -d "$app" ]] || continue
      macos_dir="$app/Contents/MacOS"
      [[ -d "$macos_dir" ]] || continue
      for f in "$macos_dir"/*; do
        if [[ -f "$f" && -x "$f" ]]; then
          echo "$f"
          exit 0
        fi
      done
    done
    echo "resolve-built-binary: no executable found inside any .app/Contents/MacOS:" >&2
    for app in "${app_candidates[@]}"; do
      echo "  $app" >&2
    done
    exit 1
    ;;
  MINGW*|MSYS*|CYGWIN*)
    candidates=(
      "target/x86_64-pc-windows-msvc/release/snapper-keeper-app.exe"
      "target/release/snapper-keeper-app.exe"
    )
    for c in "${candidates[@]}"; do
      if [[ -f "$c" ]]; then
        echo "$c"
        exit 0
      fi
    done
    echo "resolve-built-binary: no built binary found at any candidate path:" >&2
    for c in "${candidates[@]}"; do
      echo "  $c" >&2
    done
    exit 1
    ;;
  *)
    echo "resolve-built-binary: unsupported OS: $OS_RAW" >&2
    exit 1
    ;;
esac
