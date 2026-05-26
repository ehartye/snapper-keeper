# Maintainer Perspective — Round 1

Lens: A new developer 6 months from now opens this repo cold. Can they navigate, change a feature, and ship without paging an original author?

---

## Findings

### F1. Dead TS bindings: `@snk/ocr` and `@snk/updater` are wired everywhere but unused

- **What:** `packages/snk-ocr/` and `packages/snk-updater/` exist as full `@snk/*` workspace packages with `package.json`, source, tests, vitest config, and vite aliases — but **nothing in `app/src/` imports them**. Confirmed via `Grep` for `@snk/ocr`, `@snk/updater`, `checkForUpdate`, `getUpdateStatus`, `ocrStatus`, `plugin:snk-updater` — zero hits inside `app/src/`. The TS commands they expose are never invoked.
  - Updater behavior runs entirely from Rust: `crates/snk-updater/src/plugin.rs:142-160` spawns the periodic check loop in `setup()`, and the tray menu hits `snk_updater::plugin::check_for_update` directly from `app/src-tauri/src/main.rs:194-197`.
  - OCR is also fire-and-forget; `ocr_status` returns a hardcoded `"running"` string (`crates/snk-ocr/src/plugin.rs:15-17`) and is never called.
- **Where:** `packages/snk-ocr/`, `packages/snk-updater/`, `app/vitest.config.ts:41-42` (aliases for unused packages), `app/src-tauri/src/main.rs:194-197`.
- **Why it matters:** A new contributor sees a 6-package symmetric TS workspace and assumes the pattern is "every Rust plugin has a TS binding consumed by the app." They will read the bindings, write features against `ocrStatus()` or `checkForUpdate()`, and discover only after wiring that the architecture doesn't actually flow that way for these two. CI also runs tests against bindings that have no real customer.
- **Confidence:** High.
- **Suggested alternative:** Delete the two packages (and the vitest aliases). If they need to stay for symmetry / future use, document at the top of each `index.ts` that "these bindings are not currently consumed by the app — the updater runs on a Rust-side loop and emits `updater:status-changed`; subscribe to that event from React instead."

---

### F2. `LibraryState` is imported from another plugin's `::plugin` module by 4 crates

- **What:** `crates/snk-annotate/src/commands.rs:3`, `crates/snk-clipboard/src/commands.rs:9`, `crates/snk-clipboard/src/plugin.rs:13`, and `crates/snk-ocr/src/plugin.rs:6` all import `snk_library::plugin::LibraryState` via the full internal path, not the re-export `snk_library::LibraryState` that already exists in `crates/snk-library/src/lib.rs:21`.
- **Where:** Four locations above; clean re-export at `crates/snk-library/src/lib.rs:21`.
- **Why it matters:** CLAUDE.md says "no plugin imports another plugin's internals" (rule #3). The cross-plugin reach into `::plugin::LibraryState` literally types out the rule violation in the import path. The re-export at the crate root *is* the public-facing alternative; nobody uses it. The next refactor of `plugin.rs` (renaming `LibraryState` or moving it into a sub-module) will break every importer.
- **Confidence:** High — the rule is verbatim in CLAUDE.md and the code pattern is uniform across 4 sites.
- **Suggested alternative:** Mechanical sweep: replace `snk_library::plugin::LibraryState` with `snk_library::LibraryState` in all 4 files. Optionally add a clippy lint or a doc comment on `plugin.rs` saying "private; import from crate root."

---

### F3. CLAUDE.md says "errors cross IPC as typed enums" — 3 of 8 plugin command surfaces violate this

- **What:** `crates/snk-ocr/src/plugin.rs:15` returns `Result<String, String>`. `crates/snk-updater/src/plugin.rs:47, 52` return `Result<UpdateStatus, String>` and bare `UpdateStatus`. `crates/snk-capture/src/commands.rs:64-85` (`grab_screen_preview`) maps every IO error to `CaptureError::Os { message: format!(...) }` — typed at the variant level, but it loses structure compared to the documented contract.
  - Compare to `crates/snk-library/src/error.rs:4-22` and `crates/snk-clipboard/src/error.rs:4-18`, which do define typed enums with `#[serde(tag = "kind")]` per CLAUDE.md.
- **Where:** Listed above.
- **Why it matters:** The contract from CLAUDE.md ("Errors cross the IPC boundary as typed enums") is the basis on which a frontend dev decides whether to switch on `error.kind` or `try/catch` a string. Half the surface honors it, half doesn't, and there's no comment explaining which. New code will copy whichever sibling the author looked at first.
- **Confidence:** High.
- **Suggested alternative:** Either (a) introduce `OcrError`/`UpdaterError` enums to match the pattern, or (b) demote the CLAUDE.md rule to "preferred where the result type is non-trivial, opt out is fine for status-only commands" and add a comment at each `Result<_, String>` site.

---

### F4. README claims clipboard eviction limit is "configurable" — it isn't

- **What:** `README.md:22` reads "Content-hash deduplication, configurable eviction limit." The actual limit is a hardcoded `const MAX_UNPINNED: u32 = 200;` at `crates/snk-clipboard/src/watcher.rs:14`. No settings entry, no UI control. `crates/snk-library/src/settings.rs:76` *tests* a key called `clipboard.history_size`, but nothing reads it.
- **Where:** `README.md:22`; `crates/snk-clipboard/src/watcher.rs:14, 82, 129`; `crates/snk-library/src/settings.rs:76` (orphaned key name).
- **Why it matters:** First-time users adjust expectations from the README. A user reporting "I want to keep more than 200 items" gets pointed to a setting that doesn't exist. The `clipboard.history_size` key in tests creates a phantom contract — a contributor implementing this will pick a name and discover later it was already taken.
- **Confidence:** High.
- **Suggested alternative:** Either fix the docs ("200-item cap; configurable in a future release") or wire the eviction limit through `snk-library` settings using the `clipboard.history_size` key.

---

### F5. Tesseract on macOS is unbundled and undocumented in the README

- **What:** `docs/release-signing.md:73` explains "macOS bundles are not yet self-contained for OCR — users currently need `brew install tesseract`." But the `README.md` feature list (lines 24-27) advertises OCR as a flagship feature with no Mac caveat, and the README quickstart at line 56 lists `brew install tesseract` only for **dev**, not packaged-app users.
- **Where:** `README.md:24-27` (feature list silent on Mac OCR dependency); `README.md:56` (brew install scoped to dev); contrast with `docs/release-signing.md:73`.
- **Why it matters:** This is shipping in the first public release. A Mac user installs the signed bundle, captures a screenshot, never sees OCR text in search, and has no idea why. The information exists but lives in a doc most users won't read.
- **Confidence:** High.
- **Suggested alternative:** Add a Mac-specific note to the OCR feature bullet in `README.md` ("Mac users currently need `brew install tesseract` for the first release; auto-bundling coming in 0.2"). Surface the same in the first-run wizard if Mac + no tesseract is detected.

---

### F6. CLAUDE.md's "files >500 lines" rule has a current violator

- **What:** `app/src/windows/annotate/AnnotateCanvas.tsx` is 523 lines (`wc -l`). CLAUDE.md says "Files >500 lines are a red flag (Eric's standard). If you're approaching that, split the module." Note `crates/snk-library/src/captures.rs` is 991 lines but ~660 are tests, so non-test code is well within the rule — counting *lines* alone is misleading.
- **Where:** `app/src/windows/annotate/AnnotateCanvas.tsx`.
- **Why it matters:** The rule exists for a reason (cognition/readability) and the canvas file is exactly the kind of place that snowballs — Konva wrapper, drawing handlers, shape rendering, undo wiring tend to grow together. Either fix the file or fix the rule; the current state means new contributors don't know whether the rule binds.
- **Confidence:** High on the LOC count; medium on whether splitting is the right move.
- **Suggested alternative:** Either pull shape-specific draw/hit-test logic into `shapes/` (where `shapes/` already exists as a sibling dir), or update CLAUDE.md to call out the Konva canvas as an exception with a comment in-file.

---

### F7. `LibraryError`'s `From<std::io::Error>` discards the path and the underlying message

- **What:** `crates/snk-library/src/error.rs:39-46`:
  ```rust
  impl From<std::io::Error> for LibraryError {
      fn from(e: std::io::Error) -> Self {
          LibraryError::Io {
              path: String::new(),       // <— always blank
              reason: e.kind().to_string(),  // <— "permission denied", not the OS message
          }
      }
  }
  ```
  The `LibraryError::Io { path, reason }` variant is designed for path-bearing IO errors (see the explicit construction at `captures.rs:44-47`), but the `From` impl forces an empty path and a generic kind string. The test at `error.rs:122-133` even asserts the behavior is "reason contains permission denied" — i.e. coded-for, not noticed-and-fixed.
- **Where:** `crates/snk-library/src/error.rs:39-46`; assertion at `error.rs:122-133`.
- **Why it matters:** Any time a `?` operator promotes `io::Error` to `LibraryError`, the user gets `"io error at : permission denied"` with no clue *which file*. Crash reports / log noise. The information was right there at the call site.
- **Confidence:** High.
- **Suggested alternative:** Remove the `From<io::Error>` impl (so it can't be `?`-promoted blindly), and force callers to use `.map_err(|e| LibraryError::Io { path: ..., reason: e.to_string() })`. Or keep the impl but use `e.to_string()` so the OS message survives.

---

### F8. `Migration` error tells the user "from 0 to 4" regardless of actual transition

- **What:** `crates/snk-library/src/migrate.rs:17-23`:
  ```rust
  migrations()
      .to_latest(conn)
      .map_err(|e| crate::LibraryError::Migration {
          from: 0,
          to: 4,
          recoverable: e.to_string().contains("Backup"),
      })?;
  ```
  Both `from` and `to` are hardcoded literals. `to: 4` is silently load-bearing on the contributor remembering to bump it when `V005__*.sql` lands. `recoverable` is computed by substring-matching on an error message — fragile (`rusqlite_migration` could reword that string at any point).
- **Where:** `crates/snk-library/src/migrate.rs:17-23`.
- **Why it matters:** The error type pretends to carry useful information (`from`, `to`, `recoverable`) but the values are lies. Future contributor adds migration V005, ships it, the error keeps saying "from 0 to 4" forever. Or worse, a downstream consumer parses the discriminator and acts on stale numbers.
- **Confidence:** High.
- **Suggested alternative:** Either derive `to` from `migrations().current_version()` and remove `from`/`to` from the variant (since they aren't actually known here without a query) — or document that these fields are placeholders until proper migration introspection lands.

---

### F9. Theme registration is fanned out across 4 places — no single source of truth

- **What:** Adding a new theme requires updating:
  1. `app/src-tauri/src/main.rs:12-37` (PNG bytes + match-arm in `tray_icon_for`)
  2. `app/src/lib/theme.ts:9-18` (import the divider preview)
  3. `app/src/themes/<family>.css` and `.preview.tsx`
  4. `app/src/index.css` (import the CSS)
  5. The `THEMES`/`THEME_FAMILIES` registry in `theme.ts`

  `theme.ts:23-31` has a comment listing 3 steps; the comment misses the Rust-side tray icon constant in `main.rs`. So even the documentation of the seams is incomplete.
- **Where:** `app/src-tauri/src/main.rs:12-37`; `app/src/lib/theme.ts:9-18, 22-31` (comment); themes scattered across `app/src/themes/*.css|preview.tsx`.
- **Why it matters:** Theme additions are a known multi-step change and the steps cross the Rust/TS boundary. A contributor adding a 9th theme who only reads the comment in `theme.ts` will end up with a black tray icon (Rust falls back to `TRAY_HOLO_PNG`) and no warning. This is the kind of implicit knowledge that bites silently.
- **Confidence:** High.
- **Suggested alternative:** Either (a) load tray icons from disk at startup (so adding a PNG file is the whole step), or (b) update the comment block in `theme.ts:22-31` to enumerate every required change including the Rust side, and add a CI script that verifies `THEMES` keys match `themes/` files match `main.rs` constants.

---

### F10. `std::mem::forget(dir)` deliberately leaks tempdir handles in 5+ tests

- **What:** Every `fresh_db()` helper in `crates/snk-library/src/*.rs` (`captures.rs:332-337`, `clipboard.rs:252-256`, `search.rs:171-176`, `settings.rs:52-57`) and `crates/snk-capture/src/orchestrate.rs:112-117` does:
  ```rust
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("sk.db");
  std::mem::forget(dir);
  Db::open(&path).unwrap()
  ```
  The `mem::forget` is deliberate — it stops `tempfile::TempDir`'s Drop from cleaning up the directory while the test is running. But this leaks the directory permanently. Per-test the leak is harmless; in a workspace `cargo test` run, it leaves dozens of orphan dirs in `%TEMP%` / `/tmp`.
- **Where:** Five+ test helper functions across `snk-library` and `snk-capture`.
- **Why it matters:** Two issues. (1) The pattern is copy-pasted — a contributor refactoring tempdir handling has to chase it across 5 files. (2) No comment explains *why* the forget is there (it's to keep the dir alive after the local goes out of scope; the right fix is to return the `TempDir` alongside the `Db`).
- **Confidence:** High on the leak; medium on whether anyone has noticed in practice.
- **Suggested alternative:** Extract a shared test helper `fn fresh_db() -> (TempDir, Db)` in a `#[cfg(test)] mod common` or a tiny `snk-library/src/test_support.rs`. Callers bind both into local vars; Drop runs at end of test.

---

### F11. `snk-hotkeys` carries an unused `thiserror` dep + dead comment about future work

- **What:** `crates/snk-hotkeys/Cargo.toml:13` declares `thiserror.workspace = true`, but `crates/snk-hotkeys/src/lib.rs` has no `use thiserror::*` and no `#[derive(Error)]`. The crate also has the only `lib.rs`-only structure (no `plugin.rs`, no `error.rs`, no `permissions/`) of any plugin crate. The doc-comment at `lib.rs:3-4` says "A later phase reads bindings from `snk-library` (settings) and supports remapping" — no Phase 8 yet.
- **Where:** `crates/snk-hotkeys/Cargo.toml:13`; `crates/snk-hotkeys/src/lib.rs:3-4`.
- **Why it matters:** Dead dep slows builds slightly and confuses a new reader. The "later phase" comment is from Phase 1 and references work that never appeared in Phases 2-7 — readers will wonder if it's still on the roadmap or abandoned.
- **Confidence:** High on the dead dep; high on the dead comment.
- **Suggested alternative:** Drop `thiserror` from `snk-hotkeys/Cargo.toml`. Either delete the "later phase" comment or move the deferred work to a tracked issue/doc.

---

### F12. Linux is supported in CI + dev docs but excluded from the design spec

- **What:** The README:53-62 lists Linux deps and the CI `build-app` job runs `ubuntu-latest` (`.github/workflows/ci.yml:92-93`). But `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md:9` says "cross-platform (Windows + macOS)" and the release workflow (`.github/workflows/release.yml:18-30`) only publishes Windows + macOS bundles.
- **Where:** README:53-62, spec line 9, CI line 92, release line 18-30, CLAUDE.md "What this repo is" section (also "Windows + macOS").
- **Why it matters:** A new Linux contributor reads the README, follows the install steps, builds locally, then tries to run — and discovers Linux isn't a target. CI silently verifies *compilation* on Linux but the project doesn't claim to ship there. A drift between "we develop on Linux" and "we ship to Linux" should be explicit.
- **Confidence:** High.
- **Suggested alternative:** Add a one-liner near the top of README ("Builds on Linux for development convenience; published bundles target Windows and macOS only.") and call it out in the spec's "non-goals" section.

---

### F13. Hidden coupling: `SKIP_NEXT` static + `mark_skip_next()` is a process-global synchronization point

- **What:** `crates/snk-clipboard/src/watcher.rs:16` declares `static SKIP_NEXT: AtomicBool = AtomicBool::new(false);` and exposes `pub fn mark_skip_next()`. `crates/snk-clipboard/src/commands.rs:38` calls it before `clip.set_text(text)` to suppress the watcher re-firing on the app's own write. There's no per-instance state; the entire clipboard plugin coordinates through a process-wide global.
- **Where:** `crates/snk-clipboard/src/watcher.rs:16-20, 36-41`; `crates/snk-clipboard/src/commands.rs:38`.
- **Why it matters:** It works today because there's only ever one app instance and one watcher. But: any code path that races between `mark_skip_next()` and the next watcher tick (500ms in `POLL_INTERVAL`) can either skip a real write or fail to skip the synthetic one. Two `paste_item` calls in quick succession could clobber each other's flag. There's no comment explaining the race window or why a poll interval was chosen over an OS-clipboard-changed signal.
- **Confidence:** Medium — likely fine in practice given single-instance assumption, but the contract is invisible to a future maintainer.
- **Suggested alternative:** Document the assumption ("single-instance, single watcher thread") at the top of `watcher.rs`, or replace the global with a `Mutex<Option<String>>` carrying the hash to skip (so duplicate calls don't collide).

---

### F14. README's local-test instructions duplicate CLAUDE.md and will drift

- **What:** README:85-104 ("Full test suite") restates commands that already appear in CLAUDE.md (`cargo test --workspace --exclude snapper-keeper-app --exclude snk-updater`, `pnpm typecheck`, etc.). The README adds Windows-specific advice (`__COMPAT_LAYER=RunAsInvoker`) that CLAUDE.md *doesn't* have, but CLAUDE.md "Tauri 2 gotchas" (lines about UAC detection) is also not in the README. Two docs, two slightly different sets of rules.
- **Where:** README:85-104; CLAUDE.md "Code conventions" + "Tauri 2 gotchas" sections.
- **Why it matters:** The audience is different (humans vs Claude), but the *content* overlap is high and will diverge. When a contributor follows the README and hits the UAC issue, they get the workaround. When Claude follows CLAUDE.md and hits the same issue, it gets the alternative (rename test binary). Both are valid, but a maintainer reading either doc in isolation gets half the picture.
- **Confidence:** Medium — this is a "future drift" concern more than a present bug.
- **Suggested alternative:** Make one canonical (likely CLAUDE.md for technical detail, README for entry-point flow) and have the other link to it. Or pull the shared content into `docs/dev-setup.md`.

---

## Summary

The codebase is **mostly maintainable** but with a tier-2 problem: the *contracts in CLAUDE.md are aspirational, not enforced*. Of seven crates, two skip the typed-error rule (snk-ocr, snk-updater), four reach into another plugin's `::plugin::` module path, one file exceeds the 500-line ceiling, and one feature claim in the README (configurable eviction limit) isn't true. The bones are good — plugin separation is clean at the crate-graph level, `snk-library` really does own persistence, the IPC surface is small and discoverable, and the spec+plan docs are genuinely useful. But the README undersells gotchas (Mac tesseract, Linux scope), two whole TS packages are dead code from the consumer side, and a few unforced errors (hardcoded `to: 4` in `Migration`, blank path in `From<io::Error>`, `mem::forget` copy-paste in 5 test helpers) suggest the next contributor will inherit small papercuts repeatedly. A 1-2 day cleanup pass before the first public release would meaningfully reduce surprise for the first wave of external readers.
