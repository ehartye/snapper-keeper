# Project guidance for Claude sessions on snapper-keeper

This file gives Claude project-specific context. Read it before you start anything substantive. The user-level `~/.claude/CLAUDE.md` already covers Eric's general working preferences; this file covers what's true *here*.

**Dev environment setup (toolchain versions, Tesseract install, per-OS prerequisites, `pnpm tauri dev`):** [`README.md`](README.md) is canonical. This file documents *project conventions and gotchas* — not setup steps.

## What this repo is

Cross-platform (Windows + macOS) desktop utility — screen capture with annotation + OCR-indexed search, plus clipboard history with a caret-anchored popup. Tauri 2 (Rust + React/TS), local-first, no servers, no telemetry. Audience is "share-friendly side project" — signed installers + auto-update via GitHub Releases, no store distribution.

**Current phase:** Phase 6 complete (library polish). Phases 1–6 all merged to `main`. Next: Phase 7 (signing, notarization, auto-updater, release pipeline). See `docs/superpowers/plans/` for all phase plans.

**Full design:** [`docs/superpowers/specs/2026-05-20-snapper-keeper-design.md`](docs/superpowers/specs/2026-05-20-snapper-keeper-design.md) — read this first if you don't have context. Section 13 has the decisions log; sections 4-10 cover architecture, data model, flows, OS integration, errors, and testing.

## Architecture rules (do not violate)

1. **One Tauri plugin per feature.** Each plugin is its own crate under `crates/`. Plugins ship paired TS bindings under `packages/`.
2. **All persistence flows through `snk-library`.** No other plugin reads or writes DB tables directly. `snk-library` exposes the typed query/mutation API.
3. **No plugin imports another plugin's internals.** Cross-plugin communication is Tauri commands or events.
4. **OCR is fire-and-forget.** Capture emits `capture:saved`; OCR plugin subscribes async. Capture never waits.
5. **Windows are frontend-only.** Plugins are pure Rust contracts. Windows live in `app/src/windows/`.
6. **`snk-clipboard` skips its own writes** so auto-copy from capture doesn't re-trigger the watcher.

Breaking any of these is a red flag — push back before implementing.

## Code conventions

- **Files >500 lines are a red flag** (Eric's standard). If you're approaching that, split the module.
- **Rust style:** workspace inherits edition 2021, Rust 1.78+. `rustfmt.toml` uses stable-only options.
- **TS style:** strict mode + `noUncheckedIndexedAccess`. ESLint v9 flat config (`eslint.config.mjs`). React rules scoped to `**/*.tsx` only (non-React TS packages would warn otherwise).
- **No comments unless the "why" is non-obvious.** Defer to user CLAUDE.md.
- **Errors cross the IPC boundary as typed enums** (`LibraryError`, `CaptureError`, eventually `AppError`). The discriminator tag in serde is `"kind"`, so variant field names must NOT be `kind` — use `reason`, `detail`, `code`, etc.

## Tauri 2 gotchas learned in phase 1

These bit us in phase 1. Avoid repeating.

- **Workspace inheritance on the `tauri` dep blocks `tauri-build` from auto-rewriting features.** Inline the `tauri = { version = "2", features = [...] }` directly in `app/src-tauri/Cargo.toml`, do not use `workspace = true`. Same for `tauri-build`.
- **`tauri dev` always passes `--no-default-features` to cargo.** This is by design; Tauri uses runtime feature detection. Don't try to "fix" it.
- **`tauri-build` regenerates `app/src-tauri/gen/` on every build.** Already gitignored — keep it that way.
- **Windows requires `icons/icon.ico`** in addition to `icon.png`. tauri-build fails on Windows without it.
- **`Emitter` and `Listener` traits must be imported** when calling `.emit()` / `.listen()` / `.listen_any()` on `AppHandle`. Tauri 2 split these out from `Manager`.
- **Cargo workspace requires every declared member to exist on disk.** If you add members to the root `Cargo.toml` for tasks that haven't shipped yet, also commit a placeholder manifest (empty `[package]` + empty `src/lib.rs`).
- **`core:asset:default` is NOT a valid Tauri 2 permission.** The asset protocol is gated entirely by the `protocol-asset` Cargo feature + `assetProtocol` scope in `tauri.conf.json`; there's no separate per-capability permission. The frontend needs `core:path:default` to call `path.appDataDir()`.
- **Windows OpenSSH sessions are non-interactive window stations.** `RegisterHotKey` and `WebView2` will fail with cryptic errors (error 1459, "Invalid window handle") if you try to run `tauri dev` from SSH. Must be an interactive desktop (RDP, console, GUI terminal).
- **Windows UAC installer detection heuristic flags binaries with "update", "setup", or "install" in the filename.** A binary named `snk_updater-*.exe` (or any test/example exe matching this pattern) triggers UAC elevation on launch, which breaks unattended `cargo test` / `cargo run --example` runs. Name test binaries to avoid these substrings (e.g. `updater_smoke` not `snk_updater_test`), or embed a manifest declaring `asInvoker` requestedExecutionLevel.

## Worktree convention

Create worktrees as **sibling** directories: `C:/Users/ehart/repos/snapper-keeper-worktrees/<branch>/`. Not `.worktrees/` inside the repo. This matches Eric's standing preference (confirmed in phase 1 setup; saved in user memory).

## Implementation workflow (for new phases)

Use the h-superpowers skill chain:

1. **brainstorming** — convert idea → design doc in `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`
2. **writing-plans** — convert design → implementation plan in `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`
3. **using-git-worktrees** — create the sibling worktree
4. **team-driven-development** OR **subagent-driven-development** — execute the plan
5. **finishing-a-development-branch** — merge + cleanup

The team-driven model worked well for phase 1 (~30 tasks across 2 implementers + 2 reviewers); subagent-driven is fine for smaller phases.

## Plan-as-source-of-truth pattern

When an implementer finds a real bug in the plan:

1. Implementer reports the bug + proposed fix to team-lead
2. Team-lead approves the fix
3. **Team-lead edits the plan file in place** so the plan stays correct
4. Implementer implements per the (now-corrected) plan
5. Plan-fix and implementation may bundle in one commit, or split (depending on size)

Don't let plan and code drift. The audit trail "plan was fixed because X" is much better than "code diverged from plan."

## Commit conventions

- Conventional Commits: `feat(scope):`, `fix(scope):`, `chore:`, `docs:`, `ci:`, `test:`
- **Commit messages from the plan are exact strings** — implementers use them verbatim. Plan changes propagate to commit messages, not the other way.
- One task = one commit (unless a precursor is justified — workspace stubs, plan-fixes that need to land before the implementation).
- Stage files explicitly (`git add path/to/file`), never `git add .` or `-A`.

## Phase status

| Phase | Scope | Status |
|-------|-------|--------|
| 1 | Foundation + vertical slice (workspace, capture pipeline, 3 plugins) | Done |
| 2 | Region select, window capture, timed capture, post-capture toolbar | Done |
| 3 | Annotation editor + canvas | Done |
| 4 | Clipboard plugin + popup + auto-paste | Done |
| 5 | OCR + FTS5 search | Done |
| 6 | Library polish (sidebar, tags, settings, first-run wizard) | Done |
| 7 | Signing, notarization, auto-updater, release pipeline | Done |

**Known limitation:** Smoke tests on Windows require an interactive desktop session. SSH-only environments can build and lint but can't smoke. CI's `build-app` job verifies the compile across all three OSes; runtime verification is manual.

Each phase gets its own plan document in `docs/superpowers/plans/`.
