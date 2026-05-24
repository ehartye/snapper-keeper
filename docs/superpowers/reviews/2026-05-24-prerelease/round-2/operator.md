# Operator perspective — Round 2

Cross-pollination. Reading Adversary, Maintainer, and Testing Strategy through the 3am-incident lens. My Round 1 stands as written; this only adds reactions, tensions, and what the other perspectives let me see that I missed alone.

---

## Reactions

### To Adversary F1 — Stored XSS via FTS snippet → full IPC blast radius

The Adversary made the most impactful finding in this review, and it converts several of my findings from "operability gap" to "security incident detection gap." From the operator chair:

- **My finding #1 (no file-based logging) is the reason this incident would never be detected.** If a webview XSS payload exfiltrates the entire clipboard archive or fires `paste_item` into the user's terminal, there is no log on disk that says "weird invoke pattern" or "paste_item called by `library` window 50 times in 3 seconds." The user sees a glitch in their terminal, can't reproduce, files no ticket — and the operator never learns the chain even existed in the wild. **Add an IPC audit log to the panic/log sink I argued for in Round 1.** Every invoke from a non-`settings`/non-`library` window to a destructive command should be logged with the originating window label, timestamp, and command name. That data is non-PII and pays for itself the first time an issue is filed.
- **My finding #7 (no kill switch) gets dramatically worse.** If a user's installed binary is silently compromised by Adversary F1 and the attacker uses the same primitive to flip `autostart` on (Adversary F9), the user can't un-ship the attack. Cutting v1.0.1 with the XSS fix doesn't help anyone whose machine already executed the payload — the persistence + exfiltration is done. **Recommendation: the v1 release notes should include a documented post-compromise reset procedure** (delete app data dir, uninstall, reinstall) and the Settings → About panel I proposed should expose "delete all clipboard history" + "delete all captures" as one-click resets.
- **My finding #6 (latest.json URL semantics) intersects with Adversary F7 (no version pinning).** If a downgrade attack lands an old vulnerable v1.0.0 onto a v1.0.1 user, the user loses the XSS fix and the operator has no signal that this happened — silent downgrade via MITM, no logging, no rollback. Operationally this would manifest as "users mysteriously report old bugs." Without logging, untraceable.

### To Adversary F2 — Clipboard captures everything; sensitive flag is dead

This is the operator's nightmare ticket: *"I lost my entire password vault to this app."* The remediation cost is unbounded — the data is on the user's disk in plaintext, the user has typically no awareness it was captured, and the operator's options for help are zero. Two operational consequences:

- **The DB file becomes a forensic artifact.** Any user who reports a security concern (employer's IT asks "what does this app capture?") now needs the operator to be able to explain exactly what's in `snapper-keeper.db`. Without a tool to inspect it (no "view all clipboard items" debug command, no export), the user has to trust the operator's word. The Adversary's F13 reinforces this — there is no purge-on-uninstall.
- **Combined with my finding #5 (no migration backup):** if a future v1.x adds a sensitivity-detection migration that scrubs historic rows, a migration failure would leave the user in the *worst* state — partial scrubbing, no backup to restore from, no log of what was removed.

### To Adversary F5 — PRIVACY.md claims things the binary cannot do

This validates my finding #2 from a stronger angle. The Adversary correctly framed it as a regulatory/FTC issue, which I only hinted at. From the operator chair: **the moment a user posts a screenshot of the PRIVACY.md text next to a screenshot of the Settings panel showing no toggle, this becomes a HackerNews thread.** Operationally, the first user-reported version of "your privacy policy lies" is essentially unrecoverable — the artifact is permanent on the internet. **Edit-before-tag is materially cheaper than fix-after-ship.** I'd flag this for the team-lead as a *blocker for v1*, not a "fix it in 1.0.1."

### To Adversary F7/F8 — Updater has no rollback, no signature-error escalation

This is the same operational concern I raised in #7 + #8, refined: the Adversary correctly identifies that signature-mismatch errors are stringified into the same `UpdateStatus::Error` channel as network-timeout errors. **From the operator lens, this is the single most-important diagnostic distinction in the entire updater.** Network failures are noise; signature failures are alarms. Treating them the same means the on-call engineer can't even ask the user "did your machine show a signature-error toast yesterday?" — the user has no way to distinguish them.

**Concrete operator addition to Adversary F8:** the file-based log I proposed in my finding #1 should have a separate `security-events.log` file (append-only, never rotated, surfaced in Settings → About). Signature mismatches, capability denials, panic-hook captures all land there. Everything else goes to the rotating log.

### To Adversary F10 — CI actions pinned by mutable tags; supply-chain risk

This is my Round 1 finding #11 (no `cargo audit` / SBOM) viewed from a different angle. I focused on "we can't tell which release shipped which deps"; the Adversary focused on "an upstream compromise injects code into our signed release." Both are correct; both have the same root cause (no integrity gate on the build inputs).

**Operational tie-in:** if a supply-chain compromise lands a malicious dep into a signed release, the operator's first question is "which release? when? who installed it?" — and again, **no telemetry by design** means the operator has no install-base visibility. The GitHub Releases asset download count is the only signal, and it doesn't tell you who is still running an old build vs a compromised one. The operator's recourse is "publish a notice on the README and hope users see it." This argues for a *visible* in-app "your version is X, latest is Y, X has a known issue" surface — which today doesn't exist (my finding #13).

### To Adversary F11 — Chocolatey-installed Tesseract bundled without integrity check

I missed this in Round 1. The operator-incident shape is exactly the SolarWinds template: signed release, trusted upstream, no integrity check — and the blast radius is "Authenticode-signed malware shipped under your cert." The 3am call when this fires is "your installer is being flagged by Defender as a trojan" and the user-side recovery is to revoke the cert and re-issue, which on Azure Code Signing means coordinating with the Azure ops team and the EV/standard cert reissue cycle (days, not hours). **This is operationally the single most expensive failure mode in the entire pipeline.**

### To Maintainer F1 — Dead TS bindings (`@snk/ocr`, `@snk/updater`)

Maintainer-cleanup-worthy, low operator urgency. But the operational tie-in is real: a debug panel in Settings → About that calls `getUpdateStatus()` and `ocrStatus()` from the existing dead bindings is exactly the kind of artifact that pays for itself once. So instead of *deleting* the bindings as Maintainer F1 suggests, I'd argue for *consuming* them in the About panel — turning dead code into a diagnostic surface. Cheap re-purposing.

### To Maintainer F7/F8 — `LibraryError::Io` discards path; `Migration` says "from 0 to 4"

Maintainer F7 and F8 dramatically reduce the value of any logs we do write. If the eventual log file contains "io error at : permission denied" and "migration failed from 0 to 4" for every migration in 2027, the operator can read the log but can't act on it. **These are prerequisites to my finding #1 having operational value.** Adding a log file without fixing F7/F8 means writing more noise to disk.

### To Maintainer F13 — Global `SKIP_NEXT` AtomicBool in clipboard watcher

The race window is 500ms (the poll interval). Operationally: a fast user who paste-then-copies within 500ms can either lose their own copy from history or have the app's auto-paste re-captured. The symptom is *intermittent* — "sometimes my last copy disappears from history" — exactly the kind of bug that gets dismissed as user error. From the on-call chair, this is the worst class of issue: low frequency, high mystery, no log trail.

### To Testing F1 — No `tauri-driver` / WebDriver E2E layer

**This is the most operationally impactful gap in the review.** I focused on instrumentation; Testing focused on verification — both share the same root: there is no automated assertion that the binary actually does what we think it does between source and the user's machine. The two combine to a brutal operator truth: **today, the team has no signal whatsoever that any release works**, beyond the developer's manual smoke. The first user to install v1.0 is also the first automated test of v1.0 in any meaningful sense.

The Testing perspective's suggestion (one smoke test per OS, drive a Playwright/WebDriver session, assert capture → save → list) is the *minimum* operational gate to ship a public release. From the operator chair, I would add: **the smoke test should produce a runtime artifact (log file, screenshot of library window with the captured row) that gets uploaded as a CI artifact.** Then a release that "passes CI but is missing the artifact" is detectable as a partial pass — a thing CI alone wouldn't tell you.

### To Testing F2 — Updater untested end-to-end; pubkey/private-key drift undetected

This is the worst operational time-bomb in Testing's findings. **If the embedded `pubkey` ever drifts from the actual signing key, every update for every user is silently rejected forever** — the updater logs a signature error to a log file we don't have (my finding #1), the user sees no update prompt, and the operator's only signal is the install-base getting older over time without complaints. The Testing perspective's suggested fix (CI step that re-derives pubkey from `TAURI_SIGNING_PRIVATE_KEY` and diffs against `tauri.conf.json`) is **cheap, sound, and should be a hard gate in `release.yml` before the build step runs.**

### To Testing F4 — "Sensitive clipboard" is schema-only

Reinforces Adversary F2 and my finding #2. The Testing lens adds: **the manual checklist line "Sensitive clipboard: 1Password copy → not in history" cannot pass today**, which means the release engineer is either skipping it, lying about it, or hasn't read the checklist. From the operator chair, the question is: which? If skipping/lying, the checklist itself is fiction. If unread, the checklist has no enforcement and shouldn't be in the spec. Either way, the spec's §10.5 needs to be reconciled with reality before v1, or v1 ships against a checklist that is structurally impossible to satisfy.

### To Testing F13 — Coverage CI excludes the riskiest files by regex

**This is my finding #11 (no audit / SBOM) compounded.** The repo reports >90% coverage; the actual coverage on the IPC perimeter (the surface that fails when it fails) is closer to 0%. An operator who reads "snapper-keeper has 90% test coverage" in the README would conclude this is a well-tested project; the truth is "snapper-keeper has 90% coverage on the pure-logic surface that doesn't change between releases and ~0% on the surface that does." This is a *trust* failure, not just a measurement failure. The Testing perspective's split (gated logic coverage vs. tracked IPC coverage) is operationally correct because it tells the on-call which surface to be suspicious of.

### To Testing F14 — Watcher thread dies silently on `Clipboard::new()` failure

Confirms my finding #10 from a test-coverage angle. The bug is real and the test gap means it can't even be regression-protected. Both perspectives agree: retry with backoff + emit `clipboard:unavailable` event. Operationally, the *event* matters as much as the retry — without an event, the popup window has no way to render a "clipboard watcher offline" state.

---

## Tensions

### Tension 1: Maintainer F1 (delete dead TS bindings) vs my operator preference (repurpose them as diagnostics)

Maintainer's F1 argues to *delete* `@snk/ocr` and `@snk/updater` packages because nothing consumes them. From my operator chair, I'd rather *use* them in the Settings → About panel I proposed in my finding #13. Both produce a healthier codebase; the Maintainer optimizes for "the workspace looks coherent to a new contributor," I optimize for "the on-call has a debug surface."

**Resolution:** Maintainer's option is correct *if* the About panel isn't built. If the About panel is built (which I'd strongly recommend), the bindings become live and Maintainer's concern goes away. So the order matters: build About panel first, then re-evaluate whether to delete.

### Tension 2: Adversary F7 (require user confirmation before download_and_install) vs my preference for low-friction auto-update

Adversary F7 proposes "Add a user confirmation dialog before `download_and_install` fires" — auto-install is "hostile for a share-friendly side project." I'm sympathetic but disagree from the operator angle. For a side project with no staffed on-call, *aggressive auto-update is the only way to push a security fix to the install base inside hours.* A user-confirmation dialog means a security fix takes weeks to reach the long tail of users who dismiss the prompt.

**Resolution:** Differentiate by update class. Default: download in background, prompt to restart (the current design). Security-class updates: same flow but with a non-dismissable banner after 24h. Add a `urgency` field to `latest.json` that the manifest can flag. Adversary's signature-failure terminal-disable (F8) is independently correct regardless of the auto-install decision.

### Tension 3: Adversary F13 (offer SQLCipher) vs Maintainer minimalism + my logging push

Adversary F13 proposes optional SQLCipher encryption of the DB. Maintainer's lens generally favors *less* scope, not more. From my operator chair, encryption complicates the diagnostic story dramatically — if the DB is encrypted and a user reports "search is broken," the operator can't ask "send me your DB file." Encryption also conflicts with the file-based logs I want (which I'd argue should *not* be encrypted, because they're the only thing the user can send for support).

**Resolution:** Encryption should be opt-in and *off* by default for v1. Document in PRIVACY.md (the *honest* version after Adversary F5 is fixed) that the DB is plaintext. Revisit encryption in v1.x once support workflow is mature enough to handle encrypted artifacts.

### Tension 4: Testing F1 (E2E smoke per OS) vs the project's "Windows requires interactive desktop" constraint

Testing F1 wants a per-OS smoke test in CI. CLAUDE.md's known limitation: "Smoke tests on Windows require an interactive desktop session. SSH-only environments can build and lint but can't smoke." GitHub-hosted Windows runners are not interactive desktops in the way RDP is — `RegisterHotKey` and `WebView2` may or may not work in `windows-latest` runner sessions.

**Resolution:** This is testable, not assumed. The team should burn one CI job to confirm whether `windows-latest` runners are interactive enough for a basic capture-and-list smoke (no hotkeys needed for the smoke). If yes, Testing F1 lands trivially. If no, the smoke needs to be partial: macOS gets the full E2E, Windows gets a "binary starts, library window paints, no panic" minimum. Either way, *some* runtime gate is better than the current zero-runtime gate. The operator's preference is to know *which* OS is unverified per release.

---

## New Insights

### NI1. Combining my #5 (no migration backup) + Maintainer F8 (hardcoded "from 0 to 4") + Testing F7 (migration tests don't carry data forward) = the next migration is a strict liability event

I noted that there's no pre-migration backup. Maintainer noted the migration error returns lies about which version failed. Testing noted the tests don't carry data forward through migrations. Stacked: **the team will ship v1.1 with a new migration, it will fail on some real-world DB shape that none of the test fixtures exercise, the error message will say "from 0 to 4" (wrong), there will be no backup, no log file, and no fixture-based test that caught it.** The recovery story is "user files an issue with their broken-DB symptoms, the operator can't ask for an artifact because there's nothing to send."

This is the *single most likely* operational disaster for the project. None of the perspectives individually flagged it as critical; combined, it's the highest-EV failure mode.

**Recommendation:** Before v1.0 even ships, the team should commit a `tests/migration_forward_compat.rs` that takes the v0.0.1 schema (current production schema), inserts realistic fixture data (say 50 captures, 30 tags, 200 clipboard items, 80 OCR rows), then applies the migration train to whatever is current. Any future migration must pass this test. This is the cheapest insurance policy in the entire project.

### NI2. The `paste_item` command is the most dangerous IPC surface in the app — and three perspectives noticed it independently

Adversary F1 flagged `paste_item` as "the worst escape" — synthesizes a keystroke into the focused foreground app. My finding #3 flagged the over-broad capability that grants it to all windows. Adversary F6 reinforced. Testing F11 added that image clipboard items can be inserted but never pasted (the rejection error isn't tested either, so users will click and get an inscrutable error).

What I missed in Round 1: **`paste_item` is also the command with the worst "what just happened?" debuggability**. When `SendInput` fires a synthetic Ctrl+V into a focused window, there's no record on either side. The target app doesn't know which process injected. The user can't tell if a paste was the app or a real keypress. If an XSS payload uses `paste_item` maliciously, the symptom is "weird text appeared in my terminal at 2am" with no audit trail anywhere. **Add audit logging specifically for `paste_item`** — every invocation gets a line in the security-events log (NI3 below), with item kind, source window label, and target foreground window name. Privacy-preserving: log the metadata, not the content.

### NI3. Add a separate, never-rotated `security-events.log`

Synthesizing across the perspectives, the team needs *two* log sinks, not one:

- `snk.log` — rotating daily, 14-day retention (per the design's promise in §9.3), captures normal `info`/`warn`/`error` events. PII-free per design.
- `security-events.log` — append-only, never rotated, surfaced in Settings → About. Captures:
  - Updater signature failures (Adversary F8)
  - Capability denials (Adversary F6, my #3)
  - Panic hook captures (my #1)
  - `paste_item` invocations with source/target metadata (NI2)
  - Autostart toggles (Adversary F9)
  - Settings changes that touch privacy-relevant defaults (e.g. `clipboard.app_blocklist` once it exists)

The security-events log is small (events are rare) and is the artifact the operator asks for when a user reports something weird. Separating it from `snk.log` means the rotating log can be aggressive about retention without losing the security audit trail.

### NI4. The first-release tag should be a "canary" deployment to a known subset

I noted in Round 1 that the first tag must be plain SemVer or the `/releases/latest/` redirect breaks. Combined with Testing F2 (no end-to-end updater verification) and Adversary F11 (chocolatey supply chain) and my finding #11 (no audit job), the *single largest* operational risk is that the first user to install v1.0 finds the bug that breaks everyone.

**Operator recommendation:** Treat v0.1.0 as a canary. Tag, build, sign, publish — but to a *separate* GitHub release marked `prerelease: true`. The updater endpoint pointing at `/releases/latest/` skips this, so it only reaches users who visit the Releases page manually. Wait 1-2 weeks. If nothing explodes, cut v0.1.0 as `prerelease: false`. This costs a delay; it buys an asymmetric reduction in the "first user discovers everything broken" risk.

### NI5. The dead `_app` parameter pattern (Adversary F14) is the seam where my proposed audit logging fits

Adversary F14 noted that most commands accept `_app: tauri::AppHandle<R>` but ignore it — meaning the commands can't distinguish callers. From the operator chair, **this is exactly the place where the audit logging from NI2/NI3 belongs**: a small middleware wrapper that reads the calling window label from the `AppHandle`/IPC envelope, writes an entry to the security-events log for destructive commands, then forwards to the real handler. Defense-in-depth + diagnostic surface in one. Cheap to implement (~20 lines of macro or wrapper), pays for itself the first incident.

### NI6. The release workflow's smoke-test of `cmd.exe` (my finding #12) becomes much more defensible if it tests signature **flow** not just **success**

I called this out as a minor taste issue. Adversary F8 makes it bigger: signature verification is the load-bearing piece of the entire updater. The current smoke confirms "we can sign things" — but never confirms "a signed thing verifies against our pubkey." **Recommendation:** instead of signing `cmd.exe`, sign a small test artifact, then **also** verify the signature with `tauri signer verify` (or the equivalent minisign check) against the same pubkey embedded in `tauri.conf.json`. That's the Testing F2 pubkey-drift gate, the Adversary F8 signature-flow gate, and my taste objection, all addressed in one smoke step.

---

## Summary

Cross-pollination shifted my assessment from "the operational surface is undertested" (Round 1) to *"the operational surface is undertested in ways that compound — the absence of logging, the absence of E2E gates, and the over-broad capability model multiply each other's blast radius."* The Adversary findings (especially F1, F2, F5, F11) turn most of my operator concerns from "we can't debug" into "we can't even detect the incident in the first place."

Two new must-fix items for v1: (a) fixture-based migration forward-compatibility test (NI1) — the cheapest insurance against the most likely disaster, and (b) the pubkey/private-key drift CI gate from Testing F2 — the cheapest insurance against the most operationally expensive disaster. Both are <1 day of work and both prevent classes of incident that have no in-field recovery.

My single strongest cross-pollination conviction: **before v1 ships, the team needs file-based logging, a separate security-events log, an IPC audit middleware for destructive commands, and the canary deployment strategy from NI4.** None of these are big lifts individually. Together, they convert "we ship and hope" into "we ship and can respond."
