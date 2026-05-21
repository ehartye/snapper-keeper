# snapper-keeper

A cross-platform (Windows + macOS) desktop utility that combines screen capture and clipboard history in one app.

> **Status: phase 1 — foundation + vertical slice.** Not yet usable as a daily-driver utility. The current build proves the end-to-end pipeline (hotkey → capture → file write + DB row → thumbnail in a library window), the Tauri 2 plugin architecture, and the CI baseline. Phase 2+ adds the actual features (region select, annotation, clipboard popup, OCR, signed installers, auto-update).

## What it will do (v1 target)

- **Screen capture** — region drag-select, active window, full screen, timed (5s delay), with light annotation (arrow, rectangle, ellipse, freehand, highlighter, text, blur, crop, numbered step markers)
- **OCR-indexed search** over every capture (Tesseract sidecar, async on capture)
- **Clipboard manager** with caret-anchored popup (`Ctrl/Cmd+Shift+V`), text/image/file tracking, sensitive-content filtering, pinning, and auto-paste into the previously-focused app
- **Local-first** — single library directory on disk, SQLite + FTS5 for metadata and search; no servers, no accounts, no telemetry
- **Signed installers** (Apple notarization + Windows code signing) with Ed25519-signed auto-update via GitHub Releases

See [`docs/superpowers/specs/2026-05-20-snapper-keeper-design.md`](docs/superpowers/specs/2026-05-20-snapper-keeper-design.md) for the full design, decisions log, and what's deferred.

## What's in this build (phase 1)

- Cargo + pnpm workspace with one Tauri plugin per feature
- `snk-library` — SQLite + migrations (V001 schema) + Capture model + queries + atomic file write + Tauri plugin (`list_captures`, `get_capture`)
- `snk-hotkeys` — registers `Ctrl/Cmd+Shift+3` and emits an event when pressed
- `snk-capture` — primary-monitor full-screen grab (xcap) + orchestrator that wires grab → file write → library insert
- App shell — tray icon with menu, library window showing a thumbnail grid, three capture entry points (button, hotkey, tray menu)
- CI matrix — lint + typecheck + Rust tests on Linux, build verification on Linux/macOS/Windows

## Quick start

### Prerequisites

- **Rust** 1.78+ via [rustup](https://rustup.rs/)
- **Node.js** 20+ and **pnpm** 9+
- Platform deps from <https://v2.tauri.app/start/prerequisites/>:
  - **Windows:** Microsoft Visual Studio C++ Build Tools, WebView2 (pre-installed on Win 10/11)
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Linux:** `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `libxdo-dev`

### Run in dev

```bash
pnpm install
pnpm --filter @snk/app tauri dev
```

> **Windows note:** The dev session must run from an **interactive desktop session** — not from an SSH terminal. Windows OpenSSH sessions are non-interactive window stations, which cause WebView2 attach failures and `RegisterHotKey` errors. Use RDP, the console, or any GUI terminal inside your interactive desktop.

Expected on first run: Vite spins up on `localhost:5173`, the Rust crates compile (~3–5 min cold, seconds when warm), the library window opens, and a tray icon appears.

Three ways to capture in this phase-1 build:
1. Click **Capture full screen** in the window header
2. Press `Ctrl+Shift+3` (Win) or `Cmd+Shift+3` (Mac)
3. Click the tray icon → **Capture full screen**

Captures land at:
- **Windows:** `%APPDATA%\com.snapper-keeper.app\captures\YYYY\MM\<uuid>.png`
- **macOS:** `~/Library/Application Support/com.snapper-keeper.app/captures/YYYY/MM/<uuid>.png`

### Build a release bundle

```bash
pnpm --filter @snk/app tauri build
```

Bundles land in `target/release/bundle/`. Not yet signed — that's a phase 7 deliverable.

### Lint, typecheck, test

```bash
pnpm lint               # ESLint v9 flat config; max-warnings 0
pnpm typecheck          # tsc --noEmit across all 3 TS packages
cargo test --workspace --exclude snapper-keeper-app  # 10 unit tests in snk-library
cargo fmt --check
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
```

The `snapper-keeper-app` crate is excluded from the Rust test/clippy commands above because it requires the full Tauri build chain (icons, capabilities, etc.); CI's `build-app` job validates it on each OS.

## Architecture

One **Tauri plugin per feature**. Plugins live as separate Rust crates under `crates/`; each ships paired TypeScript bindings under `packages/`. The `app/` shell composes plugins, declares windows, and owns the tray.

```
crates/
  snk-library/      SQLite + migrations + Capture model + queries + Tauri plugin
  snk-hotkeys/      Global hotkey registration + event emission
  snk-capture/      xcap-based grab + orchestrator + Tauri plugin
  snk-annotate/     (phase 3) canvas + tool model + export
  snk-clipboard/    (phase 4) watcher + popup + paste synthesis
  snk-ocr/          (phase 5) Tesseract sidecar + index
  snk-tray/         (later phase) extracted from app/src-tauri
  snk-updater/      (phase 7) signed auto-update
packages/
  snk-library/      TS bindings for snk-library commands + types
  snk-capture/      TS bindings for snk-capture commands + events
  snk-annotate/     (phase 3)
  snk-clipboard/    (phase 4)
  snk-ocr/          (phase 5)
app/
  src/              React + TS + Vite frontend
  src-tauri/        Tauri shell, tray, plugin registration
```

**Architectural rules (load-bearing):**

1. **All persistence flows through `snk-library`.** Plugins never read or write another plugin's tables directly. `snk-library` exposes the typed query/mutation API; everything else is a consumer.
2. **No plugin imports another plugin's internals.** Cross-plugin communication is Tauri commands or events. Forced separation prevents shared-state creep.
3. **OCR is fire-and-forget.** `snk-capture` emits `capture:saved`; `snk-ocr` subscribes and processes asynchronously. Capture never waits on OCR.
4. **Windows are frontend-only.** Plugins are pure Rust contracts. The annotate window and clipboard popup will be frontend artifacts that compose plugin bindings; plugins don't own window lifecycle.
5. **The clipboard plugin skips its own writes.** When `snk-capture` auto-copies, the call routes through `snk-clipboard` so the watcher can tag-and-skip rather than dedup against itself.

See the design doc for full architectural rationale.

## Repository layout

```
app/                      Tauri shell + React frontend
crates/                   Rust plugin crates (one per feature)
packages/                 TS plugin packages (paired with Rust crates)
docs/superpowers/
  specs/                  Design specs (one per phase or major decision)
  plans/                  Implementation plans (one per phase)
.github/workflows/        CI
```

## Development workflow

Phase-scoped: each phase has its own spec + plan + worktree + feature branch + PR. Implementation runs through the [h-superpowers](https://github.com/) plugin ecosystem (brainstorming → writing-plans → team-driven-development → finishing-a-development-branch).

When working on this repo with Claude Code, see [`CLAUDE.md`](CLAUDE.md) for project-specific conventions and gotchas.

## License

MIT OR Apache-2.0 (dual-licensed, no contributions accepted yet — phase 1 is solo).
