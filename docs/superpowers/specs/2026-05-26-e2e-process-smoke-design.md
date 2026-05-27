# Design: Replace WebDriver E2E smoke with packaged-build process-smoke + plugin IPC tests

**Date:** 2026-05-26
**Status:** Approved (Approach B from brainstorming)
**Issue:** [#47](https://github.com/ehartye/snapper-keeper/issues/47)
**Replaces approach in:** PR [#144](https://github.com/ehartye/snapper-keeper/pull/144)

## Why

PR #144's WebDriver-based smoke (`tauri-driver` + msedgedriver on `windows-latest`) has failed 5 consecutive CI rounds in the same step: **"Install pinned Edge WebDriver matching runner Edge build."** The smoke script itself has never executed. Failures migrated through every Edge-locator strategy attempted (`msedgewebview2.exe` path, `Get-Command msedge.exe`, `${env:ProgramFiles(x86)}` path). The root cause is structural: pinning msedgedriver to the runner's current Edge build at job-start fights against the runner image and creates permanent drift debt.

Hosted macOS runners do not provide a stable WebKit WebDriver path (per the existing comment in `ci.yml`). A symmetric Windows + macOS UI driver story is not available today.

This design replaces WebDriver entirely for the **per-PR required gate**. WebDriver-based UI interactivity testing is not killed for the project — it is removed from the path contributors hit on every push. A future follow-up may add it as an optional `workflow_dispatch` / release-gated job.

## Goals

1. **Catch the named CLAUDE.md gotchas in CI**, on every PR:
   - WebView2 CSP `http://` vs `https://` loopback regression (line 47).
   - Plugin-command "three coordinated entries" ACL gotcha (line 48).
   - Panic-on-startup / failed-init across all plugins.
2. **Zero driver-version drift.** No `tauri-driver`, no msedgedriver, no Edge sniffing.
3. **Cross-OS where it matters.** Packaged-build smoke runs on Windows + macOS hosted runners. Linux stays compile-only (matches the project's audience).
4. **Stable enough for multi-contributor PRs.** Flake is the contributor experience killer; this design eliminates the structural flake sources.

## Non-goals

- **UI interactivity assertions** (clicks, typing, search results rendering). Deliberately out of scope; tracked separately as a future optional/release-gated job.
- **Linux runtime smoke.** Project ships Windows + macOS installers; Linux runtime is dev-only.
- **Replacing or augmenting existing per-plugin unit tests.** This adds an integration layer on top.
- **Performance benchmarking.** Smoke verifies "works," not "fast."

## Architecture

### Layer 1: Plugin IPC + ACL integration tests (`cargo test`, Linux runner)

For each plugin under `crates/snk-*`, add `crates/<plugin>/tests/command_acl_smoke.rs` exercising:

- **Plugin builds with `tauri::test::mock_builder()`** — confirms the plugin's `init()` function returns a `Plugin<MockRuntime>` and that `tauri-build`'s generated permissions load. A plugin that has registered a command in `invoke_handler!` but **not** in `build.rs::COMMANDS` will fail to load cleanly here.
- **Each plugin command is invokable end-to-end** via `tauri::test::get_ipc_response()` (or current equivalent), asserting no "not allowed by ACL" wire error for happy-path calls. Commands that legitimately require runtime-only state (real DB, real screen) can pass minimal mock state or be marked `#[ignore]` with a comment.
- **Re-uses the existing per-plugin `tests/` directory pattern.** Matches the `*_error_wire_shape.rs` convention already in use.

**Where it runs:** the existing `rust-test` job on `ubuntu-latest`. No new infrastructure. The ACL surface is OS-independent — the wiring being correct on Linux means it is correct on Windows and macOS.

**What it cannot catch:** WebView2-specific regressions (CSP, asset-protocol), packaged-bundle layout issues. Those are Layer 2.

### Layer 2: Packaged-build process-smoke (Windows + macOS matrix)

A cross-platform Node harness (`e2e/process-smoke.mjs`) that:

1. **Receives the path** to the already-built **packaged binary** via env var (`SNK_BINARY_PATH`). The CI job builds the binary (release-mode bundle with `pnpm tauri build` or equivalent) in a prior step and passes the path.
2. **Resolves a clean app-data directory** unique to the smoke run (env var `SNK_APP_DATA_DIR` override; falls back to a temp dir under `E2E_ARTIFACT_DIR`). Prevents cross-pollination on parallel/repeated runs.
3. **Launches the binary** as a child process. Captures stdout + stderr to memory + on-disk logs. Sets `SNK_LOG=info,snk=debug` to ensure the steady-state marker is emitted.
4. **Waits for steady-state**, with a 20-second hard cap. Steady state = a log line matching `logging initialized` (existing `logging::init` emits this; see `app/src-tauri/src/logging.rs:113`). If the cap expires without the marker, the smoke fails with reason `"steady-state timeout"`.
5. **Holds the binary running** for an additional 5 seconds to let any deferred plugin setup or window paint complete and emit further log output.
6. **Captures a screenshot** using native OS tooling — `screencapture -x` on macOS, PowerShell `[System.Drawing]`-based capture on Windows. Failure to capture is non-fatal (warning only).
7. **Scrapes the collected stdout/stderr + the app's general log file** for known-bad signatures:
   - CSP violation patterns (`"Content Security Policy"`, `"violated by"`, asset-protocol load failures).
   - `asset.localhost` or `ipc.localhost` load errors that indicate the http/https gotcha regressed.
   - Rust panic markers (`thread '.*' panicked`, `note: run with .RUST_BACKTRACE`).
   - ACL rejections (`"not allowed by ACL"`, `"command .* not found"`).
   - Plugin setup failures (the `plugin:setup-failed` event-emission code path in `main.rs`).
8. **Terminates the binary gracefully** (SIGTERM on macOS, `taskkill /T` on Windows). Falls back to SIGKILL / `taskkill /F` after a 5-second grace period.
9. **Writes a `result.json`** with status (`passed` | `failed`), reason (if failed), and the list of checks performed.
10. **Always uploads artifacts**, success or failure: `result.json`, `process-stdout.log`, `process-stderr.log`, copied app-log directory, screenshot if captured.

**Where it runs:** new `e2e-process-smoke` job, matrix `windows-latest` + `macos-latest`. Depends on a prior packaged-build step that produces the binary.

**Build mode:** Production-bundle equivalent. The CSP gotcha (CLAUDE.md line 47) is **invisible in dev mode** — Vite-served HTML skips CSP enforcement. The smoke must run against a `tauri build` artifact, not a `cargo build` debug binary. The job will reuse the build path used by `scripts/build-local.sh` (NSIS on Windows, `.app` bundle on macOS).

### Failure-mode coverage matrix

| Failure class | Caught by | Notes |
|---|---|---|
| Compile / link breakage | existing `build-app` matrix | Unchanged. |
| Plugin command not registered in `invoke_handler!` | existing `cargo test` IPC mock tests today | Limited. |
| Plugin command registered but missing from `build.rs::COMMANDS` (CLAUDE.md line 48) | **Layer 1** | `mock_builder()` load + invoke. |
| Plugin command missing from `permissions/default.toml` (CLAUDE.md line 48) | **Layer 1** | ACL wire error on invoke. |
| Plugin `init()` panics on a clean profile | **Layer 2** | Caught by `plugin:setup-failed` log scrape. |
| WebView2 CSP regression (CLAUDE.md line 47) | **Layer 2** | Caught by CSP violation log scrape. Cannot be caught in dev mode — requires packaged build. |
| Bundle resource path / icon / metadata issues | **Layer 2** | Binary fails to launch or panics on missing asset. |
| Panic on startup (anywhere) | **Layer 2** | Panic markers + the existing panic hook's crash-dump file are scraped/uploaded. |
| "Search box stopped accepting input" / UI interactivity | **Out of scope** | Future workflow_dispatch UI job. |

## CI topology

```
existing:
  rust-test (ubuntu) ── now also runs Layer 1 (command_acl_smoke per plugin)
  build-app (ubuntu, macos, windows) ── unchanged, compile-only

new:
  e2e-process-smoke (matrix: windows-latest, macos-latest)
    needs: [lint-typecheck, rust-test]
    steps:
      - checkout / pnpm / node / rust / cache
      - install platform deps (none on Windows; nothing extra on macOS)
      - pnpm install --frozen-lockfile
      - pnpm --filter @snk/app build
      - bash scripts/build-local.sh   # produces packaged binary
      - locate built binary, export SNK_BINARY_PATH
      - node e2e/process-smoke.mjs
      - upload e2e-process-smoke-artifacts-{os}

removed (PR #144):
  e2e-windows-smoke job
  e2e/windows-smoke.mjs
  test:e2e:windows script
  README WebDriver smoke section
```

## Artifact strategy

Each matrix arm uploads a uniquely-named artifact:
- `e2e-process-smoke-artifacts-windows-latest`
- `e2e-process-smoke-artifacts-macos-latest`

Contents:
- `result.json` — passed/failed, reason, checks list
- `process-stdout.log` — raw stdout capture
- `process-stderr.log` — raw stderr capture
- `app-logs/` — copy of the app's `app_log_dir` (general + security + crashes/)
- `screenshot.png` — best-effort, native capture

`if-no-files-found: error` for the artifact upload, mirroring PR #144's behavior.

## Replacement scope (what changes in this PR)

**Deleted:**
- `e2e/windows-smoke.mjs`
- `e2e-windows-smoke` job in `.github/workflows/ci.yml`
- `test:e2e:windows` script in `package.json`
- README references to Windows WebDriver smoke

**Added:**
- `e2e/process-smoke.mjs` (cross-platform launcher + scraper)
- `crates/<plugin>/tests/command_acl_smoke.rs` for **each plugin crate that ships a Tauri command surface**. Determined during implementation by grepping each crate for `invoke_handler!` or `#[tauri::command]`. Pure-data crates (no command surface) are skipped — Layer 1 has nothing to assert there.
- `tauri` dev-dependency with the `test` feature added to each plugin crate that gets a Layer 1 test (verify per-crate)
- `e2e-process-smoke` matrix job in `.github/workflows/ci.yml`
- `test:e2e:smoke` script in `package.json`
- README update: replace "Windows WebDriver smoke" section with "process-smoke" description

**Unchanged:**
- All existing per-plugin unit tests
- Existing `rust-test` / `build-app` / `coverage` / `verify-*` jobs
- The `build-local.sh` script (Layer 2 reuses it)

## Risks / open questions

1. **Tauri 2 `tauri::test` API surface stability.** Need to verify `mock_builder()` + `get_ipc_response()` work for each of the 8 plugins. If a specific plugin requires runtime state (e.g. real DB connection for `snk-library`), its smoke test may need to assert on plugin **load** only, not on command invocation. **Mitigation:** validate per-plugin during implementation; if unworkable for a plugin, document the limitation in the test file and assert what we can (plugin loads + ACL config parses).
2. **`scripts/build-local.sh` on hosted runners.** The script currently runs on developer desktops; running it from a GitHub Actions step needs verification (signing env-var gates, code-sign certificate availability, etc.). **Mitigation:** the script supports an unsigned local-build path; CI runs the unsigned path. If that path doesn't exist cleanly, fall back to `pnpm tauri build` directly with a CI-specific config overlay that disables signing.
3. **Native screenshot tooling on hosted runners.** macOS hosted runners are not a "logged-in interactive GUI desktop" — `screencapture` may produce a black image or fail. **Mitigation:** screenshot failure is non-fatal (warning only). If consistently unusable, drop screenshot from macOS arm and document.
4. **Steady-state log marker reliability.** `"logging initialized"` is emitted by `logging::init` in `main.rs`. If `main.rs` is refactored, the marker can silently move. **Mitigation:** add an explicit smoke-targeted marker — emit `tracing::info!(target: "snk::smoke", "app_ready")` after all plugins are set up. The Layer 2 scraper waits for `target=snk::smoke message=app_ready` instead of relying on a logging-init line. (Add this as part of the implementation plan.)
5. **Job runtime budget.** `tauri build` is slower than `cargo build`. Need to verify the matrix job stays under a reasonable timeout (target: 20 minutes per arm; existing PR #144 budget was 12 minutes for the debug-binary path). **Mitigation:** set `timeout-minutes: 25` per arm; revisit if hosted-runner minutes become a concern.
6. **Pnpm + Rust cache warm-up.** Two new matrix arms double the Tauri-build cost compared to PR #144's single Windows arm. Mitigated by `Swatinem/rust-cache` which is already in use; worst case is cold-cache first-run on a contributor PR. Acceptable.

## Decisions log

- **2026-05-26 — Pivot from WebDriver to process-smoke.** PR #144's WebDriver approach hit 5 consecutive structural failures in the Edge-driver pin step. Decision: drop WebDriver from the per-PR gate; replace with Layer 1 (`MockRuntime` ACL tests) + Layer 2 (packaged-binary process-smoke). UI interactivity testing deferred to a future workflow_dispatch / release-gated job.
- **2026-05-26 — Smoke must run against a packaged build, not a debug binary.** PR #144 launched `target/debug/snapper-keeper-app.exe`, which is the wrong target for the CSP gotcha — dev mode skips CSP enforcement entirely. Layer 2 will reuse `scripts/build-local.sh` (or `pnpm tauri build` with a CI overlay).
- **2026-05-26 — macOS + Windows matrix, not Windows-only.** PR #144 was Windows-only on the rationale that hosted macOS WebKit WebDriver is unstable. With WebDriver dropped, the macOS smoke uses only standard process management + native screenshot, both of which work on hosted macOS. Symmetric coverage matches the project's installer-distribution surface.
- **2026-05-26 — Add an explicit `app_ready` log marker.** Relying on a logging-init line couples the smoke to a comment in `main.rs`. Better to emit an explicit `target: "snk::smoke", "app_ready"` event once all plugins finish setup, and have the scraper look for it.
