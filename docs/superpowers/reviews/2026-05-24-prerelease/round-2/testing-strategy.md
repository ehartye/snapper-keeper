# Testing Strategy — Round 2

Cross-pollination. Round 1 is locked.

Lens: every finding from the other three perspectives gets re-read through
"would a test have caught this, and if so what test?" — because that's the
only structural fix for "we keep shipping bugs of shape X."

---

## Reactions

### R1 — to Adversary F1 (stored XSS via FTS snippet + `dangerouslySetInnerHTML`)

This is the single most damning finding from the testing lens. Not just
because the bug exists, but because **the test suite was structurally
incapable of catching it**: the SearchBar test
(`app/src/windows/library/SearchBar.test.tsx`) presumably exercises the
component with *clean* mocked snippets, not adversarial ones. The whole
class of "what does this component do when given hostile input" is absent
from the suite.

The missing test is a security regression test in
`SearchBar.test.tsx`:
```ts
it('does not execute scripts embedded in FTS snippets', async () => {
  mockedInvoke.mockResolvedValue([{
    kind: 'capture',
    id: '1',
    rank: 0,
    snippet: '<img src=x onerror="window.__pwned=true">',
  }]);
  renderWithQuery(<SearchBar />);
  await screen.findByText(/img/i);
  expect((window as any).__pwned).toBeUndefined();
});
```

Three-line test, would have failed today, would block the regression
forever. **This is the strongest argument I can make for security-shaped
unit tests at the boundary between SQLite text columns and React render
calls.** The same shape catches Adversary F15 (annotation state
unvalidated) and partially F13 (capture growth — at least if rendered
through the same path).

Connecting to my Round 1: F1 here adds a 15th issue to my list. The test
gap I flagged in my F1 (no E2E) only catches behavioral regressions; this
catches *security* regressions, which the design's manual checklist (§10.5)
also doesn't include. The manual checklist needs a "search a hostile
fixture" line, and the unit suite needs a `dangerouslySetInnerHTML`
forbidden-pattern test (could even be an ESLint rule).

### R2 — to Adversary F5 + Operator #2 (PRIVACY.md claims that don't match code)

Both perspectives flagged the same gap from different angles. From a
testing lens this is fascinating: there is **no test of any kind that
verifies the PRIVACY.md commitments map to runtime behavior**. The
manual checklist (`docs/superpowers/specs/...:685-694`) lists capture +
clipboard + multi-monitor scenarios but says nothing about "verify each
PRIVACY.md sentence corresponds to observable code." Three sentences in
that doc are now known-false:

1. "You can disable update checks in Settings." (no setting exists)
2. "The Microsoft Store edition makes zero network requests." (no edition exists)
3. (Adversary F2 / Operator unstated) Sensitive clipboard items not stored — schema-only.

This suggests the document needs a CI step:
`scripts/verify-privacy-claims.sh` that greps PRIVACY.md for each
quoted promise and asserts a code path exists. Trivial, prevents
regulatory drift.

### R3 — to Adversary F7 + Operator #6 + Operator #7 (updater rollback / kill-switch / first-tag-must-be-stable)

Three perspectives converged on "the updater has no recovery story."
From testing strategy: **none of these three failure modes can be
unit-tested today**. They can only be caught by:

- A live test against `/releases/latest/` after each tag (catches Op #6 —
  the prerelease-tag trap fires at release time, not earlier).
- A chaos test: spin up a fake `latest.json` server in CI, point a local
  build at it, exercise "what if the server says version is 0 (downgrade)",
  "what if signature is invalid", "what if download succeeds but version
  is on the kill-list".
- An "install old, upgrade, verify" smoke (my Round 1 F2's suggested
  nightly job).

These three findings collapse into one architectural fix: **the updater
needs a `MockUpdater` trait that the production code uses behind a
feature flag**, so tests can drive every state including the unhappy
ones. Right now `do_update_check` reaches `app.updater()` directly
(`crates/snk-updater/src/plugin.rs:61`) with no seam.

This adds urgency to my Round 1 F2 (auto-updater untested) — it's
not just "no end-to-end test" but "no architectural seam for testing."
Both fixes need to happen.

### R4 — to Adversary F12 (no Tesseract sandboxing) + Operator #4 (Mac OCR missing)

Adversary worries about CVE-in-Tesseract; Operator worries about missing
Tesseract on Mac. The testing lens connects them: **there is exactly one
integration test for Tesseract** (`crates/snk-ocr/tests/integration_test.rs`)
and it (a) skips silently when Tesseract is missing, and (b) only feeds
a blank image (per my Round 1 F10).

This means:
- The Adversary's "malicious PNG triggers Tesseract CVE" failure mode
  would never be detected because we never feed Tesseract a fuzz corpus
  or known-pathological images.
- The Operator's "Mac users have no Tesseract" failure mode is masked
  because the test prints "SKIP: tesseract not installed" and exits 0 —
  which is exactly the production behavior the Operator wants flagged
  as a bug.

The right move: fail-loud, not skip-silent. The CI matrix should install
Tesseract on *every* platform (including macOS), and the integration
test should `panic!("tesseract is required for tests")` when missing.
Then the Operator's release blocker becomes self-evident at CI time
instead of post-ship.

### R5 — to Maintainer F2 (cross-plugin `LibraryState` reach-in) + Maintainer F3 (typed-error rule violations)

These are testability gaps disguised as style violations. The reason
nobody enforced "import from crate root" or "typed errors only" is
because **CLAUDE.md rules have no test coverage**. There is no
`tests/clippy_rules.rs`, no compile_fail tests, no lint script. The
rules are aspirational because they're documented, not asserted.

Concrete fix: add a `scripts/check-claude-md-rules.sh` that runs:
- `grep -rn 'snk_library::plugin::' crates/` → must be empty
- `grep -rn 'Result<.*, String>' crates/*/src/plugin.rs crates/*/src/commands.rs`
  → must be empty (with a doc-comment escape hatch like `// allow string error: status-only`)
- `find . -name '*.tsx' -newer .gitkeep | xargs wc -l | awk '$1>500'` → must be empty

This gives the lints teeth. Right now the rules drift because there's
no failing build when they're violated. **From a testing lens, every
CLAUDE.md rule that matters should have either a clippy lint or a CI
script — otherwise it's a wish.**

### R6 — to Maintainer F1 (dead `@snk/ocr` and `@snk/updater` TS packages)

The testing dimension Maintainer didn't name: **the test files for those
packages are 100% wasted CI cycles** and worse, they actively *mislead*.
`packages/snk-updater/src/index.test.ts` (which my Round 1 read) asserts
that `checkForUpdate()` invokes `plugin:snk-updater|check_for_update` —
which is *technically* still true on the Rust side but **no part of the
production app ever does this**, because per Maintainer F1 the entire
updater flow is Rust-internal.

The tests pass green. They tell you the bindings work. They tell you
nothing about whether the *real* updater path works (which is the
Rust-internal `do_update_check` plus the tray menu invocation at
`app/src-tauri/src/main.rs:194-197`). This is the platonic example of
"high coverage, low confidence" — a test suite that exercises dead code.

Connecting to my Round 1 F13 (coverage CI excludes the riskiest files):
delete-the-package fix from Maintainer F1 would *also* slightly raise
the meaningful coverage number, since the unused TS bindings inflate
the denominator. Good test hygiene = delete dead tests with the dead
code.

### R7 — to Operator #1 (no file-based logging)

This is the single biggest blocker for *post-release* testing, which I
under-weighted in Round 1. Without logs, every user-reported bug becomes
"can you reproduce it?" — which is just an unfunded test request. With
logs, the user *is* providing the test fixture (their session) for free.

The testing-strategy implication: **a release that ships without log
appenders is shipping without a feedback loop**. The whole test suite,
the whole CI matrix, the whole manual checklist exists in the
pre-release world. Post-release, the only signal is GitHub issues, and
those issues are unactionable without logs. We're shipping with a
diagnostics gap that makes future testing *impossible*.

This also connects to Operator #13 (no in-app version display): without
a version number, the user can't tell you what to fix even when they
have logs.

### R8 — to Maintainer F8 + my Round 1 F7 (migration `from:0, to:4` is a lie)

Maintainer caught this from the code-readability angle. From testing:
**every migration test asserts table existence, not migration semantics**
(my Round 1 F7). The hardcoded `to: 4` means a future contributor adds
V005, ships it, and the test still passes — because the test never
inspects the `to` value, just runs `migrate()` to success.

Combined fix: add a test that asserts
`migrations().to_latest(conn).is_ok() && current_version(conn) ==
migrations().count()`. Forces the constant in `migrate.rs:21` to track
the actual latest version, OR catches the drift in CI. Cheap.

### R9 — to Operator #5 (migration backup) + Adversary F2 (cleartext DB)

Operator wants pre-migration backups for *recoverability*. Adversary
wants encryption for *confidentiality*. The intersection — and this is
new — is **there's no test that the DB is even readable after a
migration completes**. The migration tests at
`crates/snk-library/src/migrate.rs:30-91` assert *tables exist*; they
don't assert that data inserted under v_N is queryable under v_N+1.

Real-world failure: V005 renames a column, the data lives, queries
still pass... but the deserializer panics on the new column name.
This is migration-shaped, but the only test it would fail is a
fixture-based "load v1 schema with realistic data, run all migrations,
do a representative query" test. None exists.

Both Operator's backup fix and Adversary's encryption hardening would
benefit from this test scaffolding being in place first — otherwise
adding either feature ships untested.

### R10 — to Operator #10 (clipboard watcher dies silently)

I had this in my Round 1 F14. Operator's framing — "the most-used
feature dies and there's no log" — adds urgency. The testing
extension: there is no test that the watcher's *health* is observable
at all. There's no `clipboard_status` command (the OCR plugin's
`ocr_status` exists at `crates/snk-ocr/src/plugin.rs:14-17` but
hardcoded; the clipboard plugin doesn't have an equivalent). So even
if you write a test that verifies the watcher dies cleanly on
`Clipboard::new()` failure, there's no way to test what the
*application* does in response (because the application has no signal).

Architectural test gap: every long-running task (clipboard watcher,
OCR queue, updater interval) should expose a heartbeat that tests can
poll. Otherwise "is this thread alive" is not even testable, let alone
asserted.

### R11 — to Adversary F15 (PNG validation missing) + Maintainer F4 (configurable eviction misleading docs)

These look unrelated until you ask "what does the test suite say
about each?" Adversary F15: `save_annotation` writes arbitrary bytes;
the test at `crates/snk-annotate/src/commands.rs` (none currently)
would catch this trivially with `assert!(write_atomic rejects
non-PNG)`. Maintainer F4: the README claims a configurable limit;
a test asserting "set `clipboard.history_size = 50`, watcher caps at
50" would fail today because the watcher ignores the setting.

The shared shape: **tests that assert documented behavior against
actual code would catch both bugs**. We have tests that assert
implemented behavior (good) but no tests that assert
documented/promised behavior. Every line in PRIVACY.md, README, and
spec §10.5 should have a corresponding test or be marked as
known-aspirational.

### R12 — to Operator #9 (unbounded OCR queue)

Testing-strategy take: the queue is unbounded (`mpsc::unbounded_channel()`
at `crates/snk-ocr/src/queue.rs:21`), and the only test of the OCR
*system* is the sidecar-integration test against a blank image. There
is no test that:

- A burst of 100 captures all eventually get OCR'd.
- Quitting mid-queue leaves captures without OCR text (per Operator
  #9, which is exactly the gap).
- Re-launch picks up captures missing `ocr_text` rows.

A simple "fill queue, simulate quit, restart, assert all captures have
text" test would surface Operator's resumption gap as a *failing test*
before it surfaces as a user complaint. Today the failure mode is
silent and untestable.

---

## Tensions

### T1 — Adversary F1 (XSS via snippet) vs my own Round 1 F1 (need E2E layer)

**Tension:** I argued for E2E via `tauri-driver` as the headline fix.
Adversary's stored-XSS finding suggests **unit-level security tests**
(specifically: hostile-input fixtures fed through individual React
components) would catch a real exploitable bug *now*, with no
infrastructure investment.

**Resolution:** Both are needed but the order matters. Security
fuzzing at the component level is cheaper and catches a concrete bug
today; E2E is a multi-week project. Reorder priority: fix F1 first
with a unit test against `SearchBar.tsx`, then build E2E for the
class of bugs E2E uniquely catches (window-orchestration, hotkey
registration, paste-into-target). My Round 1 over-weighted the
infrastructure fix.

### T2 — Maintainer F1 (delete unused TS packages) vs my Round 1 F6 (TS bindings test command names but not Rust acceptance)

**Tension:** Maintainer argues to *delete* the unused `@snk/ocr` and
`@snk/updater` TS packages. I argued (Round 1 F6) that the TS-binding
tests are missing the Rust-acceptance side of the contract.

**Resolution:** No real conflict. Delete the *dead* TS packages
(updater, ocr — never consumed). For the *live* TS packages
(`@snk/library`, `@snk/capture`, `@snk/clipboard`, `@snk/annotate`),
keep the bindings, keep the existing tests, AND add the Rust-side
contract test I proposed. The two findings are about different
packages.

### T3 — Operator #14 (universal binary halves macOS CI) vs my Round 1 F1 (need MORE E2E in CI)

**Tension:** Operator wants `--target universal-apple-darwin` to
halve macOS CI time and ship one bundle. I want a *nightly* E2E run
that exercises both arch slices. Universal-binary collapses the
testable surface — one bundle to test instead of two.

**Resolution:** Operator's fix is correct for *release* artifacts.
For *testing*, the per-arch CI build is actually useful because it
exercises both code paths against the platform's native runner.
Compromise: matrix the test job per-arch (current behavior), but
collapse the release artifact upload to one universal bundle.

### T4 — Adversary F7 (no rollback / user confirmation before install) vs Operator #6/#7 (kill-switch protocol)

**Tension:** Adversary wants a user-confirmation dialog before
`download_and_install` fires; Operator wants automatic-deploy to
hotfix as fast as possible. These are in direct conflict — every
manual confirmation step is a deployment-velocity loss.

**Resolution from a testing lens:** the deeper bug is that **neither
behavior is tested**. Today the updater auto-installs with no
confirmation; if we add confirmation, we have to test the
confirmation. If we don't, we have to test the auto-install + the
rollback path. Either choice needs *more* test coverage than exists.
The architectural fix from R3 (MockUpdater trait) supports both
designs equally; ship the seam first, decide UX second.

---

## New Insights

### NI1 — There is no "contract test" between PRIVACY.md / README / design spec and the code

Three of four perspectives independently flagged document-vs-code
drift (Adversary F5, Operator #2, Maintainer F4/F5/F12). From a
testing lens this isn't four separate findings — it's *one missing
test category*. Every user-facing document makes assertions; none of
those assertions are verified by CI.

Concrete proposal: a `scripts/verify-docs.sh` that:
- Parses bullet points from `PRIVACY.md` and asserts code paths exist
  for each (per R2 above).
- Parses the feature list from `README.md` and asserts each feature
  has at least one integration test.
- Parses the manual release checklist from
  `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md:685-694`
  and reports which lines have automated coverage.

This category would have caught my Round 1 F4 (sensitive clipboard
schema-only), Adversary F5, Operator #2, Maintainer F4 in one pass.

### NI2 — Every long-lived background task is a test gap by construction

Looking at the codebase through Round 2 lenses, the failure modes
keep clustering around **background tasks the test suite cannot
reach**: clipboard watcher (Operator #10, my F3/F14), OCR queue
(Operator #9, my F3), updater interval loop (Operator #8, my F2),
hotkey registration listener (CLAUDE.md gotcha section), the
`tauri::async_runtime::spawn` blocks in plugin setups.

The systemic fix is that **every `tauri::async_runtime::spawn` and
`std::thread::spawn` in the codebase needs an extracted, testable
inner loop**. The pattern should be:

```rust
// Untestable today:
tokio::spawn(async move { worker(rx, db, root).await });

// Testable:
pub async fn worker_step(...) -> Result<()> { ... }
tokio::spawn(async move {
    while let Some(job) = rx.recv().await {
        worker_step(job, &db, &root).await
    }
});
```

Then `worker_step` is unit-testable in isolation. No current crate
follows this pattern. Across the seven plugins there are at least
five background tasks, all of them currently untestable, and the
operator failure modes (silent death, queue drain, interval drift)
are exactly the bugs this pattern surfaces.

### NI3 — The "manual checklist" in design §10.5 has zero connection to anything testable

This is a NEW realization triggered by reading all three perspectives
together. The spec §10.5 manual checklist (lines 685-694) lists 8
manual gates. **Not one of them is referenced anywhere else in the
codebase.** There's no issue template that prompts the releaser to
run it, no CI step that records which gates were checked, no
changelog requirement. It's a list in a spec doc that the releaser
must (a) remember to read and (b) honestly complete.

Operator finding #6 hints at this: the existing tag history shows 13
`-test` tags, suggesting the team is using real releases as
test-infrastructure because the test-infrastructure is incomplete.
The manual checklist's existence + the lack of any enforcement
mechanism = the manual gates are aspirational.

Concrete proposal: convert each line into a GitHub Issue template
("Pre-release manual smoke test #N") and require all 8 to be closed
before the tag pipeline runs. Or: add a `scripts/release-readiness.sh`
that interactively walks through the checklist and writes a signed
attestation file (`.release-checks/v{VERSION}.json`) that the release
workflow refuses to run without. Either way the manual gates need
*enforcement*, not just *existence*.

### NI4 — The codebase has no test for "what happens when a plugin's setup() panics"

Plugin `setup` closures are pervasive: every `init()` in every plugin
runs setup code. If `snk-library`'s `setup` panics (DB locked,
migrations fail, disk full), the app crashes silently on startup
because there's no panic boundary around plugin initialization.
Operator's #1 (no logging) means the user sees the app icon flash and
disappear with zero diagnostics.

**No test exists for the plugin-init failure cases.** This is a
testability gap because Tauri's plugin model doesn't easily expose
init failures to tests — but it's also exactly the kind of thing that
breaks at the first non-trivial user environment. Combined with
Operator's findings #1 and #5, a migration-failure on a real user's
machine becomes an unrecoverable, unlogged, silent crash.

### NI5 — `cargo audit` / `pnpm audit` belong to testing strategy, not just operations

Operator listed dep-audit as finding #11 (an operations concern). From
a testing-strategy lens this is actually **a continuous test of
external trust assumptions**. The Rust dep tree pulls `arboard` (raw
OS clipboard), `xcap` (raw screen capture), `rusqlite` (raw SQL),
`reqwest` (HTTP for updates), all of which have CVE histories. The
test suite assumes these are correct; `cargo audit` is the test that
verifies the assumption still holds.

It belongs in the test pyramid as the "external dependencies" layer.
The design's §10.1 pyramid (E2E / Integration / Unit) is missing this
4th layer. **The spec needs updating: Unit / Integration / E2E /
Supply-chain.**

### NI6 — The clipboard-watcher pattern (`SKIP_NEXT` global) is untestable AND uncatchable in review

Maintainer F13 noticed the `SKIP_NEXT` global. From testing: this is
the canonical example of a contract that exists nowhere except in
runtime ordering. No test can verify "the watcher skips the next
write after `mark_skip_next()` is called" because:

1. The watcher polls every 500ms, so the test would be flaky.
2. The flag is a single global `AtomicBool` — concurrent calls
   stomp on each other.
3. There is no observability hook (event, log, return value).

Testability fix that doubles as a correctness fix: replace
`AtomicBool` with `Mutex<HashSet<String>>` of content hashes to skip,
expose `pending_skips()` for tests, and assert in a unit test that
after `mark_skip_next("abc")` and a poll cycle, the watcher does NOT
re-insert the row with hash "abc". Two-line refactor, removes the
race, adds the test.

### NI7 — Adversary's F1 + F6 + F9 chain is testable as a single test

Adversary chained F1 (XSS) → F6 (popup has full IPC) → F9 (XSS can
toggle autostart). From testing: this whole chain becomes ONE
property-based test:

> For any string `s` containing HTML/JS payloads, after rendering `s`
> via the FTS snippet path, `window.__TAURI__` should be undefined OR
> the test environment's IPC mock should not have received any
> invocation.

A single such test, run against a corpus of XSS payloads from a
well-known list (OWASP XSS cheat sheet), would catch all three. It
also catches future regressions in any of the three links (someone
re-introduces `dangerouslySetInnerHTML`, someone widens capabilities,
someone exposes a new dangerous command).

This is the kind of test that the current suite structure
(`renderWithQuery` + mocked invoke) actively supports — adding it
is *cheap*. The infrastructure exists; the discipline of "test
hostile inputs, not just clean ones" is what's missing.

---

## Summary of cross-pollination

Reading the three perspectives shifted my Round 1 priorities:

1. **Security-shaped unit tests are cheaper and catch more bugs today
   than the E2E layer I prioritized.** Adversary F1 alone justifies a
   testing-strategy push toward hostile-input fixtures.

2. **Documentation-as-test is a missing category.** Three of four
   perspectives found doc-vs-code drift; the systemic fix is a CI
   step that verifies each documented promise.

3. **Background tasks are the dominant untested surface.** Every
   spawn point is a future operator incident (Operator #1, #9, #10
   confirm this). The architectural pattern (extract `_step`
   functions) is a 1-week refactor that unlocks 5+ years of test
   coverage.

4. **Coverage of dead code (Maintainer F1) actively misleads.** My
   Round 1 F13 understated this: removing the dead bindings is good
   not just for maintainability but for trustworthy test signal.

5. **The manual release checklist is unfunded test debt.** Operator
   #2/#6/#7 + my Round 1 F1 all hinge on the same root cause: the
   "manual smoke" gates have no enforcement mechanism. Either
   automate them or make their incompleteness explicit per release.
