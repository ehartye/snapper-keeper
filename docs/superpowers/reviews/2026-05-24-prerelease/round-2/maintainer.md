# Maintainer Perspective — Round 2 (Cross-Pollination)

Reading the other three Round 1 outputs through the maintainability lens: which of their findings imply hidden coupling, missing docs, or contract drift that a future maintainer will trip over? Where do my findings conflict with theirs? What did I miss that another perspective surfaced?

---

## Reactions

### R1. Adversary F2 + Testing F4 + Operator F2 all converge on a *broken-by-default contract*

Three perspectives independently flagged the same artifact from three angles:
- **Adversary F2:** The `sensitive` column on `clipboard_items` is dead — never written.
- **Testing F4:** The schema field + the `clipboard.app_blocklist` setting key both exist with zero readers; the tests *exercise the column* but never assert end-to-end behavior.
- **Operator F2:** PRIVACY.md promises "you can disable update checks in Settings" — also fictional.

My Round 1 F4 noted the README's "configurable eviction limit" lie and the orphaned `clipboard.history_size` setting key tested at `settings.rs:76`. **Pattern: this codebase has a class of "stub-promise" features — schema fields, setting keys, doc claims that name a feature without wiring it.** A maintainer six months in will discover these only by user complaint or by audit. They are uniquely hostile to onboarding because they look like real APIs in the IDE.

**Maintainer reaction:** The fix isn't only per-instance. The repo needs a *single inventory* of "promised but unimplemented" features and either ship-or-strip pass before v1. The `tests/migration_integration.rs`-style schema-snapshot approach Testing F7 recommends would also catch new orphan columns going forward — pair it with a grep-based "every settings key referenced in tests must be read by production code" lint.

### R2. Operator F1 (no file logging) intensifies my F7 + F8

Operator notes that `tracing` writes to stdout, but the Windows release has `windows_subsystem = "windows"` so stdout is *discarded*. My Round 1 F7 (`From<io::Error>` blanks the path) and F8 (`Migration` error hardcoded "from 0 to 4") become *invisible* in a packaged build — the only consumer of those error fields is a `tracing::warn!` or `tracing::error!` macro that vanishes into the void.

**Maintainer reaction:** The maintainability cost of my F7/F8 was "logs become useless." Operator's F1 raises it to "logs do not exist." Combined: a packaged build that hits a migration error returns the IPC envelope with `{kind:"migration", from:0, to:4, recoverable:false}` to the frontend, and that's the *only* surviving artifact of the failure. Any future contributor debugging a migration crash via user-supplied reproductions has nothing except the IPC trace.

**Implication:** Fix F1 (file logging) *first*, then circle back to F7/F8. Without F1, fixing F7/F8 cosmetically improves a string nobody sees.

### R3. Adversary F1 (XSS via FTS snippet) makes my F2 (LibraryState path-import) infinitely worse

Adversary F1 establishes that the library window has the *full plugin command surface* available via `window.__TAURI__`. My F2 noted that four crates import `snk_library::plugin::LibraryState` via the internal module path — at the time, my concern was "next refactor will break four importers." Adversary's lens reframes this: the four importers represent *every cross-plugin call into persistence*, which is also *every command the XSS payload can reach*.

**Maintainer reaction:** The `LibraryState` import is the connective tissue of the entire app. If I were the maintainer refactoring `snk-library`, I would now know: any change to `LibraryState`'s public surface needs a synchronized review of (a) the four importers, (b) the capability files that grant access to those commands, and (c) the frontend that invokes them. CLAUDE.md captures none of this — the architectural rule "no plugin imports another plugin's internals" is enforced by no test and is the only documentation of the constraint.

**Implication:** Add a section to CLAUDE.md titled "What changes when `LibraryState` changes" listing the four importers + capability + frontend impact — or better, make `LibraryState` `#[doc(hidden)]` and ship a stable `LibraryHandle` newtype.

### R4. Operator F3 + F15 fully validate my Round 1 hunch on capability fan-out

I noted in Round 1 F9 that theme additions require touching 4-5 places across Rust + TS. Operator F3 + F15 + Adversary F6 + F9 reveal the same pattern on **capabilities**: the design (`spec §8.3`) called for `clipboard-popup.json` to exist as a separate capability file with a *minimum* permission set — it doesn't. Instead `default.json` is a single grant for all 6 windows.

**Maintainer reaction:** This is exactly the kind of "implicit knowledge in the spec, not the code" failure that bites onboarding. A new contributor reading `capabilities/default.json` sees a flat list and assumes Tauri capabilities are inherently flat. They have to read the spec doc to learn it was a deliberate (then-abandoned) design choice. The plan-as-source-of-truth pattern (per CLAUDE.md) is supposed to prevent exactly this drift — the plan file for whichever phase made this trade-off should have been edited to record the divergence.

**Implication:** Audit the design doc for *every* place it says "X is enforced by the framework" and check the framework actually enforces it. Where it doesn't, either fix the code or update the spec. Both my F9 (themes) and the joint adversary/operator capability finding share this root cause.

### R5. Testing F12 (wrapped `Library(LibraryError)` wire shape) extends my F3

My F3 noted CLAUDE.md's "errors cross IPC as typed enums" is violated in 3 surfaces (`snk-ocr`, `snk-updater`, the `format!`-mapped `CaptureError::Os`). Testing F12 reveals that even the *honored* cases break the contract at a different layer: `ClipboardError::Library(LibraryError)` serializes as `{"kind":"library","0":{...wrapped...}}` and no test confirms the frontend can pattern-match on `err[0].kind === "not-found"`. The TS side has *no* error types at all (`packages/*/src/types.ts` defines structs but no error enums).

**Maintainer reaction:** The typed-error contract is *more* broken than I thought. Even where the Rust side correctly defines enums with `#[serde(tag = "kind")]`, the consumer (TS) treats every invoke rejection as opaque `unknown`. So the "typed contract" exists only on the producer side — a one-sided handshake. A maintainer adding a new `LibraryError` variant has no compile-time signal that the frontend will treat the new variant the same as every other.

**Implication:** Adopt `ts-rs` or `specta` (Testing F12's suggestion) to generate TS types from Rust enums. Without that, the IPC contract drifts silently and "typed errors" is aspirational on both ends.

### R6. Operator F14 (no universal macOS binary) is a docs/plan drift case too

Operator notes the design committed to a universal macOS binary; the release pipeline ships two separate per-arch builds. This is the same pattern as my F12 (Linux in CI but not the shipping target) and Operator's own F4 (Mac tesseract not bundled despite the spec). **Spec → code drift is systemic, not isolated.**

**Maintainer reaction:** The phase plans live in `docs/superpowers/plans/` and the spec lives in `docs/superpowers/specs/`. Neither has a "deviations from plan" log. The CLAUDE.md "plan-as-source-of-truth pattern" rule says the *plan* gets edited when implementation diverges — but no such edits appear in the phase 7 plan covering the macOS arch split or the tesseract bundling decision. A new contributor reading the spec will trust statements that aren't true.

**Implication:** Add a "Divergences" section to each phase plan that names every spec claim the phase chose not to honor. Or downgrade the spec's stronger statements to "intended; see phase-X plan for actual shipped behavior."

---

## Tensions

### T1. Testing F1 (need E2E) vs. CLAUDE.md's "smoke is manual on Windows"

Testing strategy says: ship at least one tauri-driver smoke test per OS before v1. CLAUDE.md notes smoke testing on Windows requires an interactive desktop session — incompatible with GitHub Actions Windows runners, which are non-interactive. The team has effectively chosen "manual smoke, no E2E" because of the platform constraint.

**My take from the maintainer lens:** Both perspectives are right and need to be reconciled in docs. The CLAUDE.md note today reads "this is a known limitation" without acknowledging it's *the entire reason* the E2E layer doesn't exist. A new contributor reading CLAUDE.md doesn't realize that "no smoke on SSH" is an architectural blocker, not a minor caveat. The honest framing: "We can't run E2E in CI; therefore manual smoke per release is the contract; therefore here is the manual checklist." If the checklist (spec §10.5) isn't run pre-tag, the safety net is *zero*. The drift between Testing's "this should exist" and Operator's "manual checklist won't scale" and CLAUDE.md's "known limitation" needs unification — they're all describing the same hole from different sides.

### T2. Adversary F7 (no auto-update prompt) vs. Operator F7 (need a kill switch)

Adversary wants the auto-updater to prompt before install ("auto-install is hostile for a share-friendly side project"). Operator wants the auto-updater to be *more* automatic so a hotfix can force-push a fix without user action. These conflict.

**My take:** They're describing different lifecycle states. Pre-incident (normal release): Adversary is right — prompt. Post-incident (yank): Operator is right — push. The current code does neither well: it auto-stages (Adversary's concern) but has no kill switch (Operator's concern). The right resolution is a *two-mode updater* — normal mode prompts, "critical" releases flagged in `latest.json` bypass prompt. Neither perspective alone gets this; the synthesis does.

### T3. Testing F4 (sensitive clipboard) vs. Adversary F2 (sensitive clipboard)

Both flag the same gap but recommend opposite things. Testing F4: "either implement source-app detection with a test, OR remove the schema column + setting + spec line." Adversary F2: "implement OS-level exclusion-format honoring + heuristics + a real blocklist before public release."

**My take from the maintainer lens:** Testing's "ship-or-strip" is the more honest near-term posture, but Adversary's full implementation is what the *spec promised*. A new maintainer encountering this in 6 months will not know which choice was made — so whichever option the team picks, it has to be recorded in the spec + PRIVACY.md *together*. The current trap is leaving the schema column in while removing the doc claim (or vice versa).

---

## New Insights (Triggered by Other Perspectives)

### N1. The `_app: AppHandle` parameter in commands is a *type-system bug-bait* (triggered by Adversary F14)

Adversary noted that most commands accept `_app: tauri::AppHandle<R>` but don't read it, so per-window authorization checks are impossible at the handler. I missed this entirely in Round 1.

**Maintainer angle:** The leading underscore is Rust convention for "intentionally unused." Every command-author who writes `_app` is signaling "I know this is unused, that's fine." A future maintainer who *wants* to use it for window-label checks (per Adversary's suggestion) has to flip every underscore — 14+ commands across 4 crates — and risk missing one. A safer pattern: omit the parameter entirely from commands that don't use it. Tauri 2 supports zero-arg-from-state-only commands. The current `_app` placeholders are a "we anticipated needing this but didn't wire it" smell that will rot.

### N2. UUIDv7 monotonicity is an undeclared invariant that *several* code paths depend on (triggered by Testing F5)

Testing F5 notes that `event.payload().trim_matches('"').to_string()` in `snk-ocr` assumes IDs don't contain quotes — and UUIDv7 is the only thing keeping this true. Maintainer angle: I count *four* places that rely on UUIDv7's properties without naming the dependency:

1. `crates/snk-capture/src/commands.rs:51-53` — preview cache token uses UUIDv7 *because* it's monotonic ("two consecutive calls always differ"). Comment says so.
2. `crates/snk-ocr/src/plugin.rs:43` — assumes the emitted payload is a valid Rust-string-quoted id.
3. `crates/snk-library/src/captures.rs:34-36` — `Uuid::now_v7()` for ordering — the implicit assumption "newer captures sort later by id" is used nowhere explicitly but feels load-bearing.
4. `crates/snk-annotate/src/commands.rs:52` — pre-generates the new id so file name + DB row match.

**Maintainer angle:** Switching away from UUIDv7 (because of, say, a future v8 RFC) silently breaks (1), (2), and (3). A maintainer who tries to centralize id generation into a helper function will have to discover each invariant on a case-by-case basis. Add a doc comment in `snk-library/src/lib.rs` declaring "Capture IDs are UUIDv7; consumers may rely on monotonicity and the absence of HTML/JSON-special characters." Without that, this is implicit knowledge.

### N3. The `app/dist/` directory is checked into the repo (triggered by Operator F1's investigation pattern)

While verifying Operator's claims I noticed `ls` of the repo root showed `app/dist/` exists alongside `app/src/`. This is the Vite build output and is normally gitignored. If it's actually tracked (I didn't `git status` to confirm), then every PR that touches the frontend will produce noisy diffs in `dist/`. A new contributor running `pnpm tauri dev` will see modified `dist/` files and not know whether to commit them.

**Maintainer angle:** Quick to verify and quick to fix; the impact is high contributor-confusion-per-PR. If it's gitignored and just a local artifact, nothing to do. Worth a one-liner check.

### N4. The auto-updater state machine has no `Cancelled` or `Skipped` variant (triggered by Adversary F8 + Operator F7)

Adversary wants signature failures treated as terminal; Operator wants a kill switch. Both imply the state machine needs a transition the type doesn't currently express. Looking at `UpdateStatus` (`crates/snk-updater/src/plugin.rs:11-19`): `Idle | Checking | Available | Downloading | Ready | Error`. No `RejectedBySignature`, no `SuppressedByPolicy`, no `Skipped`. So both improvements force a type extension *and* a frontend update — but neither has been thought through together.

**Maintainer angle:** This is the kind of state machine that grows messily. If Adversary's "signature rejection is terminal" and Operator's "policy can suppress updates" land in different PRs, they will collide on the type and one will rebase the other. Recommend designing the state machine *once* with both transitions, then implementing in two PRs that share the type definition.

### N5. The capability schema bug is *cross-cutting* — not three issues, one issue (triggered by Adversary F6 + F9, Operator F3 + F15)

Four findings (Adversary F6, F9; Operator F3, F15) all describe variations of "default.json grants too much to too many windows." The aggregate impact is bigger than any one alone: every plugin that adds a new permission widens the surface for every window. From the maintainer lens: when a new plugin ships, the contributor adds `snk-newplugin:default` to the single `default.json` because that's the only template they see — *and they're right that's the easiest path*. The current structure makes "do the right thing" expensive.

**Maintainer angle:** The fix has to make per-window capability files the **default template**, not the exception. Add a `capabilities/_template.json.example` with comments explaining when a window should get a permission. Update CLAUDE.md's "Architecture rules" with rule #7: "Every new plugin permission must be granted only to the windows that demand it. Default to NOT granting; require justification to grant beyond `library` + `settings`."

### N6. The `app/src-tauri/gen/` directory is regenerated every build but its existence drift could break new contributors (triggered by reviewing CLAUDE.md gotchas)

CLAUDE.md notes "`tauri-build` regenerates `app/src-tauri/gen/` on every build. Already gitignored." But it doesn't say *what's in there* or *what to do if it's stale*. A new contributor with a stale `gen/` from a failed build will see cryptic compile errors in regenerated code they can't read. Add a one-liner: "If you see compile errors mentioning `gen/`, run `cargo clean -p snapper-keeper-app` and rebuild."

---

## Closing meta-observation

Reading three other perspectives confirms a single root cause from the maintainer lens: **CLAUDE.md and the design spec are partially aspirational, and there's no enforcement layer**. The plan-as-source-of-truth pattern is documented as a workflow but no review process catches drift. Adversary's findings are mostly *real attacks*; Operator's findings are mostly *missing instrumentation*; Testing's findings are mostly *contract gaps*. Mine were *contract violations and doc drift*. All four perspectives end up at the same place — the documents describe a system more rigorous than the code is. The single highest-leverage maintainability improvement is to add a pre-release checklist that does for every spec claim what Testing F4 does for sensitive clipboard: "is this implemented?  is it tested?  if neither, strip the claim or implement the feature." Run it before v1.0.
