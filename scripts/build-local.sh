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

# --- Pre-build (Windows only) ---
if [[ "$OS" == "windows" ]]; then
  # Install the EXIT trap BEFORE copying, so an interrupted build cleans up.
  # Preserves .placeholder so the bundle resource glob continues to match in dev.
  cleanup_tesseract() {
    if [[ -d "app/src-tauri/resources/tesseract" ]]; then
      find "app/src-tauri/resources/tesseract" -mindepth 1 ! -name '.placeholder' -delete 2>/dev/null || true
    fi
  }
  trap cleanup_tesseract EXIT

  # Resolve a Tesseract source dir using the same resolver order as
  # snk-ocr/sidecar.rs at runtime. Both Program Files locations are
  # checked because the runtime resolver also checks both (older /
  # 32-bit UB-Mannheim installers landed in Program Files (x86)).
  TESSERACT_BIN=""
  if [[ -n "${SNK_TESSERACT_PATH:-}" && -x "$SNK_TESSERACT_PATH" ]]; then
    TESSERACT_BIN="$SNK_TESSERACT_PATH"
  elif command -v tesseract >/dev/null 2>&1; then
    TESSERACT_BIN="$(command -v tesseract)"
  elif [[ -x "/c/Program Files/Tesseract-OCR/tesseract.exe" ]]; then
    TESSERACT_BIN="/c/Program Files/Tesseract-OCR/tesseract.exe"
  elif [[ -x "/c/Program Files (x86)/Tesseract-OCR/tesseract.exe" ]]; then
    TESSERACT_BIN="/c/Program Files (x86)/Tesseract-OCR/tesseract.exe"
  fi

  if [[ -z "$TESSERACT_BIN" ]]; then
    echo "Tesseract not found. Install via 'winget install UB-Mannheim.TesseractOCR' or 'choco install tesseract' (see README -> Prerequisites)." >&2
    echo "Set SNK_TESSERACT_PATH to override." >&2
    exit 1
  fi

  TESSERACT_DIR="$(dirname "$TESSERACT_BIN")"
  echo "build-local: bundling Tesseract from $TESSERACT_DIR"

  mkdir -p "app/src-tauri/resources/tesseract"
  # Copy everything from the install dir; -p preserves attributes.
  # Trailing /. on the source ensures contents-of (not the dir itself) are copied.
  cp -Rp "$TESSERACT_DIR/." "app/src-tauri/resources/tesseract/"
fi

# --- Build ---
echo "build-local: invoking pnpm tauri build"
pnpm --filter @snk/app tauri build \
  --target "$TARGET" \
  --bundles "$BUNDLES" \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'

# TODO(task-5): post-build summary
