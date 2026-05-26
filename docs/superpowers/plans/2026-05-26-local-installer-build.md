# Local installer build (unsigned) — implementation plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Make `pnpm build:local` produce production-fidelity unsigned installers on Windows + macOS, and remove the dead-weight `bundle.windows.signCommand` from base `tauri.conf.json` (production Windows signing already happens directly in the `sign-win-x64` CI job, not via Tauri's hook).

**Architecture:** One bash script (`scripts/build-local.sh`) is the entry point. It runs natively on macOS and inside Git Bash on Windows. Per-OS branches handle target-triple computation, Tesseract bundling on Windows (copied from the user's existing install), and the `tauri build` invocation with `--config '{"bundle":{"createUpdaterArtifacts":false}}'` overlay to skip the updater-payload signing step. A precursor refactor removes `bundle.windows.signCommand` from base config and the corresponding `Strip signCommand` step from `release.yml` — these are dead weight that exists only as a workaround for itself.

**Tech Stack:** bash, jq (unused after refactor), pnpm, Tauri 2 CLI, Git Bash (Windows), Homebrew tesseract (macOS), winget/choco tesseract (Windows).

**Spec:** [`docs/superpowers/specs/2026-05-26-local-installer-build-design.md`](../specs/2026-05-26-local-installer-build-design.md)

---

## Implementer notes (read before starting)

- **CLAUDE.md worktree convention:** create the worktree as a sibling — `C:/Users/ehart/repos/snapper-keeper-worktrees/<branch>/`, **not** inside the repo. The execution skill handles this.
- **One task = one commit.** Conventional Commits required (`feat(scope):`, `chore:`, `docs:`, `refactor:`, etc.). Commit messages are quoted exactly in each task.
- **Stage files explicitly** with `git add path/to/file` — never `git add .` or `-A` (per CLAUDE.md).
- **Per-OS verification limitation:** Tasks 3 (macOS) and 4 (Windows) require an interactive desktop on that OS to fully verify. If the implementer doesn't have access to a given OS, write the code per the plan and defer the runtime verification to the user — call this out in the commit message or PR notes. CI's `build-app` job will verify cross-OS compilation but does NOT run the script or produce installers.
- **`set -euo pipefail`:** all bash assumes strict mode. Quote every variable expansion; paths contain spaces (`"Snapper Keeper.app"`).
- **Trap ordering matters:** the EXIT trap must be installed *before* the Tesseract copy step on Windows so that an interrupted build still cleans up.
- **SHA-256 + file-size portability:** Git Bash on Windows ships GNU coreutils (including `wc`, `sha256sum`), and macOS ships BSD versions with the same `wc -c` semantics for byte count. The plan uses `wc -c < "$file"` (portable bytes) + `sha256sum` (portable on both via `shasum -a 256` fallback on macOS — see Task 5).

---

## Task 1: Remove `bundle.windows.signCommand` from base config + clean up `release.yml`

**Rationale:** This is a precursor refactor that lands independent of the script. After it, `pnpm tauri build` on Windows no longer attempts to invoke `sign code ...`, but it still fails on `createUpdaterArtifacts:true` requiring `TAURI_SIGNING_PRIVATE_KEY` — that's by design; the script's `--config` overlay handles it. The `sign code` invocation that actually signs production builds is in `release.yml`'s `sign-win-x64` job (lines 495–524) and does NOT route through Tauri's hook, so removing the field has zero impact on signed releases.

**Files:**
- Modify: `app/src-tauri/tauri.conf.json` (lines 114–125, remove the `bundle.windows` block)
- Modify: `.github/workflows/release.yml` (lines ~100–117, delete the `Strip signCommand from tauri.conf.json (Windows build)` step and its preamble comment)

### Step 1: Remove the `bundle.windows` block from `tauri.conf.json`

Open `app/src-tauri/tauri.conf.json`. The current `bundle` block ends like this:

```json
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true,
    "icon": ["icons/icon.ico", "icons/icon.png"],
    "resources": {
      "resources/tesseract/**/*": "tesseract/"
    },
    "windows": {
      "signCommand": "sign code artifact-signing -ase https://eus.codesigning.azure.net -asa HartyeTech -ascp snapper-keeper %1"
    }
  },
```

Change it to:

```json
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true,
    "icon": ["icons/icon.ico", "icons/icon.png"],
    "resources": {
      "resources/tesseract/**/*": "tesseract/"
    }
  },
```

(Removed: the trailing comma on the `resources` closing brace and the entire `"windows": { ... }` block.)

### Step 2: Verify JSON is still valid

Run from repo root:

```bash
jq '.' app/src-tauri/tauri.conf.json > /dev/null && echo "OK"
```

Expected: `OK` (no jq parse error).

### Step 3: Delete the strip step from `release.yml`

Open `.github/workflows/release.yml`. Find this block (around lines 100–117):

```yaml
      # Windows: delete bundle.windows.signCommand from tauri.conf.json
      # before building. The build job is secrets-free, so it cannot
      # invoke `dotnet sign` (no Azure creds). Attempts to disable the
      # signCommand via --config overlay don't work:
      #   - `signCommand: ""` -> Tauri tries to exec the empty command
      #     and fails with "program path has no file name".
      #   - `signCommand: null` -> unverified at time of writing; jq -d
      #     is the robust fix that just removes the field entirely.
      # The sign-win-x64 job runs `dotnet sign` against the unsigned
      # installer instead.
      - name: Strip signCommand from tauri.conf.json (Windows build)
        if: runner.os == 'Windows'
        shell: bash
        run: |
          jq 'del(.bundle.windows.signCommand)' app/src-tauri/tauri.conf.json > /tmp/conf.json
          mv /tmp/conf.json app/src-tauri/tauri.conf.json
          echo "windows.signCommand after patch:"
          jq '.bundle.windows.signCommand // "(absent)"' app/src-tauri/tauri.conf.json
```

Delete the entire block (preamble comment + the step). Replace the deleted block with a single blank line so the file stays readable.

### Step 4: Verify the `Build Tauri app (unsigned)` step's comment is still coherent

The next step in `release.yml` (`Build Tauri app (unsigned)`, lines ~119 onward in the original file) has a multi-line preamble comment explaining `createUpdaterArtifacts: false` and `shell: bash`. That comment is self-contained — it does NOT reference the deleted strip step. Read it to confirm; no edit expected. If a future change to that comment HAD added a reference to the strip step, edit it out here. Today: nothing to do in this step.

### Step 5: Verify release.yml is still valid YAML

Pick whichever validator is available locally:

```bash
# Preferred (if installed): actionlint catches GH Actions schema issues too
actionlint .github/workflows/release.yml

# Or generic YAML validation:
npx --yes yaml-lint .github/workflows/release.yml

# Or python (if on PATH):
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "OK"
```

If none of these are easy to install, skip — the YAML will fail loudly in CI on push if broken, and we run the green-CI check in Task 9 Verification 1.

### Step 6: Commit

```bash
git add app/src-tauri/tauri.conf.json .github/workflows/release.yml
git commit -m "refactor(release): remove dead-weight bundle.windows.signCommand from base config

The signCommand field has been dead weight since release-pipeline
hardening: the sign-win-x64 job invokes 'sign code' directly on the
downloaded unsigned .exe, not via Tauri's signCommand hook. Keeping
it in base config forced CI to mutate tauri.conf.json with jq before
the build job, and made local builds fail without Azure credentials.

Removing the field has zero impact on production signing and unblocks
the upcoming 'pnpm build:local' entry point."
```

---

## Task 2: Create `scripts/build-local.sh` skeleton (preamble + OS detection + Linux abort)

**Rationale:** Land the bones of the script first — preamble, OS / arch detection, target-triple computation, and the Linux abort path. No build invocation yet. This keeps the diff reviewable and gives us an entry point we can sanity-check before adding the heavy lifting.

**Files:**
- Create: `scripts/build-local.sh`

### Step 1: Create the script

Create `scripts/build-local.sh` with exactly this content:

```bash
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
```

Make it executable:

```bash
chmod +x scripts/build-local.sh
```

### Step 2: Verify Linux abort path

If you have access to Linux or WSL, run:

```bash
bash scripts/build-local.sh
```

Expected output:
```
Local installer build is supported on Windows + macOS only (matches production targets).
Run 'pnpm tauri dev' to develop on Linux.
```
Exit code: 1.

If you don't have Linux access, simulate by forcing the case match. Add this line temporarily after `OS_RAW=...`:

```bash
OS_RAW="Linux"
```

Run the script, observe the same output and exit code, then remove the temporary line.

### Step 3: Verify OS detection on your current OS

Run:

```bash
bash scripts/build-local.sh
```

Expected: prints one line like `build-local: OS=macos  TARGET=aarch64-apple-darwin  BUNDLES=app,dmg` (or the Windows / Intel-Mac equivalent), then exits 0 (no errors because the body is just TODOs).

### Step 4: Commit

```bash
git add scripts/build-local.sh
git commit -m "chore(scripts): add build-local.sh skeleton with OS detection

Preamble, repo-root resolution, OS/arch detection, target-triple +
bundles computation, and Linux abort path. Build invocation, Tesseract
handling, and post-build summary land in subsequent commits."
```

---

## Task 3: Add macOS build invocation

**Rationale:** macOS needs no pre-build work — Tauri's bundler handles `.app` + `.dmg` directly. Wire up the build invocation for the Darwin branch, then verify locally on macOS.

**Files:**
- Modify: `scripts/build-local.sh`

### Step 1: Replace the `TODO(task-3)` line

Open `scripts/build-local.sh`. Find:

```bash
# TODO(task-3): macOS build invocation
# TODO(task-4): Windows pre-build + build invocation
# TODO(task-5): post-build summary
```

Replace the block with:

```bash
# --- Pre-build (Windows only — task 4) ---
# TODO(task-4): Tesseract resolver + copy + EXIT trap

# --- Build ---
echo "build-local: invoking pnpm tauri build"
pnpm --filter @snk/app tauri build \
  --target "$TARGET" \
  --bundles "$BUNDLES" \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'

# TODO(task-5): post-build summary
```

(The `pnpm tauri build` invocation is common to both OSes. Windows-specific pre-build work — Tesseract — lands in Task 4 above the build call.)

### Step 2: Verify on macOS

Skip this step if you're not on a macOS machine; instead, ensure the script changes compile (the `bash -n` syntax check) and defer runtime verification to the user.

Syntax check (any OS):
```bash
bash -n scripts/build-local.sh && echo "syntax OK"
```
Expected: `syntax OK`.

On macOS, full run:
```bash
bash scripts/build-local.sh
```

Expected behavior:
- Prints `build-local: OS=macos  TARGET=...  BUNDLES=app,dmg`
- Prints `build-local: invoking pnpm tauri build`
- `tauri build` runs (~5–15 min cold). The build may emit Apple-codesign warnings (`SKIP - skipping code signing`) — these are expected for unsigned builds and not failures.
- On success, these artifacts exist:
  - `target/<TARGET>/release/bundle/macos/Snapper Keeper.app`
  - `target/<TARGET>/release/bundle/dmg/Snapper Keeper_<version>_<arch>.dmg`

Verify with:
```bash
ls "target/$(uname -m | sed 's/arm64/aarch64/')-apple-darwin/release/bundle/macos/"
ls "target/$(uname -m | sed 's/arm64/aarch64/')-apple-darwin/release/bundle/dmg/"
```

You should see `Snapper Keeper.app` and a `.dmg` respectively.

### Step 3: Smoke-launch the produced app (optional but recommended)

```bash
open "target/<TARGET>/release/bundle/macos/Snapper Keeper.app"
```

If Gatekeeper blocks it (first launch of an unsigned binary), right-click → Open → Open anyway. The library window should appear; the tray icon should appear. Quit the app.

### Step 4: Commit

```bash
git add scripts/build-local.sh
git commit -m "feat(build): add macOS branch to build-local.sh

Tauri's bundler handles .app + .dmg natively for macOS with no
pre-build work. The --config overlay disables updater-artifact
generation, which would otherwise demand TAURI_SIGNING_PRIVATE_KEY.

DMG layout differs cosmetically from production (which uses
Homebrew's create-dmg with custom volname/window-size in the
sign-mac-* CI jobs); installable contents are functionally
identical."
```

---

## Task 4: Add Windows pre-build (Tesseract resolver + copy + EXIT trap)

**Rationale:** Production Windows installers bundle Tesseract OCR. CI gets this by running `choco install tesseract` and copying `C:\Program Files\Tesseract-OCR\*` into `app/src-tauri/resources/tesseract/` before the build. Locally, the contributor already has Tesseract installed (README requires it for dev). We resolve the existing install and copy from there — no admin elevation, no network hit.

**Files:**
- Modify: `scripts/build-local.sh`

### Step 1: Replace the `TODO(task-4)` line

Open `scripts/build-local.sh`. Find:

```bash
# --- Pre-build (Windows only — task 4) ---
# TODO(task-4): Tesseract resolver + copy + EXIT trap
```

Replace with:

```bash
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
  # snk-ocr/sidecar.rs at runtime.
  TESSERACT_BIN=""
  if [[ -n "${SNK_TESSERACT_PATH:-}" && -x "$SNK_TESSERACT_PATH" ]]; then
    TESSERACT_BIN="$SNK_TESSERACT_PATH"
  elif command -v tesseract >/dev/null 2>&1; then
    TESSERACT_BIN="$(command -v tesseract)"
  elif [[ -x "/c/Program Files/Tesseract-OCR/tesseract.exe" ]]; then
    TESSERACT_BIN="/c/Program Files/Tesseract-OCR/tesseract.exe"
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
```

(The block is wrapped in `if [[ "$OS" == "windows" ]]` so it's a true no-op on macOS.)

### Step 2: Syntax check

```bash
bash -n scripts/build-local.sh && echo "syntax OK"
```
Expected: `syntax OK`.

### Step 3: Verify on Windows (interactive desktop required)

Skip this step if you're not on a Windows interactive desktop (RDP / console / GUI terminal). Per CLAUDE.md, SSH sessions can't run interactive builds. Defer runtime verification to the user.

Open Git Bash (or any bash) in the worktree. Confirm Tesseract is installed:
```bash
where tesseract.exe
```
Expected: a path like `/c/Program Files/Tesseract-OCR/tesseract.exe`. If empty, run `winget install UB-Mannheim.TesseractOCR` first.

Then:
```bash
bash scripts/build-local.sh
```

Expected behavior:
- Prints `build-local: OS=windows  TARGET=x86_64-pc-windows-msvc  BUNDLES=nsis`
- Prints `build-local: bundling Tesseract from <path>`
- `tauri build` runs (~10–20 min cold).
- After build completes successfully:
  - `target/x86_64-pc-windows-msvc/release/bundle/nsis/Snapper Keeper_<version>_x64-setup.exe` exists.
  - `app/src-tauri/resources/tesseract/` contains only `.placeholder` (the EXIT trap removed the copied files).

Verify with:
```bash
ls "target/x86_64-pc-windows-msvc/release/bundle/nsis/"
ls -A "app/src-tauri/resources/tesseract/"
```

Second listing should show only `.placeholder`.

### Step 4: Confirm working-tree cleanliness

```bash
git status
```

Expected: no changes to `app/src-tauri/resources/tesseract/` (everything inside is gitignored at `.gitignore:23-24`).

### Step 5: Smoke-install the produced .exe (optional)

Double-click `Snapper Keeper_<version>_x64-setup.exe`. SmartScreen will warn ("Windows protected your PC" → "More info" → "Run anyway"). Installer launches; install to default location. Launch the installed app; verify the library window appears, a screen capture works, OCR returns text (this validates bundled Tesseract). Uninstall via Settings → Apps after verification.

### Step 6: Commit

```bash
git add scripts/build-local.sh
git commit -m "feat(build): add Windows branch to build-local.sh with Tesseract bundling

Resolves the contributor's existing Tesseract install (via
SNK_TESSERACT_PATH, then PATH, then C:\\Program Files\\Tesseract-OCR),
copies the install dir into app/src-tauri/resources/tesseract/, and
installs an EXIT trap that cleans up the copied files (preserving
.placeholder) on script exit.

Mirrors the resolver order snk-ocr/sidecar.rs uses at runtime, so a
contributor who can dev with their local Tesseract can also build an
installer with it."
```

---

## Task 5: Add post-build summary (path + size + SHA-256 + install instructions)

**Rationale:** The script should make it obvious what was produced and how to install it. A summary table with path, size, and SHA-256 also helps you sanity-check produced artifacts against a CI-built reference if you ever need to compare.

**Files:**
- Modify: `scripts/build-local.sh`

### Step 1: Replace the `TODO(task-5)` line

Open `scripts/build-local.sh`. Find:

```bash
# TODO(task-5): post-build summary
```

Replace with:

```bash
# --- Post-build summary ---

# Portable SHA-256: prefer sha256sum (Linux + Git Bash on Windows), fall back
# to shasum -a 256 (macOS). Both emit "<hash>  <path>"; we grab the first field.
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

# Portable byte count via wc -c (BSD + GNU both support reading from stdin
# this way, which avoids the BSD-stat / GNU-stat divergence).
bytes_of() {
  wc -c < "$1" | tr -d '[:space:]'
}

# Format bytes as MB with one decimal place (no external deps).
mb_of() {
  awk -v b="$1" 'BEGIN { printf "%.1f MB", b/1024/1024 }'
}

print_artifact() {
  local label="$1" path="$2"
  if [[ ! -e "$path" ]]; then
    echo "build-local: WARNING: expected artifact not found: $path" >&2
    return
  fi
  local size_bytes size_mb sha
  size_bytes="$(bytes_of "$path")"
  size_mb="$(mb_of "$size_bytes")"
  sha="$(sha256_of "$path")"
  printf '\n%s\n' "$label"
  printf '  Path:   %s\n' "$path"
  printf '  Size:   %s (%s bytes)\n' "$size_mb" "$size_bytes"
  printf '  SHA256: %s\n' "$sha"
}

echo ""
echo "================================================================"
echo "  Build complete (UNSIGNED)"
echo "================================================================"

if [[ "$OS" == "macos" ]]; then
  # .app is a directory; size_of doesn't apply. Print path only for .app;
  # full summary for the .dmg (the actual installable).
  APP_PATH="target/$TARGET/release/bundle/macos/Snapper Keeper.app"
  if [[ -d "$APP_PATH" ]]; then
    echo ""
    echo "App bundle:"
    echo "  Path: $APP_PATH"
  fi
  # Glob the .dmg (filename includes version + arch).
  DMG_PATH=""
  for f in "target/$TARGET/release/bundle/dmg/"*.dmg; do
    [[ -e "$f" ]] && DMG_PATH="$f" && break
  done
  if [[ -n "$DMG_PATH" ]]; then
    print_artifact "Installer (.dmg):" "$DMG_PATH"
  fi
  echo ""
  echo "To install:"
  echo "  - Open the .dmg, drag the .app to /Applications"
  echo "  - First launch: right-click the .app -> 'Open' -> 'Open anyway'"
  echo "  - OR run: xattr -d com.apple.quarantine '/Applications/Snapper Keeper.app'"
fi

if [[ "$OS" == "windows" ]]; then
  EXE_PATH=""
  for f in "target/$TARGET/release/bundle/nsis/"*-setup.exe; do
    [[ -e "$f" ]] && EXE_PATH="$f" && break
  done
  if [[ -n "$EXE_PATH" ]]; then
    print_artifact "Installer (.exe):" "$EXE_PATH"
  fi
  echo ""
  echo "To install:"
  echo "  - Run the .exe"
  echo "  - SmartScreen will warn: click 'More info' -> 'Run anyway'"
fi

echo ""
```

### Step 2: Syntax check

```bash
bash -n scripts/build-local.sh && echo "syntax OK"
```
Expected: `syntax OK`.

### Step 3: Re-run on your current OS

If you completed Task 3 (macOS) or Task 4 (Windows) end-to-end, the artifacts from that run still exist in `target/`. Re-running the script doesn't require rebuilding — actually, the script DOES re-run `tauri build`, but Tauri's incremental build is fast (~30 sec when nothing changed).

```bash
bash scripts/build-local.sh
```

Expected output ends with something like:

On macOS:
```
================================================================
  Build complete (UNSIGNED)
================================================================

App bundle:
  Path: target/aarch64-apple-darwin/release/bundle/macos/Snapper Keeper.app

Installer (.dmg):
  Path:   target/aarch64-apple-darwin/release/bundle/dmg/Snapper Keeper_0.1.2_aarch64.dmg
  Size:   45.3 MB (47497216 bytes)
  SHA256: 1a2b3c4d5e6f...

To install:
  - Open the .dmg, drag the .app to /Applications
  - First launch: right-click the .app -> 'Open' -> 'Open anyway'
  - OR run: xattr -d com.apple.quarantine '/Applications/Snapper Keeper.app'
```

On Windows:
```
================================================================
  Build complete (UNSIGNED)
================================================================

Installer (.exe):
  Path:   target/x86_64-pc-windows-msvc/release/bundle/nsis/Snapper Keeper_0.1.2_x64-setup.exe
  Size:   142.7 MB (149635072 bytes)
  SHA256: 1a2b3c4d5e6f...

To install:
  - Run the .exe
  - SmartScreen will warn: click 'More info' -> 'Run anyway'
```

### Step 4: Commit

```bash
git add scripts/build-local.sh
git commit -m "feat(build): add post-build summary with paths, sizes, SHA-256

Prints a clear summary of produced artifacts and per-OS install
instructions for unsigned builds (SmartScreen on Windows, Gatekeeper
on macOS).

Uses portable shell primitives: sha256sum with shasum fallback for
macOS, wc -c for byte count, awk for MB formatting. No external
deps beyond what Git Bash on Windows / coreutils on macOS ship."
```

---

## Task 6: Wire up `pnpm build:local`

**Rationale:** Make the script discoverable via the standard pnpm entry point.

**Files:**
- Modify: `package.json` (root)

### Step 1: Inspect the current scripts block

```bash
jq '.scripts' package.json
```

Note the current keys so you can add `build:local` in a sensible alphabetical-ish position.

### Step 2: Add the `build:local` script

Open `package.json` (root). Add this line to the `scripts` object (alphabetical order is fine; place near other `build:*` entries if any):

```json
    "build:local": "bash scripts/build-local.sh",
```

Be careful with trailing commas — JSON doesn't allow them on the last key.

### Step 3: Verify JSON is valid

```bash
jq '.' package.json > /dev/null && echo "OK"
```
Expected: `OK`.

### Step 4: Verify pnpm sees the script

```bash
pnpm run
```

Expected: the output lists `build:local` among the available scripts.

### Step 5: Smoke-invoke (optional, redundant with Tasks 3–5)

```bash
pnpm build:local
```

Same end-to-end behavior as `bash scripts/build-local.sh`. Skip if you've already verified.

### Step 6: Commit

```bash
git add package.json
git commit -m "chore(scripts): wire up pnpm build:local

Exposes scripts/build-local.sh as the standard pnpm entry point."
```

---

## Task 7: Update `README.md` — replace "Build a release bundle" section

**Rationale:** The README's current "Build a release bundle" section documents `pnpm --filter @snk/app tauri build` as if it just works — but on Windows it fails without Azure credentials, and on both OSes it fails without `TAURI_SIGNING_PRIVATE_KEY`. Replace with documentation for the actually-working `pnpm build:local` path.

**Files:**
- Modify: `README.md` (lines 75–81)

### Step 1: Find the section

Open `README.md`. The current section reads (around lines 75–81):

```markdown
### Build a release bundle

```bash
pnpm --filter @snk/app tauri build
```

Bundles land in `target/release/bundle/`. See [`docs/release-signing.md`](docs/release-signing.md) for signing setup.
```

### Step 2: Replace it

Replace the whole section with:

````markdown
### Build a local installer (unsigned)

Produce an unsigned installer locally for smoke-testing what end users will receive:

```bash
pnpm build:local
```

On macOS this produces a `.app` + `.dmg` for your machine's architecture; on Windows it produces an NSIS `*-setup.exe` with bundled Tesseract. The artifact path + SHA-256 are printed when the build completes.

**Differences from production:**

- Not Authenticode-signed (Windows) or codesigned + notarized (macOS) — the OS will warn on first launch (see below).
- No updater payload (`.app.tar.gz` + `.sig`) — local builds can't sign the updater manifest.
- Otherwise identical: same target triples, same bundle contents, same Tesseract bundling on Windows.

**Installing an unsigned build:**

- **Windows:** SmartScreen warns; click "More info" → "Run anyway."
- **macOS:** Right-click the `.app` → "Open" → "Open anyway", or run `xattr -d com.apple.quarantine "<path-to-app>"` to clear the Gatekeeper flag.

Linux is not a supported installer target — use `pnpm --filter @snk/app tauri dev` for Linux development.

For signed-release setup, see [`docs/release-signing.md`](docs/release-signing.md).
````

### Step 3: Verify markdown renders

The simplest check: read the section in any markdown renderer (VS Code preview, GitHub blob view). Confirm:
- The code block fences are balanced.
- The link to `docs/release-signing.md` is intact.
- No accidental nested code blocks broke layout.

### Step 4: Commit

```bash
git add README.md
git commit -m "docs(readme): document pnpm build:local for unsigned local installer

Replaces the 'Build a release bundle' section that referenced a
pnpm tauri build invocation that hasn't worked without Azure /
Apple / TAURI_SIGNING_PRIVATE_KEY secrets since the release pipeline
landed. Documents per-OS install steps for unsigned builds."
```

---

## Task 8: Add a note to `docs/release-signing.md`

**Rationale:** Explain to future readers why `app/src-tauri/tauri.conf.json` has no `signCommand` and where production signing actually happens. Without this note, a reader who clones the repo will look at the base config, see no signing config, and wonder if signing is broken.

**Files:**
- Modify: `docs/release-signing.md`

### Step 1: Find the top of the doc

Open `docs/release-signing.md`. The file starts with:

```markdown
# Release Signing Setup

## Ed25519 updater key pair
```

### Step 2: Insert a new section before "Ed25519 updater key pair"

Insert this section between the H1 and the existing first H2:

```markdown
## Where signing lives in the build system

`app/src-tauri/tauri.conf.json` contains no signing commands. Production code signing happens entirely in the release workflow's per-platform sign jobs (`sign-mac-arm`, `sign-mac-x64`, `sign-win-x64`), which download the unsigned artifact and invoke `codesign` / `dotnet sign` directly. This keeps the base config buildable without secrets — `pnpm build:local` (see [`README.md`](../README.md#build-a-local-installer-unsigned)) produces a working unsigned installer on any contributor's machine.

The minisign signature on the updater payload is also added in the sign jobs (using the `TAURI_SIGNING_PRIVATE_KEY` secret), not at build time. The build job sets `createUpdaterArtifacts: false` via inline `--config` overlay; the sign jobs re-create the updater payload (`<app>.tar.gz` on macOS, the signed `.exe` itself on Windows) and run `minisign -S` on it.

```

### Step 3: Verify the markdown links

The new section links to `../README.md#build-a-local-installer-unsigned`. GitHub auto-slugifies "Build a local installer (unsigned)" to `build-a-local-installer-unsigned` (lowercase, spaces → hyphens, parentheses stripped). Open the rendered README on the feature branch and confirm clicking the link from `docs/release-signing.md` lands at the right section.

(If link verification can't be done before merge, leave it — the slug rule is reliable.)

### Step 4: Commit

```bash
git add docs/release-signing.md
git commit -m "docs(release-signing): explain where production signing lives

Adds a 'Where signing lives in the build system' section to head off
the question 'why is there no signCommand in tauri.conf.json?'.
Documents that signing happens in the per-platform sign jobs via
direct codesign / dotnet sign invocations, not via Tauri's hook."
```

---

## Task 9: Pre-merge verification

**Rationale:** Validate that (a) the script works end-to-end on both Windows and macOS, and (b) the CI release pipeline still produces a signed Windows installer after the `signCommand` removal.

This task has no commits — it's a checklist of verifications you perform before merging the feature branch.

### Verification 1: CI green path

After the last code-change commit, push the branch. Confirm `.github/workflows/ci.yml`'s `build-app` job passes on all three OSes (Linux, macOS, Windows). This proves the `tauri.conf.json` change doesn't break compilation.

```bash
git push -u origin <branch>
gh pr create --fill
gh pr checks --watch
```

Expected: all checks green.

### Verification 2: macOS local build (if available)

If you have a macOS machine:

```bash
pnpm build:local
```

Expected:
- `.app` + `.dmg` produced under `target/<triple>/release/bundle/{macos,dmg}/`
- Summary block at end with path/size/SHA-256 for the `.dmg`
- Launching the `.app` from the `.dmg` works (right-click → Open to clear Gatekeeper)
- OCR works (via `brew install`-ed Tesseract)

### Verification 3: Windows local build (interactive desktop)

On your interactive Win11 desktop:

```bash
pnpm build:local
```

Expected:
- `*-setup.exe` produced under `target/x86_64-pc-windows-msvc/release/bundle/nsis/`
- Summary block at end with path/size/SHA-256
- `app/src-tauri/resources/tesseract/` contains only `.placeholder` after script exit
- Running the `.exe` (SmartScreen → "Run anyway") installs the app
- Installed app captures a screen and OCR returns text (validates bundled Tesseract)

Uninstall the test install via Settings → Apps when done.

### Verification 4: Dry-run the release pipeline

Cut a throwaway prerelease tag to validate the production signing path still works after the `signCommand` removal. The `releases/latest/` updater pointer ignores prereleases, so existing clients are unaffected.

```bash
# From the feature branch (after the last commit)
git tag v0.1.3-localbuild-test
git push origin v0.1.3-localbuild-test
```

This triggers `.github/workflows/release.yml`. Approve the `production-release` environment deployment when prompted (per CLAUDE.md instructions):

```bash
RUN_ID=<the run id from gh run list>
pending="$(gh api repos/ehartye/snapper-keeper/actions/runs/$RUN_ID/pending_deployments)"
env_id="$(echo "$pending" | jq -r '.[] | select(.environment.name == "production-release") | .environment.id')"
echo "{\"environment_ids\":[${env_id}],\"state\":\"approved\",\"comment\":\"dry-run for #local-installer-build PR\"}" \
  | gh api -X POST repos/ehartye/snapper-keeper/actions/runs/$RUN_ID/pending_deployments --input -
```

Watch the `sign-win-x64` job's `Authenticode-sign the installer` step. Expected: `Get-AuthenticodeSignature` reports `Valid` with `HartyeTech` as the signer subject. The job succeeds, `publish-release` creates a GitHub Release.

After the run completes successfully, **delete the test release and tag**:

```bash
gh release delete v0.1.3-localbuild-test --yes
git push --delete origin v0.1.3-localbuild-test
git tag -d v0.1.3-localbuild-test
```

If the run fails on the signing step or on JSON parse of `tauri.conf.json`, revert by adding `bundle.windows.signCommand` back to the base config (the original string from the Task 1 diff) — that's the entire rollback.

### Verification 5: Final PR review + merge

When all four verifications above are green:

1. Update the PR description with the dry-run release URL (proves Windows signing still works).
2. Self-review the diff one more time. Confirm no `target/` or `app/src-tauri/resources/tesseract/` files were accidentally committed.
3. Merge using whatever merge strategy this repo uses (squash, rebase, or merge — check existing PR history).

---

## Self-review notes (for the implementer)

When you reach the end of Task 9, before merging, sanity-check:

1. **All spec sections covered:** Per-OS behavior (Task 3 macOS, Task 4 Windows), Tesseract resolver (Task 4), DMG generator choice (Task 3 — uses Tauri's built-in, documented in Task 3 commit message), error model (Task 2 preamble + Task 4 Tesseract-missing exit), post-build summary (Task 5), config/CI refactor (Task 1), docs (Tasks 7 + 8), validation plan (Task 9). ✓
2. **No placeholder commits:** every task has a real code or doc change. ✓
3. **Commit messages are Conventional Commits** with correct scopes (`refactor(release)`, `chore(scripts)`, `feat(build)`, `docs(readme)`, `docs(release-signing)`). ✓
4. **Each commit leaves the repo in a working state:** Task 1 standalone doesn't break CI (proven by `build-app` running on the feature branch). Tasks 2–6 build the script incrementally; the script is only used (via `pnpm build:local`) after Task 6. Tasks 7–8 are doc-only.
