#!/usr/bin/env bash
#
# Build an unsigned production-fidelity installer locally.
#
# Usage:
#   pnpm build:local
# or:
#   bash scripts/build-local.sh
#
# See docs/superpowers/specs/2026-05-26-local-installer-build-design.md
# for the full design rationale.

set -euo pipefail

trap 'echo "build-local: failed at line ${LINENO}" >&2' ERR

# Resolve repo root so the script works from any subdirectory.
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Detect OS and architecture.
OS_RAW="$(uname -s)"
ARCH_RAW="$(uname -m)"

case "$OS_RAW" in
  Darwin)
    OS="macos"
    case "$ARCH_RAW" in
      arm64)  TARGET="aarch64-apple-darwin" ;;
      x86_64) TARGET="x86_64-apple-darwin" ;;
      *)
        echo "build-local: unsupported macOS architecture: $ARCH_RAW" >&2
        exit 1
        ;;
    esac
    BUNDLES="app,dmg"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    OS="windows"
    if [[ "$ARCH_RAW" != "x86_64" ]]; then
      echo "build-local: unsupported Windows architecture: $ARCH_RAW (only x86_64 is supported)" >&2
      exit 1
    fi
    TARGET="x86_64-pc-windows-msvc"
    BUNDLES="nsis"
    ;;
  Linux)
    echo "Local installer build is supported on Windows + macOS only (matches production targets)." >&2
    echo "Run 'pnpm tauri dev' to develop on Linux." >&2
    exit 1
    ;;
  *)
    echo "build-local: unsupported OS: $OS_RAW" >&2
    exit 1
    ;;
esac

echo "build-local: OS=$OS  TARGET=$TARGET  BUNDLES=$BUNDLES"

# TODO(task-3): macOS build invocation
# TODO(task-4): Windows pre-build + build invocation
# TODO(task-5): post-build summary
