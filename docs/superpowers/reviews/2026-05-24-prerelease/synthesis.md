# snapper-keeper Pre-Release Review — Synthesis

Compiled from 4 perspectives × 2 rounds (Adversary, Operator, Maintainer, Testing Strategy). Round 1 = independent findings; Round 2 = cross-pollination.

---

## Executive Summary

**Recommendation: hold v1.0 against the GitHub Releases endpoint.** Three independent perspectives (Adversary, Operator, Maintainer) and the Testing-Strategy lens converge on the same structural issue — the codebase ships a series of features documented in `PRIVACY.md`, the design spec, and the README that are not implemented in code (sensitive clipboard exclusion, updater opt-out, Microsoft Store edition, configurable eviction, pre-migration backups, per-window capability isolation, file logging). This "designed but not implemented" pattern is itself the headline vulnerability: every external due-diligence read forms a security/privacy model against features that don't exist.

**Top 3 release blockers** (each flagged independently by 2+ perspectives):
1. **Stored XSS via FTS snippet → full IPC blast radius** (Adversary F1; amplified by Operator R-Adv F1, Testing R1, Maintainer R3) — `dangerouslySetInnerHTML` over un-sanitized OCR/clipboard/window-title text, with `csp: null` and uniform `default.json` capability.
2. **Clipboard watcher captures secrets with no exclusions** (Adversary F2; Testing F4; Operator R-Adv F2; Maintainer F4 derivative) — `sensitive` column dead, `clipboard.app_blocklist` setting dead, password-manager copies stored plaintext.
3. **PRIVACY.md / spec / README make material claims the binary cannot keep** (Adversary F5; Operator #2; Testing R2; Maintainer F4/F5) — "disable update checks in Settings", "Microsoft Store edition compiles out updater", "configurable eviction", "macOS universal binary", "pre-migration backup", "sensitive items not stored". Edit or implement before tagging.

**Top 3 post-release follow-ups** (high value but not immediate blockers):
1. **Split capabilities per window** (Adversary F6/F9; Operator #3/#15; Maintainer N5) — highest-leverage single defense; ~half-day; design doc already specifies it.
2. **File-based logging + panic hook + separate `security-events.log`** (Operator #1; Adversary R-Op #1; Maintainer R2; Testing R7) — without it, post-incident forensics is impossible.
3. **Fixture-based migration forward-compatibility test + pre-migration backup** (Operator #5/NI1; Testing F7/R9; Maintainer F8) — single most likely operational disaster; prevents the v1.x upgrade-bricks-library scenario.

---

## Independent Findings (Round 1)

### Release-Blocker Consensus (2+ perspectives, ship-blocking)

#### CB1 — Stored XSS via FTS snippet rendered through `dangerouslySetInnerHTML` with CSP disabled
- **Perspectives:** Adversary F1; corroborated by Testing R1 (test gap), Operator R-Adv F1 (forensics gap), Maintainer R3 (blast-radius framing)
- **Where:** `app/src/windows/library/SearchBar.tsx:173`; `crates/snk-library/src/search.rs:86,104`; `app/src-tauri/tauri.conf.json:85` (`csp: null`)
- **Severity:** CRITICAL
- **Suggested fix:** Render snippet as React elements (split on literal `<mark>`/`</mark>`, never `dangerouslySetInnerHTML`) AND set a real CSP.
- **Issue title:** `[security] Stored XSS in FTS snippet rendering — sanitize + enable CSP`

#### CB2 — Clipboard watcher stores everything, no exclusions, dead `sensitive` flag
- **Perspectives:** Adversary F2; Testing F4; Maintainer F4 (derivative — README's "configurable eviction limit"); Operator R-Adv F2
- **Where:** `crates/snk-clipboard/src/watcher.rs:54-136`; `crates/snk-library/migrations/V002__clipboard_items.sql:11`; `crates/snk-library/src/settings.rs:86` (orphaned `app_blocklist` test fixture)
- **Severity:** CRITICAL
- **Suggested fix:** Honor Windows `ExcludeClipboardContentFromMonitors` / `CanIncludeInClipboardHistory` and macOS `org.nspasteboard.ConcealedType`; implement `clipboard.app_blocklist` with default password-manager list; or strip the schema/setting/spec claim and document plaintext storage.
- **Issue title:** `[privacy] Implement (or strip) sensitive-clipboard exclusion before v1`

#### CB3 — PRIVACY.md / spec promises not honored by code
- **Perspectives:** Adversary F5; Operator #2; Testing R2; Maintainer F4/F5/F12 (doc drift pattern)
- **Where:** `PRIVACY.md:25-29`; `app/src/windows/settings/SettingsWindow.tsx` (no updater toggle row); `crates/snk-updater/src/plugin.rs:142-160` (unconditional); no Microsoft Store build variant
- **Severity:** CRITICAL (regulatory/reputational)
- **Suggested fix:** Either implement the `updater.enabled` setting + check, or edit `PRIVACY.md` to remove both fictional sentences before tagging.
- **Issue title:** `[privacy] Reconcile PRIVACY.md updater-disable and Microsoft Store claims`

#### CB4 — Uniform capability grants every window full IPC blast radius
- **Perspectives:** Adversary F6/F9 + F14; Operator #3/#15; Maintainer R4/N5
- **Where:** `app/src-tauri/capabilities/default.json:5-28` (single capability for all 6 windows); design `§8.3` lines 557-565 specified `clipboard-popup.json` separately, file doesn't exist
- **Severity:** HIGH (CRITICAL when combined with CB1)
- **Suggested fix:** Split into per-window capability files per the design (`library.json`, `capture-overlay.json`, `clipboard-popup.json`, `annotate.json`, `settings.json`).
- **Issue title:** `[security] Split Tauri capabilities per window per design spec §8.3`

#### CB5 — No file-based logging; stdout discarded on packaged Windows builds
- **Perspectives:** Operator #1; Adversary R-Op #1 (forensics); Testing R7 (post-release feedback loop); Maintainer R2 (amplifies F7/F8 invisibly)
- **Where:** `app/src-tauri/src/main.rs:67-71` (only `tracing_subscriber::fmt()` → stdout); `main.rs:1` (`windows_subsystem = "windows"`); no `tracing-appender`; no panic hook anywhere
- **Severity:** HIGH (blocks post-release diagnosis of every other finding)
- **Suggested fix:** Add `tracing-appender` with daily rotation under `app.path().app_log_dir()`; add `std::panic::set_hook` writing to `crashes/` subfolder; surface "Open log folder" in Settings.
- **Issue title:** `[ops] Add file-based logging with daily rotation + panic hook`

#### CB6 — Pre-migration backup promised, not implemented; `recoverable` flag is a lie
- **Perspectives:** Operator #5; Testing F7 (no forward-compat tests); Maintainer F8 (hardcoded `from:0 to:4`); Adversary R-Op F5 (malicious migration vector)
- **Where:** `crates/snk-library/src/migrate.rs:15-23` (string-matches "Backup" which never appears); no `backups/` directory; `crates/snk-library/src/plugin.rs:42-48` (no recovery path)
- **Severity:** HIGH (highest-EV operational disaster per Operator NI1)
- **Suggested fix:** Copy DB file to `backups/pre-vN-<ts>.db` after `wal_checkpoint(TRUNCATE)` before migrations; restore latest on failure; replace hardcoded `to: 4` with `migrations().current_version()`; add `tests/migration_forward_compat.rs` with realistic fixture data.
- **Issue title:** `[reliability] Pre-migration backup + forward-compatibility test`

---

### Cross-Perspective Consensus (non-blocker, multi-perspective)

#### CP1 — Auto-updater has no signature-error distinction, no rollback floor, no kill switch
- **Perspectives:** Adversary F7/F8; Operator #6/#7; Testing F2/R3
- **Where:** `crates/snk-updater/src/plugin.rs:63-134`; `app/src-tauri/tauri.conf.json:110-112`; `.github/workflows/release.yml:259-306` (`latest.json` itself unsigned)
- **Severity:** HIGH
- **Suggested fix:** Sign `latest.json` itself; store highest-ever-seen version locally and refuse strict downgrades; distinguish signature errors as terminal (disable for process lifetime); document kill-switch protocol.
- **Issue title:** `[updater] Sign latest.json + downgrade floor + signature-error terminal handling`

#### CP2 — macOS OCR silently broken (Tesseract not bundled)
- **Perspectives:** Operator #4; Maintainer F5; Testing R4
- **Where:** `.github/workflows/release.yml:66-76` (Windows-only); `crates/snk-ocr/src/sidecar.rs:80-86` (macOS fallbacks all require brew/macports); `README.md:24-27,56` (silent on Mac OCR caveat)
- **Severity:** HIGH (silent feature failure on shipped platform)
- **Suggested fix:** Bundle Tesseract on macOS in release workflow (with hash-pinned source per CP3) OR surface a first-run banner when `resolve_tesseract()` returns `None` OR adopt Apple Vision OCR on macOS.
- **Issue title:** `[ocr] Bundle Tesseract for macOS or surface missing-dependency banner`

#### CP3 — CI supply-chain: actions pinned by mutable tag, choco-tesseract unverified, dotnet-sign `--prerelease`
- **Perspectives:** Adversary F10/F11; Operator R-Adv F11
- **Where:** `.github/workflows/release.yml:66-76,90,308-310`; `.github/workflows/ci.yml`
- **Severity:** HIGH
- **Suggested fix:** Pin all GitHub Actions by full commit SHA; drop `--prerelease` from `dotnet tool install sign`; pin choco Tesseract version + SHA256-verify; replace `softprops/action-gh-release` with `gh release create` in an inline script; split build/sign from publish with `environment: production-release` approval gate.
- **Issue title:** `[ci] Pin CI actions by SHA + pin/verify Tesseract chocolatey source`

#### CP4 — Test coverage gate is misleading (excludes IPC perimeter)
- **Perspectives:** Testing F13; Operator R-Test F13; Adversary R-Test F13
- **Where:** `.github/workflows/ci.yml:82-85` (regex excludes `plugin.rs|commands.rs|caret.rs|paste.rs|watcher.rs|queue.rs|paste.rs|...`)
- **Severity:** MEDIUM
- **Suggested fix:** Either drop the threshold gate or split into "logic coverage (gated 90%)" and "IPC surface coverage (reported, not gated, with improvement target)."
- **Issue title:** `[ci] Split coverage reporting between pure-logic and IPC surfaces`

#### CP5 — Clipboard watcher silent thread-death on `Clipboard::new()` failure
- **Perspectives:** Operator #10; Testing F3/F14
- **Where:** `crates/snk-clipboard/src/watcher.rs:22-30`
- **Severity:** MEDIUM-HIGH
- **Suggested fix:** Retry with exponential backoff (cap 60s) + emit `clipboard:unavailable` event so popup can show offline banner; add a `clipboard_status` command.
- **Issue title:** `[clipboard] Retry watcher init + expose health event for offline state`

#### CP6 — Dead `@snk/ocr` and `@snk/updater` TS packages
- **Perspectives:** Maintainer F1; Testing R6 (misleading coverage); Adversary R-M F1 (supply-chain surface); Operator R-M F1 (repurpose for About panel)
- **Where:** `packages/snk-ocr/`, `packages/snk-updater/`; `app/vitest.config.ts:41-42`
- **Severity:** MEDIUM
- **Suggested fix:** Either delete (Maintainer/Adversary preference) or consume in a new Settings → About diagnostic panel (Operator preference). Either way, end the current "ships but unused" state.
- **Issue title:** `[cleanup] Delete or wire dead @snk/ocr and @snk/updater TS packages`

#### CP7 — No "About" panel / no in-app version surface
- **Perspectives:** Operator #13; Maintainer F1 (Operator-amplified); Testing R7 derivative
- **Where:** `app/src/windows/settings/SettingsWindow.tsx`; `app/src-tauri/src/main.rs:143-157` (tray menu)
- **Severity:** MEDIUM
- **Suggested fix:** Add About section to Settings showing: app version, data dir + "Open", log dir + "Open", updater status, last check timestamp.
- **Issue title:** `[ux] Add Settings → About panel with version + paths + updater status`

#### CP8 — No E2E layer in CI (design §10.2 calls for `tauri-driver`)
- **Perspectives:** Testing F1; Operator R-Test F1; Maintainer T1 (reconcile with Windows-interactive constraint)
- **Where:** Absent. `.github/workflows/ci.yml:87-112` compile-only; `release.yml` has no smoke between build and upload.
- **Severity:** MEDIUM (acknowledged constraint; resolution requires testing whether windows-latest runners are interactive enough)
- **Suggested fix:** Test whether windows-latest supports basic capture-and-list smoke; macOS gets full E2E; Windows gets minimum "binary starts, library window paints, no panic"; produce CI artifact (log + screenshot) per run.
- **Issue title:** `[ci] Add minimal E2E smoke per OS with uploaded runtime artifact`

#### CP9 — Cross-plugin `LibraryState` imports via `::plugin::` internal path
- **Perspectives:** Maintainer F2; Testing R5 (no enforcement); Adversary R-M F2 (refactor-blocking)
- **Where:** `crates/snk-annotate/src/commands.rs:3`; `crates/snk-clipboard/src/commands.rs:9`; `crates/snk-clipboard/src/plugin.rs:13`; `crates/snk-ocr/src/plugin.rs:6`; clean re-export at `crates/snk-library/src/lib.rs:21`
- **Severity:** LOW-MEDIUM
- **Suggested fix:** Mechanical sweep to crate-root re-export; add CI grep script (`grep -rn 'snk_library::plugin::' crates/` must be empty).
- **Issue title:** `[refactor] Use snk_library::LibraryState re-export + lint for ::plugin:: reach-ins`

#### CP10 — Typed-error contract violated; TS side has zero error types
- **Perspectives:** Maintainer F3; Testing F12; Adversary R-M F3 (subtle)
- **Where:** `crates/snk-ocr/src/plugin.rs:15` (`Result<String, String>`); `crates/snk-updater/src/plugin.rs:47,52`; `crates/snk-capture/src/commands.rs:64-85`; `packages/*/src/types.ts` (no Error types)
- **Severity:** MEDIUM
- **Suggested fix:** Adopt `ts-rs` or `specta` to generate TS types from Rust error enums; either add `OcrError`/`UpdaterError` enums or formally allow `Result<_, String>` for status-only commands with a comment.
- **Issue title:** `[ipc] Generate TS error types from Rust + enforce typed-error rule`

#### CP11 — `mem::forget(dir)` leaks tempdirs across 5+ test helpers
- **Perspectives:** Maintainer F10; Testing F9
- **Where:** `crates/snk-library/src/captures.rs:332-337`; `clipboard.rs:252-256`; `search.rs:171-176`; `settings.rs:52-57`; `crates/snk-capture/src/orchestrate.rs:112-117`; `ocr.rs:66-70`; `tags.rs:173-177`
- **Severity:** LOW
- **Suggested fix:** Extract a shared `fn fresh_db() -> (TempDir, Db)` helper; return both so Drop runs at end of test.
- **Issue title:** `[tests] Shared test_support fresh_db helper to remove tempdir leaks`

---

### Unique Findings — Adversary

- **A-U1 — Asset-protocol scope wide open under `$APPDATA/**`** (F3) — `app/src-tauri/tauri.conf.json:86-91`. Lets webview read entire SQLite + WAL via `asset:` if XSS lands. SEVERITY: HIGH. Issue: `[security] Tighten assetProtocol scope to captures/ and clipboard/ subdirs only`.
- **A-U2 — `csp: null` provides zero defense-in-depth** (F4) — `tauri.conf.json:85`. SEVERITY: HIGH. Issue: `[security] Set a real Content-Security-Policy in tauri.conf.json`.
- **A-U3 — Tesseract sidecar runs unsandboxed with full app token** (F12) — `crates/snk-ocr/src/sidecar.rs:151-199`. SEVERITY: MEDIUM. Issue: `[security] Sandbox Tesseract sidecar (job object / sandbox-exec) + per-invocation timeout`.
- **A-U4 — Image clipboard / capture stores grow unbounded; no encryption option** (F13) — no cap on `captures` table; no SQLCipher; no retention default. SEVERITY: MEDIUM. Issue: `[privacy] Document retention defaults + offer SQLCipher opt-in for local data at rest`.
- **A-U5 — `_app: AppHandle` unused in 14+ commands; no per-window authorization** (F14) — `crates/snk-library/src/commands.rs:11-186`; `crates/snk-clipboard/src/commands.rs:21-53`. SEVERITY: MEDIUM. Issue: `[security] Add per-window authorization middleware for destructive commands`.
- **A-U6 — `save_annotation`/`derive_capture` accept any bytes as PNG; no validation** (F15) — `crates/snk-annotate/src/commands.rs:8-37,39-79`. SEVERITY: LOW-MEDIUM. Issue: `[validation] Validate PNG magic bytes + state_json schema + max payload size`.
- **A-U7 — `SNK_LOG` env-var could log clipboard/OCR text to user-redirected sink** (F16) — `app/src-tauri/src/main.rs:67-71`. SEVERITY: LOW. Issue: `[logging] Redacted<T> newtype for sensitive fields in tracing output`.
- **A-U8 — Release workflow build step has full secrets in env; any transitive `build.rs` can exfil** (Round 2 N6) — `.github/workflows/release.yml:172-183`. SEVERITY: HIGH (compounds with CP3). Issue: `[ci] Split build job (no secrets) from sign+publish job (minimum-needed secrets)`.

### Unique Findings — Operator

- **O-U1 — Updater scheduler double-tick semantics + default `MissedTickBehavior::Burst`** (#8) — `crates/snk-updater/src/plugin.rs:144-160`. SEVERITY: LOW-MEDIUM. Issue: `[updater] Set MissedTickBehavior::Delay + collapse redundant first tick`.
- **O-U2 — OCR queue unbounded; quit-mid-queue captures left without OCR; no resumption** (#9) — `crates/snk-ocr/src/queue.rs:21,41-110`. SEVERITY: MEDIUM. Issue: `[ocr] Bounded queue + startup sweep for captures missing ocr_text`.
- **O-U3 — Nightly `cargo audit` / `pnpm audit` + SBOM publishing missing** (#11) — design §10.4 promised. SEVERITY: MEDIUM. Issue: `[ci] Add nightly cargo-audit + pnpm-audit + per-release SBOM (cyclonedx)`.
- **O-U4 — Release workflow signs `cmd.exe` as smoke test** (#12) — `.github/workflows/release.yml:118-142`. SEVERITY: LOW. Issue: `[ci] Replace cmd.exe smoke with vendored test artifact + verify roundtrip`.
- **O-U5 — macOS released as separate per-arch bundles; design promised universal binary** (#14) — `.github/workflows/release.yml:21-26`. SEVERITY: MEDIUM. Issue: `[release] Build macOS universal binary (--target universal-apple-darwin)`.
- **O-U6 — Updater endpoint dependent on `/releases/latest/` semantics; first-tag-must-be-stable trap** (#6) — `app/src-tauri/tauri.conf.json:110-112`. SEVERITY: MEDIUM. Issue: `[release] Document first-tag-must-be-plain-SemVer + add fallback endpoint`.
- **O-U7 — Add separate never-rotated `security-events.log`** (Round 2 NI3) — companion to CB5. SEVERITY: MEDIUM. Issue: `[ops] Add security-events.log (sig failures, capability denials, paste_item, autostart toggles, panic captures)`.
- **O-U8 — IPC audit middleware for destructive commands** (Round 2 NI2/NI5) — uses the `_app` parameter from A-U5. SEVERITY: MEDIUM. Issue: `[ops] IPC audit log for paste_item + destructive commands with source window label`.
- **O-U9 — First-release canary deployment** (Round 2 NI4) — tag v0.1.0 as `prerelease: true` first; promote after 1-2 weeks. SEVERITY: LOW (procedural). Issue: `[release] Canary procedure for first user-facing tag`.

### Unique Findings — Maintainer

- **M-U1 — `AnnotateCanvas.tsx` exceeds 500-line CLAUDE.md ceiling (523 LoC)** (F6) — `app/src/windows/annotate/AnnotateCanvas.tsx`. SEVERITY: LOW. Issue: `[refactor] Split AnnotateCanvas.tsx into shape-specific modules under shapes/`.
- **M-U2 — `LibraryError::From<io::Error>` discards path + uses kind string** (F7) — `crates/snk-library/src/error.rs:39-46`. SEVERITY: MEDIUM (compounds with CB5). Issue: `[errors] LibraryError::Io should preserve OS message; consider removing From<io::Error>`.
- **M-U3 — Theme registration fanned out across 5 places (Rust + TS)** (F9) — `app/src-tauri/src/main.rs:12-37`; `app/src/lib/theme.ts:9-31`. SEVERITY: LOW. Issue: `[refactor] Load tray icons from disk OR add CI script verifying THEMES keys ↔ Rust constants`.
- **M-U4 — `snk-hotkeys` carries unused `thiserror` dep + dead "later phase" comment** (F11) — `crates/snk-hotkeys/Cargo.toml:13`. SEVERITY: LOW. Issue: `[cleanup] Drop unused thiserror dep + resolve "later phase" comment in snk-hotkeys`.
- **M-U5 — Linux in CI/README but excluded from spec; drift not declared** (F12) — `README.md:53-62`; `docs/superpowers/specs/...:9`. SEVERITY: LOW. Issue: `[docs] Declare Linux as dev-only; add "Builds on Linux for dev convenience" note`.
- **M-U6 — Global `SKIP_NEXT AtomicBool` synchronization in clipboard watcher** (F13) — `crates/snk-clipboard/src/watcher.rs:16-20`. Testing NI6 amplifies. SEVERITY: MEDIUM. Issue: `[clipboard] Replace SKIP_NEXT AtomicBool with Mutex<HashSet<hash>> for testable skip semantics`.
- **M-U7 — README local-test instructions drift from CLAUDE.md** (F14) — duplicated content with platform tips in only one. SEVERITY: LOW. Issue: `[docs] Consolidate dev-setup; CLAUDE.md and README cross-link to single source`.
- **M-U8 — UUIDv7 monotonicity is an implicit invariant across 4 call sites** (Round 2 N2). SEVERITY: LOW. Issue: `[docs] Document UUIDv7 invariant (monotonicity + no HTML/JSON specials) in snk-library/src/lib.rs`.
- **M-U9 — Plugin `setup()` panics are unhandled; silent app death** (via Testing NI4 + Operator overlap; surfaced in Maintainer review). SEVERITY: MEDIUM. Issue: `[reliability] Panic boundary around plugin setup() with user-facing error`.
- **M-U10 — Updater `UpdateStatus` has no `RejectedBySignature`/`SuppressedByPolicy` variants** (Round 2 N4). SEVERITY: MEDIUM. Issue: `[updater] Design unified state machine for signature-rejection and policy-suppression before implementing`.

### Unique Findings — Testing Strategy

- **T-U1 — Auto-updater pubkey ↔ private-key drift CI gate** (F2) — `.github/workflows/release.yml`. SEVERITY: HIGH. Issue: `[ci] Re-derive pubkey from TAURI_SIGNING_PRIVATE_KEY and diff against tauri.conf.json before build`.
- **T-U2 — `capture:saved` event protocol untested across emitters** (F5) — `snk-capture/src/commands.rs:13,24,40`; `snk-annotate/src/commands.rs:76`; `snk-ocr/src/plugin.rs:43`. SEVERITY: MEDIUM. Issue: `[tests] Add cross-plugin event protocol test using tauri::test::mock_app`.
- **T-U3 — TS bindings test names but not Rust acceptance of JSON args** (F6). SEVERITY: MEDIUM. Issue: `[tests] Rust-side test that fires actual TS-binding JSON through tauri::test::get_ipc_response`.
- **T-U4 — `OnceLock` test pollution in `snk-ocr::sidecar` acknowledged but unfixed** (F8) — `crates/snk-ocr/src/sidecar.rs:266-269`. SEVERITY: LOW. Issue: `[tests] Serialize env-mutating tests in snk-ocr::sidecar (Mutex or serial_test)`.
- **T-U5 — No real-image OCR fixture; integration test feeds blank PNG** (F10) — `crates/snk-ocr/tests/integration_test.rs:16-45`. SEVERITY: MEDIUM. Issue: `[tests] Commit hello-world OCR fixture + assert text extraction quality`.
- **T-U6 — Image clipboard items insertable but never pasteable; UX paper-cut** (F11) — `crates/snk-clipboard/src/commands.rs:29-35`. SEVERITY: MEDIUM. Issue: `[clipboard] Either implement image paste OR hide image rows in popup with test`.
- **T-U7 — Wrapped `Library(LibraryError)` wire shape untested** (F12). SEVERITY: MEDIUM. Issue: `[tests] Snapshot-test wire shape of Library(LibraryError) wrapper across crates`.
- **T-U8 — Background-task `_step` extraction pattern (Round 2 NI2)** — every `tokio::spawn` lacks a testable inner-step. SEVERITY: MEDIUM (architectural). Issue: `[arch] Extract worker_step from background spawn blocks for unit testability`.
- **T-U9 — Manual checklist (spec §10.5) has no enforcement mechanism (Round 2 NI3)** — 8 manual gates, no issue templates, no CI hook. SEVERITY: MEDIUM. Issue: `[release] Convert spec §10.5 manual checklist to enforced release-readiness gate`.
- **T-U10 — `cargo audit` / `pnpm audit` belong as a 4th test pyramid layer (Round 2 NI5)** — adds to O-U3 framing. SEVERITY: MEDIUM. Issue: `[docs] Update spec §10.1 to include Supply-chain layer in test pyramid`.
- **T-U11 — Migration test asserts `current_version == migrations().count()` (R8)** — catches future hardcode drift. SEVERITY: LOW. Issue: `[tests] Assert migration count matches latest version applied`.

---

## Cross-Pollination Insights (Round 2)

### Tradeoff Tensions

#### TT1 — Auto-update urgency vs user consent (Adversary F7 vs Operator #7)
- **Adversary:** "Add user confirmation before `download_and_install`; auto-install is hostile for a side project."
- **Operator:** "For an unstaffed side project, aggressive auto-update is the *only* way to push a security fix to the install base inside hours."
- **For v1:** Operator wins for security-class updates; Adversary wins for default releases.
- **Resolution (per Maintainer R-Adv F7 + N4):** Two-mode updater. Default: download in background, prompt to restart. Add `urgency` field to `latest.json`; security-flagged updates push automatically with non-dismissable post-24h banner. Signature failures are terminal regardless. **Design the state machine once (per M-U10) before implementing.**

#### TT2 — Delete dead TS packages (Maintainer F1) vs repurpose as About panel diagnostics (Operator R-M F1)
- **Maintainer:** Delete `@snk/ocr` and `@snk/updater` — they're never imported.
- **Operator:** Consume them in the Settings → About panel as a debug surface.
- **For v1:** Build the About panel first (CP7), then re-evaluate. If About panel ships and uses them, keep. Otherwise delete.

#### TT3 — Bundle Tesseract on macOS (Operator #4) vs don't bundle from unverified upstream (Adversary F11)
- **Operator:** Bundle Tesseract via `brew install` then copy — macOS is silently broken without it.
- **Adversary:** Choco/brew upstream compromise becomes signed-installer malware.
- **For v1:** Bundle on both, with hash-pinning per Adversary F11(c) — vendor in-repo with hash-pinned download. Either platform's "free download then copy" is the same supply-chain risk.

#### TT4 — Image clipboard half-shipped (Testing F11) vs blast-radius minimization (Adversary F2)
- **Testing:** Implement image paste or filter image rows from popup.
- **Adversary:** Do not implement image paste — keystroke-injecting arbitrary clipboard images is worse than the current broken UX.
- **For v1:** Adversary's framing wins. Either don't store images at all unless opt-in setting OR filter them from the popup with a clear "image clipboard items are view-only" affordance. **Do not** add a paste path for images.

#### TT5 — Encrypted DB (Adversary F13) vs operator debuggability (Operator R-Adv F13)
- **Adversary:** Offer SQLCipher with OS-keychain-derived key.
- **Operator:** Encryption breaks "send me your DB file" support workflow.
- **For v1:** Off by default. Document plaintext storage honestly in PRIVACY.md (after CB3 fix). Revisit encryption in v1.x when support workflow is mature.

#### TT6 — Universal macOS binary (Operator #14) vs per-arch CI exercise (Testing T3)
- **Operator:** Halve macOS CI time; one bundle.
- **Testing:** Matrix per-arch to exercise both code paths against native runners.
- **For v1:** Matrix the *test/build* per-arch (current); collapse *release artifact* to one universal bundle.

#### TT7 — Manual smoke checklist (CLAUDE.md known limit) vs full E2E (Testing F1)
- **CLAUDE.md:** Windows can't smoke in CI (non-interactive).
- **Testing:** Need at least one smoke per OS.
- **For v1:** Test the assumption — see CP8. macOS gets full E2E; Windows gets minimum "binary starts + library paints + no panic" if interactive enough. Either way, some runtime gate beats zero.

### Amplified Concerns (escalated by cross-pollination)

#### AC1 — File-logging absence makes every other finding undetectable in field
Adversary R-Op F1, Maintainer R2, Testing R7 all confirm: CB5 (no file logging) compounds CB1 (XSS), CB6 (migration failure), CP5 (silent watcher death), CP1 (updater signature errors), Maintainer F7 (`From<io::Error>` blanks path), Maintainer F8 (`Migration` lies "from 0 to 4"). **CB5 must land before fixing M-U2 / Maintainer F8 — otherwise the fixes improve strings nobody sees.**

#### AC2 — "Sensitive clipboard" feature is triply-promised, zero-implemented
Adversary F2 + Testing F4 + Maintainer F4 (derivative on README) + Operator #2 (PRIVACY.md proxy) + spec §10.5 manual checklist line. Five independent surfaces reference an unimplemented feature. **The Round 2 N1 ("designed but not implemented") meta-finding is the strongest cross-pollination signal in the entire review.**

#### AC3 — Test fixture references to unimplemented features actively mislead audits
Testing F4 noted `settings.rs:85-87` test fixture for `clipboard.app_blocklist = ["1Password", "KeePass"]`. Adversary R-Test F4 amplifies: a security auditor doing source review would cite this fixture as evidence the feature is tested. **Planted fixtures of unimplemented features need to be removed or replaced with `#[ignore = "not yet implemented"]`.**

#### AC4 — Three independent perspectives identified `paste_item` as worst single command (Round 2 Operator NI2)
Adversary F1 chain end-point + Operator #3 (capability fan-out) + Testing F11 (untested rejection) all converge. `paste_item` is the highest-blast-radius single IPC surface in the app. **CB4 (per-window capabilities) is the structural fix; O-U8 (audit log) is the diagnostic supplement.**

#### AC5 — `cargo build` in release job has full secrets in env
Adversary Round 2 N6: any transitive `build.rs` can read `TAURI_SIGNING_PRIVATE_KEY`. Combined with CP3 (mutable-tag actions) and A-U8 (split jobs), this is the credible end-to-end pipeline-compromise chain. **A-U8 must land regardless of other CI fixes.**

### New Insights (emerged only from cross-pollination)

#### NI1 — "Designed but not implemented" is the meta-vulnerability (Adversary Round 2 N1)
Across PRIVACY.md, design spec, README, schema columns, test fixtures, and setting keys: at least 12 verified instances where documented features have no implementation (Adversary F5; Operator #2/#13/#14; Maintainer F4/F5/F12/N5; Testing F1/F4/F11). **Recommendation per Maintainer R1:** ship-or-strip pass before v1 — every aspirational claim either gets implemented, deleted, or marked "roadmap, not v1.0."

#### NI2 — Background tasks are the dominant untested surface (Testing Round 2 NI2)
Every `tokio::spawn` and `std::thread::spawn` lacks an extracted testable inner step. Five+ background tasks across plugins, all untestable. Pattern fix: extract `worker_step` so spawn blocks become 3-line wrappers around testable units. **Architectural change but unlocks 5+ years of test coverage** (T-U8).

#### NI3 — Dead/unused code zones double as silent persistence sites (Adversary Round 2 N2)
Dead `@snk/ocr`/`@snk/updater` packages (Maintainer F1) + unused `_app` parameter (A-U5) + missing OCR startup sweep (O-U2) + silent watcher death (CP5) share an adversary-friendly property: zones where state change goes unnoticed. **The systemic property is "fire and forget without observability."**

#### NI4 — Documentation-as-test category is entirely missing (Testing Round 2 NI1)
Three of four perspectives found doc/spec/code drift independently. Systemic fix: `scripts/verify-docs.sh` that parses PRIVACY.md bullets, README feature list, and spec §10.5 checklist and asserts each has a corresponding code path or test. **Catches NI1 going forward.**

#### NI5 — Migration is the single most likely operational disaster (Operator Round 2 NI1)
Stacking Operator #5 (no backup) + Maintainer F8 (wrong from/to) + Testing F7 (no forward-compat tests): v1.1 ships with a new migration, fails on real-world DB shape no fixture exercises, error message lies about version, no backup, no log file, no fixture test that caught it. **CB6 + Operator NI1's `tests/migration_forward_compat.rs` is the cheapest insurance in the entire project.**

#### NI6 — Adversary's F1+F6+F9 attack chain is testable as a single property test (Testing Round 2 NI7)
> "For any string `s` containing HTML/JS payloads, after rendering via FTS snippet, `window.__TAURI__` should be undefined OR the IPC mock should not have received any invocation."
Single test against OWASP XSS cheat sheet corpus catches CB1, CB4, A-U7 chain in one assertion. **High-leverage; infrastructure already exists in the test suite.**

#### NI7 — Pubkey/private-key drift is silent and unrecoverable (Testing F2; reinforced by Adversary R-Test F2)
If `tauri.conf.json:109` pubkey diverges from `TAURI_SIGNING_PRIVATE_KEY` (rotation, accidental regenerate, secret-store recovery), every update silently fails forever — and without CB5 there's no log. **T-U1 (pubkey-drift CI gate) is the cheapest insurance against the most operationally expensive disaster.**

#### NI8 — Smoke test signing flow, not just signing success (Operator Round 2 NI6)
Current cmd.exe smoke verifies "we can sign." Should also verify "a signed thing verifies against our embedded pubkey." Combines T-U1 + Adversary F8 signature-flow gate + Operator #12 taste objection into one improved smoke step.

---

## Recommended GitHub Issue Backlog

Ordered: BLOCKER first, then HIGH, MEDIUM, LOW. Designed for direct `gh issue create` use.

### BLOCKERS (must land before v1.0 tag)

- **[BLOCKER]** `[security] Stored XSS in FTS snippet rendering — sanitize + enable CSP` — Replace `dangerouslySetInnerHTML` with React elements built from literal `<mark>` split; set a real CSP in `tauri.conf.json`. (Adversary, Testing, Operator, Maintainer)
- **[BLOCKER]** `[privacy] Implement (or strip) sensitive-clipboard exclusion before v1` — Honor OS exclusion formats + implement `clipboard.app_blocklist`, OR remove the schema column / setting / spec claim / test fixture. (Adversary, Testing, Maintainer, Operator)
- **[BLOCKER]** `[privacy] Reconcile PRIVACY.md updater-disable and Microsoft Store claims` — Implement `updater.enabled` setting + check OR edit PRIVACY.md before tagging. (Adversary, Operator, Testing, Maintainer)
- **[BLOCKER]** `[security] Split Tauri capabilities per window per design spec §8.3` — Per-window capability files (`library.json`, `capture-overlay.json`, `clipboard-popup.json`, `annotate.json`, `settings.json`); enforces least privilege the design committed to. (Adversary, Operator, Maintainer)
- **[BLOCKER]** `[ops] Add file-based logging with daily rotation + panic hook` — `tracing-appender` under `app.path().app_log_dir()` + `std::panic::set_hook` writing to `crashes/`; blocks post-release diagnosis of every other finding. (Operator, Adversary, Maintainer, Testing)
- **[BLOCKER]** `[reliability] Pre-migration backup + forward-compatibility test` — Copy DB before migrations; restore on failure; replace hardcoded `to: 4` with `migrations().current_version()`; add fixture-based `tests/migration_forward_compat.rs`. (Operator, Testing, Maintainer)

### HIGH (land within v1.0.x; major risk reduction)

- **[HIGH]** `[updater] Sign latest.json + downgrade floor + signature-error terminal handling` — Sign manifest with minisign; refuse strict downgrades; distinguish signature errors as terminal (disable for process lifetime). (Adversary, Operator, Testing)
- **[HIGH]** `[ci] Re-derive pubkey from TAURI_SIGNING_PRIVATE_KEY and diff against tauri.conf.json before build` — Cheapest insurance against silent updater break. (Testing, Adversary, Operator)
- **[HIGH]** `[ci] Split build job (no secrets) from sign+publish job (minimum-needed secrets)` — Reduces `build.rs` secret-exfil surface. (Adversary)
- **[HIGH]** `[ci] Pin CI actions by SHA + pin/verify Tesseract chocolatey source` — Pin all third-party actions by commit SHA; pin choco Tesseract version + SHA256-verify; drop `--prerelease` from `dotnet tool install sign`. (Adversary, Operator)
- **[HIGH]** `[ocr] Bundle Tesseract for macOS or surface missing-dependency banner` — macOS OCR is silently broken; either bundle (with hash pinning per CP3) or first-run banner. (Operator, Maintainer, Testing)
- **[HIGH]** `[security] Set a real Content-Security-Policy in tauri.conf.json` — Defense-in-depth even after CB1 fix. (Adversary)
- **[HIGH]** `[security] Tighten assetProtocol scope to captures/ and clipboard/ subdirs only` — Currently `$APPDATA/**` exposes entire SQLite. (Adversary)

### MEDIUM (post-release follow-up)

- **[MEDIUM]** `[ops] Add security-events.log (sig failures, capability denials, paste_item, autostart toggles, panic captures)` — Append-only, never rotated, surfaced in Settings → About. (Operator)
- **[MEDIUM]** `[ops] IPC audit log for paste_item + destructive commands with source window label` — Uses the dead `_app` parameter; defense-in-depth + diagnostic surface. (Operator, Adversary)
- **[MEDIUM]** `[ux] Add Settings → About panel with version + paths + updater status` — Version, data dir + "Open", log dir + "Open", updater status, last check. (Operator, Maintainer)
- **[MEDIUM]** `[clipboard] Retry watcher init + expose health event for offline state` — Exponential backoff (cap 60s) + `clipboard:unavailable` event + `clipboard_status` command. (Operator, Testing)
- **[MEDIUM]** `[clipboard] Replace SKIP_NEXT AtomicBool with Mutex<HashSet<hash>> for testable skip semantics` — Removes race, adds observability. (Maintainer, Testing)
- **[MEDIUM]** `[clipboard] Either implement image paste OR hide image rows in popup with test` — Prefer hide-with-test per Adversary's tradeoff resolution. (Testing, Adversary)
- **[MEDIUM]** `[ocr] Bounded queue + startup sweep for captures missing ocr_text` — `mpsc::channel(100)` + `SELECT id FROM captures WHERE id NOT IN (SELECT capture_id FROM ocr_text)`. (Operator)
- **[MEDIUM]** `[ocr] Sandbox Tesseract sidecar (job object / sandbox-exec) + per-invocation timeout` — Limits PNG-CVE blast radius. (Adversary)
- **[MEDIUM]** `[release] Build macOS universal binary (--target universal-apple-darwin)` — Single .dmg; halves macOS CI; design committed to it. (Operator)
- **[MEDIUM]** `[release] Document first-tag-must-be-plain-SemVer + add fallback endpoint` — Avoid `/releases/latest/` semantic trap. (Operator)
- **[MEDIUM]** `[release] Convert spec §10.5 manual checklist to enforced release-readiness gate` — Issue templates per gate OR `release-readiness.sh` attestation. (Testing, Operator)
- **[MEDIUM]** `[ci] Split coverage reporting between pure-logic and IPC surfaces` — Current 90% gate measures the wrong things. (Testing, Adversary, Operator)
- **[MEDIUM]** `[ci] Add nightly cargo-audit + pnpm-audit + per-release SBOM (cyclonedx)` — Design §10.4 committed; supply-chain layer of test pyramid. (Operator, Testing)
- **[MEDIUM]** `[ci] Add minimal E2E smoke per OS with uploaded runtime artifact` — At minimum library window paints; full E2E where interactive desktop available. (Testing, Operator)
- **[MEDIUM]** `[ipc] Generate TS error types from Rust + enforce typed-error rule` — `ts-rs` or `specta`; lint to enforce CLAUDE.md rule. (Maintainer, Testing)
- **[MEDIUM]** `[ipc] Add per-window authorization middleware for destructive commands` — Use the `_app: AppHandle` parameter to assert window label for `hard_delete_capture`, `purge_trash`, `set_setting`. (Adversary)
- **[MEDIUM]** `[errors] LibraryError::Io should preserve OS message; consider removing From<io::Error>` — Land after CB5 to avoid improving invisible strings. (Maintainer)
- **[MEDIUM]** `[updater] Design unified state machine for signature-rejection and policy-suppression before implementing` — Avoid PR collision; add `RejectedBySignature`/`SuppressedByPolicy`/`Skipped` variants. (Maintainer)
- **[MEDIUM]** `[reliability] Panic boundary around plugin setup() with user-facing error` — Today setup panic = silent app death. (Testing/Maintainer derivative)
- **[MEDIUM]** `[tests] Add cross-plugin event protocol test using tauri::test::mock_app` — Test `capture:saved` payload shape across 3 emitters and OCR/frontend consumers. (Testing)
- **[MEDIUM]** `[tests] Rust-side test that fires actual TS-binding JSON through tauri::test::get_ipc_response` — Locks IPC argument naming. (Testing)
- **[MEDIUM]** `[tests] Commit hello-world OCR fixture + assert text extraction quality` — Current integration test feeds blank image. (Testing)
- **[MEDIUM]** `[tests] Snapshot-test wire shape of Library(LibraryError) wrapper across crates` — Freeze wrapped-enum serde contract. (Testing)
- **[MEDIUM]** `[arch] Extract worker_step from background spawn blocks for unit testability` — Pattern fix for clipboard watcher, OCR queue, updater interval. (Testing)
- **[MEDIUM]** `[security/property] OWASP XSS corpus property test against SearchBar snippet rendering` — Single property test catches CB1+CB4+A-U7 chain. (Testing/Adversary cross-pollination NI6)
- **[MEDIUM]** `[docs] CI script: verify-docs.sh asserts PRIVACY.md / README / spec §10.5 claims map to code` — Catches NI1 doc-drift category going forward. (Testing meta)
- **[MEDIUM]** `[privacy] Document retention defaults + offer SQLCipher opt-in for local data at rest` — Off by default per TT5; PRIVACY.md must call out plaintext. (Adversary)
- **[MEDIUM]** `[validation] Validate PNG magic bytes + state_json schema + max payload size` — `save_annotation` / `derive_capture` accept any bytes today. (Adversary)
- **[MEDIUM]** `[cleanup] Delete or wire dead @snk/ocr and @snk/updater TS packages` — Either delete or consume in About panel per TT2. (Maintainer, Testing, Adversary, Operator)

### LOW (polish / sweep)

- **[LOW]** `[refactor] Use snk_library::LibraryState re-export + lint for ::plugin:: reach-ins` — Mechanical sweep + CI grep. (Maintainer, Testing)
- **[LOW]** `[tests] Shared test_support fresh_db helper to remove tempdir leaks` — `(TempDir, Db)` tuple. (Maintainer, Testing)
- **[LOW]** `[updater] Set MissedTickBehavior::Delay + collapse redundant first tick` — Avoid burst behavior on flaky networks. (Operator)
- **[LOW]** `[refactor] Split AnnotateCanvas.tsx into shape-specific modules under shapes/` — 523 LoC > 500 ceiling. (Maintainer)
- **[LOW]** `[refactor] Load tray icons from disk OR add CI script verifying THEMES keys ↔ Rust constants` — Theme registration drift risk. (Maintainer)
- **[LOW]** `[cleanup] Drop unused thiserror dep + resolve "later phase" comment in snk-hotkeys` — Dead dep + stale TODO. (Maintainer)
- **[LOW]** `[docs] Declare Linux as dev-only; add "Builds on Linux for dev convenience" note` — README/spec drift. (Maintainer)
- **[LOW]** `[docs] Consolidate dev-setup; CLAUDE.md and README cross-link to single source` — Drift risk. (Maintainer)
- **[LOW]** `[docs] Document UUIDv7 invariant (monotonicity + no HTML/JSON specials) in snk-library/src/lib.rs` — Implicit invariant across 4 sites. (Maintainer)
- **[LOW]** `[logging] Redacted<T> newtype for sensitive fields in tracing output` — Guard against future drift in tracing call sites. (Adversary)
- **[LOW]** `[tests] Serialize env-mutating tests in snk-ocr::sidecar (Mutex or serial_test)` — Known acknowledged-but-unfixed flake risk. (Testing)
- **[LOW]** `[tests] Assert migration count matches latest version applied` — Catch hardcoded-to-N drift. (Testing)
- **[LOW]** `[ci] Replace cmd.exe smoke with vendored test artifact + verify roundtrip` — Auditability + adds sig-verify gate (NI8). (Operator)
- **[LOW]** `[release] Canary procedure for first user-facing tag` — Tag prerelease first, promote after 1-2 weeks. (Operator)
- **[LOW]** `[docs] Update spec §10.1 to include Supply-chain layer in test pyramid` — Doc improvement alongside O-U3. (Testing)

---

## Blind Spots

Areas the selected perspectives (Adversary, Operator, Maintainer, Testing Strategy) didn't adequately cover:

- **Accessibility (a11y).** No perspective addressed screen-reader, keyboard-navigation, or contrast compliance. A library window + popup that don't work for keyboard-only users is a real accessibility miss for a "share-friendly side project."
- **Internationalization / locale.** OCR language defaulting to English-only is referenced once; no perspective examined RTL languages, font fallback in annotations, OS locale detection, or date/time formatting in the library UI.
- **Performance under load / scaling characteristics.** Adversary F13 notes unbounded growth; Operator notes OCR queue backlog. No perspective measured: how does the library window render with 10k captures? FTS search latency at 100k rows? Clipboard popup with 200 items? The design doc references performance targets that weren't tested against.
- **User-facing design / UX quality of the first-run flow, settings affordances, error states.** No perspective examined whether the first-run wizard actually onboards a non-technical user, or whether error toasts are actionable. Several perspectives note "user sees inscrutable error" — but didn't audit the existing error UI.
- **Distribution channels beyond GitHub Releases.** No perspective examined: signed-installer behavior on Defender SmartScreen first-run, Gatekeeper quarantine on macOS first-launch (.dmg notarization vs xattr stripping), the upgrade-from-zero-trust experience.
- **Localization of secrets at rest.** Adversary covers plaintext DB; no one covered: clipboard images written to disk under `clipboard/` as raw PNG (not just rows in SQLite), preview cache `.preview.png` files, annotation autosave artifacts.
- **License / legal posture.** No perspective examined LICENSE files, third-party license aggregation (required for Mac App Store; recommended for any redistribution), or the implications of bundling Tesseract (Apache 2.0).
- **Telemetry / crash-reporting trade-offs against privacy stance.** Operator notes "no telemetry by design" + "we have no install base visibility." No perspective examined whether an opt-in crash-report flow (Sentry-style, privacy-preserving) could close the gap without violating the privacy stance.
- **Mobile-adjacent or alternative form factors.** Not relevant for v1, but no perspective noted it as out-of-scope.
- **Source-code-signing-key recovery and rotation procedures.** Adversary covers the long-lived secret risk; nobody covered the operational "what happens if the maintainer loses the Azure Trusted Signing cert" recovery story.
