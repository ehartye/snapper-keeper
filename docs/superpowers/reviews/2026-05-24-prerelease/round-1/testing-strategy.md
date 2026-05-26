# Testing Strategy — Round 1

Perspective: testing-strategy
Target: snapper-keeper @ main (post-Phase 7, pre-first-public-release)

## Headline

The test suite is broad-but-shallow: extensive in-process Rust unit/integration
coverage of `snk-library` persistence, but **the entire IPC perimeter, the
cross-plugin event protocol, the auto-updater, the watcher thread, the
release-pipeline signing, and several security-relevant features the schema
claims to support are completely untested.** The pyramid in the design doc
(spec §10.1) calls for an E2E layer via `tauri-driver`; that layer does not
exist. CI runs unit/integration only and never instantiates a Tauri app, signs
nothing, and never exercises the updater. The smoke test surface is documented
as "manual" (CLAUDE.md known-limitation block) and the manual checklist (spec
§10.5) is the only thing standing between green CI and a broken first release.

Below: every concrete gap, file:line, and the bug it would let ship.

---

## Findings

### F1. The `tauri-driver` / WebDriver E2E layer from the design does not exist

- **What:** Design §10.2 calls for ~5% E2E coverage via `tauri-driver` against
  a real built binary on each OS — "smoke tests: hotkey → capture → save →
  library shows it; clipboard popup → pick → pasted into target window. Runs
  on PRs + nightly. Gates releases." None of this exists. No
  `tauri-driver`/`webdriver` deps anywhere; the `.github/workflows/` directory
  contains only `ci.yml` and `release.yml`, neither of which runs E2E.
- **Where:** Absent. Should exist as a `crates/snk-e2e/` test crate or
  `app/e2e/` Playwright/WebDriverIO suite, gated in CI.
  - `.github/workflows/ci.yml:46-62` is the entire Rust test runner — only
    `cargo test --workspace`. No `tauri-driver`.
  - `.github/workflows/ci.yml:87-112` `build-app` is *compile only* on each
    OS — never runs the binary.
  - `.github/workflows/release.yml` has no smoke step between "Build Tauri
    app" and "Upload" — the installer is shipped without ever being launched.
- **Why it matters:** Every IPC contract, every emit/listen pair, every
  hotkey registration, the window-creation race that bit phase 1, the actual
  paste-into-target-app flow (paste.rs uses raw `SendInput`/`CGEvent` —
  zero integration test) all ship unverified. CLAUDE.md acknowledges this:
  "CI's `build-app` job verifies the compile across all three OSes; runtime
  verification is manual." Manual verification fails to scale and is exactly
  the regression vector that already produced the v0.0.0-test5 macOS keychain
  hang (per memory).
- **Confidence:** High.
- **Suggested alternative:** Even one minimal smoke test per OS — launch
  binary, invoke `plugin:snk-capture|capture_full_screen` via the WebDriver
  bridge, assert a row appears via `plugin:snk-library|list_captures` — would
  give the first real cross-plugin integration signal. A scheduled nightly job
  is the spec's stated plan and is not implemented.

### F2. The auto-updater is untested end-to-end; updater plugin tests only check enum getter/setter mechanics

- **What:** `crates/snk-updater/src/plugin.rs` exposes 8 tests
  (`tests` module starting line 167). Every test is a `UpdaterState` get/set
  on the local in-memory `Mutex<UpdateStatus>`. Zero tests cover:
  - The actual update check (`do_update_check`, lines 56-134), including
    network failure, malformed `latest.json`, signature verification failure.
  - The background download path inside `tokio::spawn` (lines 76-114) —
    a panic in there is silently swallowed.
  - The 24-hour interval loop (lines 152-159) — if the first `interval.tick()`
    swallow is ever removed, updates spam GitHub every poll cycle.
  - The 5-second startup delay (line 147) — if the app is closed in <5s
    no check fires, and there is no catch-up.
- **Where:** `crates/snk-updater/src/plugin.rs:167-265`. Notice that
  `do_update_check` is the function with all the state transitions but it is
  never invoked from any test.
- **Why it matters:** A broken updater is unrecoverable in the field — users
  cannot ship a fix because the fix can't be delivered. The spec's manual
  checklist (§10.5) lists "Update flow: install old build · check for update ·
  apply · relaunch" as a manual gate but there is no record of it having been
  exercised between phase 7 and v0.1. The pubkey embedded in
  `app/src-tauri/tauri.conf.json:109` is **not validated against the private
  key referenced as `${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}` in
  `release.yml:174`** by any test. If they ever drift, every update is
  rejected at the user.
- **Confidence:** High.
- **Suggested alternative:**
  - A test that mocks the `Updater` HTTP layer (or hits a local fixture
    server) and exercises `do_update_check` through all five `UpdateStatus`
    transitions. Verify state propagation to the emitter.
  - A CI step in `release.yml` that re-derives the pubkey from
    `TAURI_SIGNING_PRIVATE_KEY` and `diff`s against the literal in
    `tauri.conf.json`. Cheap, prevents the silent-key-drift failure mode.
  - A nightly job that downloads the *previous* release, installs it in a VM,
    runs `check_for_update` against the current `latest.json`, and confirms a
    successful download + signature verify. This is the only way to catch
    pipeline regressions before users do.

### F3. The clipboard watcher thread has zero tests

- **What:** `crates/snk-clipboard/src/watcher.rs` is the single most
  security-sensitive surface in the app — it polls every clipboard mutation
  the OS exposes, persists it to disk + DB, and is supposed to honor a "skip
  own writes" handshake. Zero `#[cfg(test)]` in the file.
- **Where:** `crates/snk-clipboard/src/watcher.rs` (the file ends with
  `poll_image` — no tests module).
- **Why it matters:**
  - The "skip own writes" handshake is a **global `static AtomicBool
    SKIP_NEXT`** (line 16). A user pasting a clipboard item while the watcher
    is mid-poll has a textbook race: the user's own copy could be skipped, or
    the app's auto-copy could be persisted. No test asserts the timing.
  - The watcher's `poll_text` branch returns `true` for *every* non-empty
    text payload (line 89-90) — meaning if the clipboard contains *both*
    text and image (e.g. browser copy of a styled image), the image is
    silently dropped. There's no test asserting this is the intended choice.
  - The 500ms poll interval (line 13) is hard-coded. If two clipboard
    mutations happen within the window, only the latter is recorded. No
    test of dedup-near-poll-boundary.
  - The watcher catches `Err` from `Clipboard::new()` at line 26 and
    *returns* (kills the thread silently). After that, the app appears
    healthy but clipboard history just stops working. No test, no recovery.
- **Confidence:** High.
- **Suggested alternative:** Refactor `poll_text`/`poll_image` to take a
  `ClipboardLike` trait so a scripted "clipboard" can drive them in a unit
  test. Then write tests for: (a) SKIP_NEXT consumed exactly once, (b)
  text-wins-over-image, (c) repeated identical text not re-inserted, (d)
  near-poll-boundary dedup, (e) the silent thread-death case is at minimum
  surfaced via tracing::error and ideally retried.

### F4. "Sensitive clipboard items" are schema-only — no detection code, no test

- **What:** The DB schema has a `sensitive INTEGER NOT NULL DEFAULT 0`
  column (`crates/snk-library/migrations/V002__clipboard_items.sql:11`) and
  a `ClipboardItem.sensitive: bool` field
  (`crates/snk-library/src/clipboard.rs:61`). The settings module's tests
  reference `clipboard.app_blocklist = ["1Password", "KeePass"]`
  (`settings.rs:85-87`). But **nothing in the codebase ever reads that
  setting, ever sets `sensitive = 1`, or ever consults the blocklist**.
  Grep for `sensitive` / `blocklist` in `crates/snk-clipboard/` returns
  zero matches.
- **Where:** Missing implementation in `crates/snk-clipboard/src/watcher.rs`
  (where source-app detection + blocklist check would fire). The spec's
  manual checklist explicitly lists this as a release gate:
  `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md:692`:
  > "Sensitive clipboard: 1Password copy → not in history"
- **Why it matters:** This is a privacy promise the README/PRIVACY.md likely
  makes. Shipping v1 with the schema column populated as `0` for every
  password-manager paste means the manual checklist line is impossible to
  satisfy — the test would fail, and there's nothing to fix without writing
  the feature. From a Testing Strategy lens: the gap was hidden by tests
  exercising the *column* (`clipboard.rs:91`) and the *setting key*
  (`settings.rs:85-87`) without ever asserting that an end-to-end clipboard
  poll respects them.
- **Confidence:** High (verified by grep).
- **Suggested alternative:** Either implement source-app detection in the
  watcher with a corresponding test, or remove the schema column + the
  blocklist setting key + the spec line so the test surface honestly
  reflects what ships. Either is acceptable; the current state — claim
  without implementation — is the worst option.

### F5. Cross-plugin event protocol (`capture:saved`) is untested

- **What:** Three crates emit `app.emit("capture:saved", &capture.id)`:
  - `crates/snk-capture/src/commands.rs:13,24,40`
  - `crates/snk-annotate/src/commands.rs:76`
  - The receiver `crates/snk-ocr/src/plugin.rs:43-61` consumes via
    `event.payload().trim_matches('"').to_string()` — which assumes the
    payload is a JSON-quoted string.
  - The frontend listener `app/src/windows/library/LibraryWindow.tsx:72` also
    consumes the event.
- No test in any crate asserts:
  - The wire shape of the payload (JSON-quoted string vs `{id: "..."}`).
  - That all three emitters produce identical shape.
  - That `trim_matches('"')` survives unusual ids (none can today because of
    UUIDv7, but the brittleness is real — if anyone ever switches to a
    custom id format containing a quote, OCR silently breaks).
  - That the frontend payload type matches.
- **Where:** Absent; should exist as a small Rust test using `MockRuntime`
  or, failing that, a TS contract test that asserts the listener handler
  shape matches what the emitter produces.
- **Why it matters:** OCR silently failing means the FTS index never
  populates for new captures. The user sees search returning nothing and
  reports "search is broken" — but the underlying bug is a wire-format
  mismatch that the test suite cannot detect. This is the exact failure
  mode CLAUDE.md rule #3 ("No plugin imports another plugin's internals.
  Cross-plugin communication is Tauri commands or events.") is meant to
  enforce — and the test suite has no enforcement.
- **Confidence:** High.
- **Suggested alternative:** A single test in `snk-ocr` that uses
  `tauri::test::mock_app` (Tauri 2's mock runtime) to assert
  `app.emit("capture:saved", "abc-123")` triggers `OcrQueue::enqueue`
  with `capture_id == "abc-123"`. Trivial to write; catches the entire
  failure mode.

### F6. TS bindings test command names but not Rust acceptance of the args

- **What:** Each `packages/snk-*/src/index.test.ts` (e.g.
  `packages/snk-library/src/index.test.ts:148-152`) confirms the TS side
  *sends* `{captureId: "c1", tagId: "t1"}` but never confirms the Rust
  side at `crates/snk-library/src/commands.rs:140-148` actually accepts
  those names. Tauri 2 does snake_case → camelCase conversion by default
  but that can be overridden per-command. If anyone adds
  `#[tauri::command(rename_all = "snake_case")]` to fix a different bug,
  every TS call site breaks at runtime, not test time.
- Same pattern: `pngData` (TS) vs `png_data` (Rust at
  `crates/snk-annotate/src/commands.rs:13`), `windowId` (TS) vs `window_id`
  (Rust at `crates/snk-capture/src/commands.rs:21`), etc.
- **Where:** All `packages/snk-*/src/index.test.ts` files. The Rust side
  has no `#[test]` that invokes the command handler with the JSON the TS
  side would send.
- **Why it matters:** Tauri's invoke handlers panic-translate to "unknown
  arg" errors at runtime, which surface as opaque error toasts in the
  frontend. Today it works because the convention is followed; the test
  suite gives no guarantee it stays followed.
- **Confidence:** Medium (the bug requires deliberate change; the test
  gap is structural).
- **Suggested alternative:** One test per command using `tauri::test::mock_app`
  + `tauri::test::get_ipc_response` that fires the actual JSON the TS
  binding would produce. Could be auto-generated from the command list.
  Alternatively: define a JSON Schema for the IPC layer and validate both
  sides against it in CI.

### F7. Migration tests don't carry data forward

- **What:** `crates/snk-library/src/migrate.rs:30-91` has tests for each of
  V001-V003 that **only assert tables exist**. There is no test that:
  - V001 data still queryable after V002 + V003 + V004 apply.
  - V003's `ocr_text` row inserted under V003 schema survives a future V005.
  - V004's `annotation_state` column added to existing `captures` rows
    preserves their existing data.
  - Downgrade — none tested, but `rusqlite-migration` supports it and
    `LibraryError::Migration.recoverable` advertises it.
- **Where:** `crates/snk-library/src/migrate.rs:30-91` are the migration
  tests. Notice there's no `M::up_down` usage and no fixture-based
  test that simulates "user installed v0.1, upgraded to v0.2, their data
  is still there".
- **Why it matters:** Existing users are the only users for whom migration
  matters. A migration bug strands the user with their library
  (irrecoverable for non-technical users). For a "share-friendly side
  project" with no telemetry (per CLAUDE.md), you will not learn about
  these bugs until a user emails you. Phase 7 introduced no migrations,
  but phase 8 will — the test surface needs to exist *before* the
  migration that triggers a bug.
- **Confidence:** High.
- **Suggested alternative:** A `tests/migration_integration.rs` that
  applies V001 only, inserts fixture rows representing realistic library
  state (a few captures, tags, clipboard items), then runs the remaining
  migrations one at a time, asserting the fixture rows still satisfy
  invariants after each step. Snapshot the DB schema at each version so
  schema drift in V001-V004 (e.g. an accidental `ALTER TABLE` in V005
  that drops a default) is caught.

### F8. `OnceLock` test pollution in `snk-ocr::sidecar` is acknowledged but unaddressed

- **What:** `crates/snk-ocr/src/sidecar.rs` uses two process-global
  `OnceLock`s: `TESSERACT_PATH` (line 16) and `BUNDLED_RESOURCE_DIR`
  (line 20). The tests at lines 244-298 mutate `PATH` env-var (line 269)
  with a comment that *acknowledges* the parallelism issue:
  > "tests are single-threaded for env mutation here. (cargo runs each
  > test on its own thread; if this becomes flaky, gate behind a Mutex.)"
  The fix is not applied. `cargo test` by default uses a multi-threaded
  runner — if `which_finds_an_executable_we_just_created` happens to run
  concurrently with `bundled_tessdata_dir_returns_none_when_no_real_dir`,
  one test mutating PATH can poison the other.
- **Where:** `crates/snk-ocr/src/sidecar.rs:266-269` (the comment) and
  lines 246-289 + 291-305 (the tests themselves).
- **Why it matters:** A flaky test in CI gets ignored ("just rerun it");
  the underlying flake is a real concurrency hazard in the sidecar
  resolver. Once the test starts being unreliable, devs lose trust in
  the suite as a whole.
- **Confidence:** Medium (no current evidence of flake; the gap is
  structural).
- **Suggested alternative:** Either gate env-mutating tests with a
  module-level `Mutex` (the comment's own suggestion), or use
  `#[serial_test::serial]`. Cheap, lets the test stay rather than
  removing it.

### F9. Several Rust tests `std::mem::forget(dir)` to leak tempdirs

- **What:** Eight tests across `snk-library` use the pattern:
  ```rust
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("sk.db");
  std::mem::forget(dir);  // <-- leak
  Db::open(&path).unwrap()
  ```
  Examples: `captures.rs:333-336`, `clipboard.rs:252-256`,
  `ocr.rs:66-70`, `search.rs:171-175`, `settings.rs:52-56`,
  `tags.rs:173-177`, `orchestrate.rs:112-116`.
- **Why it matters:** Not a correctness bug, but each `cargo test` run
  leaks ~10 temp directories on the host. On CI that's swept; on a dev
  machine they accumulate in `%TEMP%`. More importantly the *pattern*
  encourages teams to ignore the lifetime — and the tests with the leak
  cannot easily be ported to use `Db::open` with a `Path` borrow because
  the borrow would outlive `dir`. The right pattern (returning `(Db,
  TempDir)` so the dir lives with the Db) was discovered locally —
  `hard_delete_removes_row_and_works_on_soft_deleted` at
  `captures.rs:766` does it correctly with `let tmp = tempfile::tempdir()`
  + passing `tmp.path()` through. The leak is gratuitous.
- **Confidence:** High.
- **Suggested alternative:** A small `TestDb` helper that bundles
  `(Db, TempDir)` and `Drop`s them together. Removes the leak and the
  bad pattern.

### F10. No fixture-based OCR test against a real image with known text

- **What:** `crates/snk-ocr/tests/integration_test.rs` has two tests. The
  one that exercises the real tesseract binary
  (`sidecar_extracts_text_from_image`, line 16) feeds it a **blank
  white image** and asserts the output is `<10 chars` or an error. It
  never confirms tesseract actually reads text correctly. There is no
  fixture image like "screenshot of a code snippet → 'hello world'
  appears in OCR output".
- **Where:** `crates/snk-ocr/tests/integration_test.rs:16-45`.
- **Why it matters:** The test passes whether tesseract is broken or
  working. Tesseract version drift (chocolatey upgrading to a newer
  build), tessdata regression, or a `--psm` change can all silently
  degrade OCR quality. From a Testing Strategy lens, this is a test
  that runs but doesn't *verify* — it only confirms tesseract exits
  successfully. The spec §10.2 specifically calls for "OCR runs the
  real tesseract sidecar against a small image fixture set" — the
  fixture set is absent.
- **Confidence:** High.
- **Suggested alternative:** Commit a tiny PNG fixture
  (`crates/snk-ocr/tests/fixtures/hello_world.png`, ~1KB) with rendered
  text "Hello World 123", and assert `output.text.to_lowercase()
  .contains("hello")` and `.contains("world")` and `.contains("123")`.

### F11. Image clipboard items can be inserted but never pasted; no test covers the rejection

- **What:** `crates/snk-clipboard/src/commands.rs:29-35` rejects any
  paste attempt for `kind == "image"` with
  `"paste for kind 'image' not yet supported"`. Meanwhile the watcher at
  `crates/snk-clipboard/src/watcher.rs:92-136` happily ingests image
  clipboard items. No test in any crate or frontend file asserts:
  - The rejection error reaches the frontend in a consumable shape.
  - The frontend renders the image clipboard items as non-pasteable
    (i.e. doesn't let the user click them only to get an error toast).
- **Where:** `crates/snk-clipboard/src/commands.rs:32`. The frontend
  consumer `app/src/windows/clipboard-popup/ClipboardPopupItem.tsx:14,39`
  has branches on `item.kind === 'text'` but no test confirms image
  rows are visually distinguished as non-pasteable.
- **Why it matters:** Users will copy an image, see it in the popup,
  click it, get an inscrutable error. This is the kind of paper-cut that
  ships with a v1 and tanks Hacker News goodwill. Easy to fix; the test
  gap masks it.
- **Confidence:** High.
- **Suggested alternative:** Either implement image paste before v1 (so
  the schema-vs-feature gap closes) or add a TS test asserting image
  rows render with `disabled` or are filtered out of the popup grid.

### F12. The Rust error enums wrap `LibraryError` without a serde-flatten — wire shape is untested

- **What:** Three crates wrap library errors as a `Library(LibraryError)`
  newtype variant:
  - `crates/snk-capture/src/error.rs:19-21`
  - `crates/snk-clipboard/src/error.rs:7-8`
  - `crates/snk-annotate/src/error.rs:7-8`
  The outer enum uses `#[serde(tag = "kind", rename_all = "kebab-case")]`.
  The wrapped `LibraryError` *also* uses
  `#[serde(tag = "kind", rename_all = "kebab-case")]`. Serde's behavior
  for `Library(LibraryError)` under an internally-tagged enum is to
  serialize as `{"kind":"library","0":{...wrapped...}}` — but **no test
  in any of the three crates asserts the wire shape**.
- The error.rs tests all use the round-trip-via-rust pattern (assert
  variant matches), never serialize-then-inspect for the wrapped case.
- The TS side has *no* type definitions for any of these errors (grep
  for `Error` in `packages/*/src/types.ts` returns empty). Frontend code
  treats invoke rejections as opaque `unknown`.
- **Where:**
  - `crates/snk-capture/src/error.rs:96-100` (only tests the
    non-wrapped variant's serde shape)
  - Same pattern at `crates/snk-clipboard/src/error.rs:83-87` and
    `crates/snk-annotate/src/error.rs:77-82`.
  - `packages/snk-library/src/types.ts` (grepped — no Error types)
- **Why it matters:** If serde's behavior changes (e.g. a future
  version flattens wrapping enums differently), or if anyone adds
  `#[serde(transparent)]` to a wrapper, the wire shape silently
  changes and the frontend can't tell the difference between a
  `LibraryError::NotFound` and a `CaptureError::Library(...
  NotFound)`. CLAUDE.md says "Errors cross the IPC boundary as typed
  enums" — the *typing* doesn't exist on the TS side, so the boundary
  is untyped in practice.
- **Confidence:** High.
- **Suggested alternative:**
  - One test per error enum that serializes the wrapped `Library`
    variant and asserts the exact JSON shape with `assert_eq!(json,
    r#"{"kind":"library","0":{"kind":"not-found","what":"x"}}"#)`.
    This freezes the contract.
  - Generate TS types for each error enum (e.g. via `ts-rs` or
    `specta`) so the frontend has discriminated unions and can switch
    on `err.kind === 'not-found'` instead of pattern-matching on
    `err.message`.

### F13. Coverage CI excludes the riskiest files by regex

- **What:** `.github/workflows/ci.yml:82-85` runs
  `cargo llvm-cov --fail-under-lines 90 --summary-only` with an
  `--ignore-filename-regex` that excludes:
  `plugin.rs|commands.rs|caret.rs|paste.rs|watcher.rs|hotkeys.src.lib\.rs|queue.rs|build.rs|foreground.rs|capture.src.grab\.rs|capture.src.orchestrate\.rs`
- The comment justifies this — these need a real Tauri runtime or live
  monitors/keyboard. Fair. But the 90% threshold is then **only over
  pure-logic files**, not over the surface that actually fails when it
  fails. Coverage numbers are misleading: you can hit 90% while having
  zero coverage on every file that handles IPC.
- **Where:** `.github/workflows/ci.yml:82-85`.
- **Why it matters:** Coverage signals trust. A repo that reports
  >90% line coverage suggests "the test suite covers this codebase".
  In reality the test suite covers the parts that don't change much
  and ignores the parts that change every release. The honest number
  (with `plugin.rs` etc. included) is far lower and would prompt the
  team to add E2E coverage.
- **Confidence:** High.
- **Suggested alternative:** Either drop the threshold gate (it's
  measuring the wrong thing), or split the coverage job into "logic
  coverage (gated at 90%)" and "IPC surface coverage (reported, not
  gated, with a target to improve)". The current single threshold
  hides where the real risk is.

### F14. No test for the watcher's silent thread-death on `Clipboard::new()` failure

- **What:** Specific instance of F3 worth calling out independently.
  `crates/snk-clipboard/src/watcher.rs:25-30`:
  ```rust
  let mut clip = match Clipboard::new() {
      Ok(c) => c,
      Err(e) => {
          error!(error = %e, "failed to open clipboard for watching");
          return;
      }
  };
  ```
  The thread dies. The app keeps running. Clipboard history is
  permanently broken until the user restarts. There is no recovery, no
  health check, no telemetry (per CLAUDE.md, no telemetry by design),
  no test.
- **Why it matters:** On macOS, accessibility/clipboard permissions can
  be revoked at any time via System Settings. The thread dies, the
  user notices their popup is empty, and there is no in-app hint as to
  why. A test couldn't fix the missing recovery, but it would document
  the failure mode and prompt the team to add a periodic re-init.
- **Confidence:** High.
- **Suggested alternative:** Retry loop with exponential backoff on
  `Clipboard::new()` failure + emit a `clipboard:unavailable` event so
  the frontend can surface a permissions-issue banner. Test the retry
  with a mock clipboard that fails N times then succeeds.

---

## Summary

The Rust unit/integration coverage of `snk-library` persistence is genuinely
solid — captures, tags, clipboard, search, FTS5 indexing, soft/hard delete,
trash purge are all well-covered with realistic in-process SQLite. That layer
is trustworthy and refactor-safe.

**Everything outside `snk-library` is a different story.** The entire IPC
perimeter, the cross-plugin event protocol, the updater, the watcher, the
hotkey registration, the paste synthesis, the Windows + macOS code-signing
pipeline, and the on-disk migration path between releases are all sitting
*outside* the test net. The CI coverage threshold (>90% line) is enforced
against a list that excludes every one of those files, which means the
trust signal coverage provides is actively misleading.

**Most concerning for v1 ship:** the design's "sensitive clipboard" promise
(1Password copies excluded) is schema-only with no implementation, the
auto-updater has zero end-to-end verification (and updaters that ship broken
are unrecoverable), and the manual smoke checklist is the only thing
gating real correctness. I would not ship a public release without (a) at
minimum one smoke test per OS that drives the binary, (b) a pubkey-drift
guard in `release.yml`, and (c) either implementing or removing the
sensitive-clipboard feature so the checklist line is honest.
