# snapper-keeper

A cross-platform (Windows + macOS) desktop utility that combines screen capture with annotation, clipboard history, and OCR-indexed search. Built with Tauri 2 (Rust + React/TypeScript), local-first, no servers, no telemetry.

## Features

### Screen capture
- **Full screen**, **active window**, **region drag-select**, and **timed** (5s delay) modes
- Post-capture toolbar: save, copy, annotate, or discard
- Hotkeys: `Ctrl+Shift+3` / `Cmd+Shift+3` (full screen), plus region, window, and timed variants
- Tray menu access for all capture modes

### Annotation editor
- Tools: arrow, rectangle, ellipse, freehand, highlighter, text, blur/pixelate, crop, numbered step markers
- Color picker, stroke width control, undo/redo
- Save annotated copy alongside the original

### Clipboard history
- Caret-anchored popup via `Ctrl+Shift+V` / `Cmd+Shift+V`
- Tracks text and image clipboard entries
- Filter, pin favorites, keyboard navigation, auto-paste into the previously focused app
- Content-hash deduplication, configurable eviction limit

### OCR + search
- Native OS OCR (Apple Vision on macOS, Windows.Media.Ocr on Windows) runs asynchronously on every capture
- FTS5 full-text search across OCR text, clipboard content, and tag names
- Search bar in the library window with debounced queries

### Library
- Thumbnail grid with smart sections (Today, Yesterday, This Week, Older)
- Sidebar with tag filtering and clipboard history view
- Soft-delete with trash, pinning, tag management (create, assign, color-code)
- Settings window for capture, clipboard, and OCR configuration
- First-run wizard

### Auto-updater
- Ed25519-signed update manifests via GitHub Releases
- Checks on launch (5s delay) + every 24 hours
- Tray menu "Check for updates" item
- Download + prompt to restart (never auto-applied)

### Release pipeline
- GitHub Actions workflow triggered on `v*` tags
- Build matrix: macOS (aarch64 + x86_64) + Windows (x86_64)
- Apple code signing + notarization, Windows code signing
- Generates `latest.json` manifest for the auto-updater

## Quick start

### Prerequisites

- **Rust** 1.78+ via [rustup](https://rustup.rs/)
- **Node.js** 20+ and **pnpm** 9+
- Platform deps from <https://v2.tauri.app/start/prerequisites/>:
  - **Windows:** Microsoft Visual Studio C++ Build Tools, WebView2 (pre-installed on Win 10/11)
  - **macOS:** 14.0+ (Sonoma) — Apple Vision OCR requires it. Xcode Command Line Tools (`xcode-select --install`)
  - **Linux:** `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `libssl-dev`. Linux is supported as a **dev convenience only** — the project ships signed installers for Windows and macOS only, and CI does not run a Linux release pipeline.

### Run in dev

```bash
pnpm install
pnpm --filter @snk/app tauri dev
```

Vite starts on `localhost:5173`, the Rust crates compile (~3-5 min cold, seconds warm), the library window opens, and a tray icon appears.

> **Windows note:** Must run from an **interactive desktop session** (not SSH). Windows OpenSSH sessions are non-interactive window stations, causing WebView2 and `RegisterHotKey` failures.

### Build a local installer (unsigned)

Produce an unsigned installer locally for smoke-testing what end users will receive:

```bash
pnpm build:local
```

> **Windows users:** Run from **Git Bash** (or any bash shell) — the underlying script is bash; PowerShell and `cmd.exe` will fail to invoke it. Git Bash ships with [Git for Windows](https://git-scm.com/download/win).

On macOS this produces a `.app` + `.dmg` for your machine's architecture; on Windows it produces an NSIS `*-setup.exe`. The artifact path + SHA-256 are printed when the build completes.

**Differences from production:**

- Not Authenticode-signed (Windows) or codesigned + notarized (macOS) — the OS will warn on first launch (see below).
- No updater payload (`.app.tar.gz` + `.sig`) — local builds can't sign the updater manifest.
- Otherwise identical: same target triples, same bundle contents.

**Installing an unsigned build:**

- **Windows:** SmartScreen warns; click "More info" → "Run anyway."
- **macOS:** Right-click the `.app` → "Open" → "Open anyway", or run `xattr -d com.apple.quarantine "<path-to-app>"` to clear the Gatekeeper flag.

Linux is not a supported installer target — use `pnpm --filter @snk/app tauri dev` for Linux development.

For signed-release setup, see [`docs/release-signing.md`](docs/release-signing.md).

## Local testing

### Full test suite

```bash
# TypeScript
pnpm lint               # ESLint v9 flat config, max-warnings 0, 7 packages
pnpm typecheck          # tsc --noEmit across all 7 TS packages

# Rust (65 unit tests + 2 integration tests)
cargo test --workspace --exclude snapper-keeper-app --exclude snk-updater
cargo fmt -- --check
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings

# App compilation (requires full Tauri build chain)
cargo build -p snapper-keeper-app
```

### Why some crates are excluded

- **`snapper-keeper-app`** — requires the full Tauri build chain (icons, capabilities, codegen). CI's `build-app` job validates it on each OS.
- **`snk-updater`** — on Windows, the test binary triggers UAC elevation (error 740) because Windows detects "update" in the filename. The 8 unit tests pass on macOS/Linux and in CI. On Windows, set `__COMPAT_LAYER=RunAsInvoker` in your shell env *before* running `cargo test -p snk-updater` to suppress the UAC prompt.

### What to test manually

Automated tests cover data layer logic and serde contracts. UI and OS integration require manual verification on an interactive desktop:

- Capture modes (full screen, region, window, timed) produce thumbnails in the library
- Annotation editor opens from the post-capture toolbar, saves annotated copy
- Clipboard popup appears at the caret on `Ctrl+Shift+V` / `Cmd+Shift+V`
- OCR text appears in search results after a few seconds
- Tags can be created, assigned to captures, and filtered in the sidebar
- Tray menu items work (all capture modes, settings, check for updates, quit)
- Settings window persists changes across restart

## Architecture

One **Tauri plugin per feature**. Plugins are Rust crates under `crates/`, each with paired TypeScript bindings under `packages/`. The `app/` shell composes plugins, declares windows, and owns the tray.

```
crates/
  snk-library/      SQLite + migrations + models + queries + Tauri commands
  snk-hotkeys/      Global hotkey registration + event emission
  snk-capture/      xcap grabs + orchestrator (region, window, timed, fullscreen)
  snk-annotate/     Annotation save/export Tauri commands
  snk-clipboard/    Clipboard watcher + paste synthesis + caret detection
  snk-ocr/          Native OCR backends (Vision / Windows.Media.Ocr) + async queue
  snk-updater/      Ed25519-signed auto-update via tauri-plugin-updater
packages/
  snk-library/      TS bindings: captures, tags, settings, search
  snk-capture/      TS bindings: capture modes, window listing
  snk-annotate/     TS bindings: save annotation
  snk-clipboard/    TS bindings: clipboard list, paste, pin
  snk-ocr/          TS bindings: OCR trigger
  snk-updater/      TS bindings: check for update, get status
app/
  src/              React + TypeScript + Vite frontend
  src-tauri/        Tauri shell, tray, plugin registration, capabilities
```

### Architectural rules

1. **All persistence flows through `snk-library`.** No other plugin reads or writes DB tables directly.
2. **No plugin imports another plugin's internals.** Cross-plugin communication uses Tauri commands or events.
3. **OCR is fire-and-forget.** Capture emits `capture:saved`; OCR subscribes async. Capture never waits.
4. **Windows are frontend-only.** Plugins are pure Rust contracts; windows live in `app/src/windows/`.
5. **The clipboard plugin skips its own writes** so auto-copy from capture doesn't re-trigger the watcher.

### Data storage

All data lives in the OS app-data directory:
- **Windows:** `%APPDATA%\com.snapper-keeper.app\`
- **macOS:** `~/Library/Application Support/com.snapper-keeper.app/`

SQLite database with WAL mode, 3 migrations (captures + FTS, clipboard, OCR). Capture images stored as `captures/YYYY/MM/<uuid>.png` alongside annotated copies.

## CI

Two workflows:

- **CI** (`.github/workflows/ci.yml`) — runs on push to `main` and all PRs. Lint + typecheck + Rust tests on Linux, app build verification on Linux/macOS/Windows, plus an `e2e-process-smoke` matrix job on Windows + macOS that runs the packaged binary and scrapes for known-bad signatures (see [design doc](docs/superpowers/specs/2026-05-26-e2e-process-smoke-design.md)).
- **Release** (`.github/workflows/release.yml`) — runs on `v*` tags. Builds signed bundles for macOS (aarch64 + x86_64) and Windows (x86_64), notarizes macOS builds, generates `latest.json` update manifest, publishes to GitHub Releases.

### E2E process-smoke

The `e2e-process-smoke` CI job runs on every PR against `windows-latest` and `macos-latest`. It builds an unsigned packaged binary via `scripts/build-local.sh`, launches it, waits for the `snk::smoke` `app_ready` log marker (emitted from `main.rs` at the end of `setup()`), then scrapes stdout/stderr/app-log for known-bad signatures: CSP violations, asset-protocol load failures, Rust panics, Tauri ACL rejections, plugin setup failures. Per-OS artifact bundles upload on every run (success or failure) with `process-stdout.log`, `process-stderr.log`, `screenshot.png`, copied `app-logs/`, and `result.json`.

Layer 1 of the same strategy runs in the `rust-test` job: each plugin crate has a `tests/command_acl_smoke.rs` asserting the plugin's `init` function exists with the expected `fn() -> TauriPlugin<R>` signature — a compile-time API-surface check.

UI interactivity (clicks, typing, search results) is intentionally out of scope for the per-PR gate.

## Releasing

1. Generate an Ed25519 keypair and configure GitHub Actions secrets (see [`docs/release-signing.md`](docs/release-signing.md))
2. Set the public key in `app/src-tauri/tauri.conf.json` under `plugins.updater.pubkey`
3. Configure Apple and Windows signing certificates as repo secrets
4. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`

The release workflow builds, signs, notarizes, and publishes automatically.

## Development workflow

Phase-scoped: each phase has its own spec + plan + worktree + feature branch. Implementation plans live in `docs/superpowers/plans/`, the design spec in `docs/superpowers/specs/`.

When working on this repo with Claude Code, see [`CLAUDE.md`](CLAUDE.md) for project-specific conventions and gotchas.

## License

MIT OR Apache-2.0 (dual-licensed).
