# Adversary Perspective — Round 2 (cross-pollination)

My Round 1 stands as written; this round is purely reactions, tensions, and new attack-surface insights triggered by reading the other three perspectives.

The single biggest update: **the convergence of independent perspectives on the same gaps is itself a security signal.** When Maintainer and Testing-Strategy independently arrive at "sensitive-clipboard is schema-only" and "no implementation," and Operator independently arrives at "PRIVACY.md promises a setting that doesn't exist," that is no longer four separate findings — it is a coherent picture of features that exist on paper, gate decisions are being made against, and which an adversary can exploit precisely because everyone assumes they exist.

---

## Reactions

### To Operator F1 (no file-based logging, no panic hook) — opens new attack surface

This is the *enabling* condition for almost every attack I identified in my Round 1 to go undetected. F8 in my report — signature verification failures emitted as plain strings to a transient UI status — assumed at minimum the tracing call would land in *some* persistent sink. Operator confirms it lands nowhere. So: a MITM-served forged manifest fails signature, surfaces a 2-second UI toast, and is gone. No log file, no postmortem, no way for the user to even say "I think something weird happened at 3:14pm." From the adversary side this is a gift — the same attack can be retried daily because there's no record of the prior failure. **This upgrades my F7+F8 (updater rollback / sig-error handling) from "high" to "showstopper" in combination.**

Additionally: stored-XSS payloads (my F1) can call any IPC including future-added debug commands; with no audit log of which commands fired from which window, post-incident forensics is impossible. The user's only artifact is the SQLite database, which the XSS payload can mutate as part of cleanup.

### To Operator F3 / F15 (capability-file scope) — pure confirmation

We arrived at this independently with the same fix. Operator notes the design doc *explicitly committed* to a `clipboard-popup.json` capability file (§8.3 lines 557-565) that doesn't exist. That changes the framing of my F6 — this isn't an oversight, it's a regression from a designed-and-documented decision. The team built the right model, then didn't implement it. That's worse than not having designed it, because the design doc is now lying to future contributors and to any security-aware reader doing diligence.

### To Operator F4 / F5 (macOS OCR silently broken; no DB backup before migration)

Operator F4 reframes my threat model. I had been thinking of OCR as the channel that introduces attacker-controlled text into the FTS index (XSS source per my F1). If macOS OCR silently fails to run for the vast majority of macOS installs, that drastically reduces the F1 *attack rate on macOS specifically* — but it also means the Mac install base is essentially a different product than what's documented, and the search-based attack surface lives almost entirely on Windows. Doesn't change the severity; does change the targeting.

Operator F5 (no pre-migration backup, fake `recoverable` flag) raises a destructive-update vector I hadn't considered: a malicious update doesn't need to drop a payload — it can ship a migration that intentionally corrupts the DB. My F7 (no version-pinning / rollback floor) combined with Operator F5 (no backup): a forged update at version N+1 with a malicious migration permanently destroys the user's library, and there is no recovery path even if the user immediately reinstalls. **Net: this is a wormable destructive primitive given any single compromise of the release pipeline.**

### To Operator F7 (no kill-switch for bad builds)

I had F7 in my report for downgrade attacks; Operator hits the inverse — the team can't pull back a bad release. These compose: if signing infrastructure is compromised and a malicious build is pushed to `latest.json`, the team can't yank it (Operator F7), users can't downgrade (my F7), and the malicious build's auto-update timer fires every 24h to re-pull whatever the attacker serves next (my F7 + my F8). The system is engineered to be *maximally helpful to an attacker who briefly gets pipeline access*.

### To Operator F11 (no `cargo audit` / SBOM in CI)

This affects my F11 (chocolatey-installed Tesseract bundled verbatim) directly. Without an SBOM published per release, even if the team eventually pins Tesseract version, downstream consumers (corp IT vetting the binary, dependabot-style scanners on their side) have no way to know which version shipped. The information asymmetry favors attackers — they can scan released bundles for known-vulnerable Tesseract versions in their leisure; defenders can't proactively know they're affected. Combined with Operator F1 (no logs) means a user running a vulnerable bundled Tesseract has zero forensic surface if the CVE fires.

### To Maintainer F1 (dead TS bindings for `@snk/ocr` and `@snk/updater`)

This is a small attack surface I missed in Round 1: dead npm workspace packages that ship in the source tree but are never imported. A supply-chain compromise of these packages (or their transitive deps) does *not* land in the shipped product today — but does land in the dev-machine `pnpm install` and CI runs, where the attack can read secrets (`GITHUB_TOKEN` in CI, `.npmrc` tokens on dev machines, Azure CLI credentials cached on dev machines). Their continued existence is gratuitous attack surface for zero functional benefit. **Add this as: dead packages = expanded supply-chain surface with no benefit.**

### To Maintainer F2 (cross-plugin internal-path imports)

Confirms structural problem; minor security adjacency: when imports go through `::plugin::LibraryState` rather than the crate-root re-export, future refactors that intentionally restrict what `plugin` exports (e.g., locking down `LibraryState::db` to a smaller surface) will be silently bypassed because importers reach through. From an adversary lens this is the kind of pattern that prevents future hardening work from actually hardening anything. The CLAUDE.md rule #3 was correctly written; not enforced.

### To Maintainer F4 (README claims clipboard eviction is "configurable")

Adds to my F2 (clipboard watcher capture-everything). The README implicitly markets the clipboard feature as "with safety knobs the user can tune." None exist. A privacy-attuned user reads the README, installs, fails to find the knob, files an issue or moves on — meanwhile their cleartext clipboard sits at rest under `MAX_UNPINNED = 200`. Same shape as my F5 (PRIVACY.md mismatch) and Operator F2: documentation makes promises the binary can't keep.

### To Maintainer F7 (`From<io::Error>` discards path)

Subtle adversary angle: when error messages lose the failing path, server-side / log-side correlation becomes impossible. If a future installation logs "io error at : permission denied", a defender investigating ransomware-like behavior (selective file-permission denial via local malware) can't tell which file was targeted. Lower-tier but worth noting alongside Operator F1.

### To Testing-Strategy F2 (auto-updater untested end-to-end)

Testing-Strategy nails the specific concrete fix I should have proposed for my F7+F8: "A CI step in `release.yml` that re-derives the pubkey from `TAURI_SIGNING_PRIVATE_KEY` and `diff`s against the literal in `tauri.conf.json`." If those two ever drift (rotation, accidental regenerate, secret-store recovery), every update silently fails — and per Operator F1 there's no log of the failure. The pubkey is `dW50cnVzdGVkIGNvbW1lbnQ6...` at `tauri.conf.json:109`; the only thing preventing key drift is human discipline. **Adopt Testing-Strategy's pubkey-diff guard as a hardening recommendation in addition to my F7 manifest-signing fix.**

### To Testing-Strategy F3 + F4 (clipboard watcher untested; sensitive-clipboard schema-only)

This is independent triangulation on my F2 — three perspectives (Adversary, Operator F1/F4 derivative, Testing-Strategy F3/F4) converge. Testing-Strategy adds an insight I didn't have: the *test fixture* in `settings.rs:85-87` planted a `["1Password", "KeePass"]` blocklist string as a fixture, which is doubly misleading — readers of the test file (including security auditors doing source review) will assume the blocklist is implemented and tested. From an adversary perspective, that fixture is *evidence* I would have cited in Round 1 if I'd noticed it, that the feature exists; it does not. Worth elevating: planted fixtures of unimplemented features actively mislead third-party audits.

### To Testing-Strategy F5 (cross-plugin event protocol untested)

Adversary angle on this: `event.payload().trim_matches('"').to_string()` (Testing-Strategy F5, citing `snk-ocr/src/plugin.rs:43-61`) is parser-fragile in a way that combines badly with my F15 (annotation state stored as opaque JSON, never validated). If a derived-capture flow ever lets a frontend-controlled string reach the `capture:saved` payload, an attacker can craft an id containing embedded `"` to break the trim_matches contract — at minimum, OCR fires against a path the attacker shaped. Low-likelihood today (UUIDv7 sender side) but the receiver's casual string-handling assumes the sender forever; that's exactly the kind of implicit contract Testing-Strategy F5 says is untested. **Add as: untyped event-payload-as-string handling is a future XSS-adjacent injection point if the sender ever changes.**

### To Testing-Strategy F6 (TS bindings test command names but not Rust acceptance)

This is the *defense* side of my F1. If the IPC command surface ever flips snake_case behavior, an XSS payload's `window.__TAURI__.invoke('hardDeleteCapture', ...)` would silently fail — which would actually mitigate my F1 partially by accident. Per Testing-Strategy this isn't tested either way. From an adversary lens, neither side benefits — defender can't rely on the case-mismatch protection, attacker can't rely on the surface staying invoke-able. The right answer is what Testing-Strategy proposes: a generated schema.

### To Testing-Strategy F8 (`OnceLock` test pollution acknowledged but unfixed) and F10 (no real-image OCR fixture)

F10 is the most-quotable counterpart to my F1 (XSS via FTS). I cited "Tesseract OCR'd text from a hostile webpage" as the input vector; Testing-Strategy notes there is no test asserting that Tesseract *correctly extracts text from any real image*. So the team has no in-CI signal whether a tesseract version bump degrades OCR — and the failure mode for "OCR stopped working" is identical to "OCR is being defeated by an attacker who knows to use anti-OCR fonts." Defenders can't distinguish "feature broken" from "feature evaded." Low-priority for v1; worth noting that the test gap means OCR's robustness is unmonitored.

### To Testing-Strategy F13 (coverage CI excludes the riskiest files)

This is the meta-finding that contextualizes most of my Round 1: the coverage gate looks healthy (>90%) precisely because it measures the parts that are well-tested. The `commands.rs`/`plugin.rs`/`watcher.rs`/`paste.rs` files — i.e., every file I found a vulnerability in — are excluded from the gate. **A security-auditing reader who skims CI and sees ">90% coverage" will assume the IPC surface is exercised; it is not.** This is the same shape as my F5 (PRIVACY.md misrepresents the implemented surface) and Testing-Strategy F4 (sensitive-clipboard schema-only): documents and metrics that overstate the actual security posture. The pattern is consistent enough to be its own meta-finding (see New Insights below).

### To Testing-Strategy F14 (silent thread-death on `Clipboard::new()` failure)

Adversary angle: if a local attacker can briefly grab clipboard ownership at app startup (trivial — open Notepad, copy something programmatically, race), the clipboard watcher dies for the session. The user's clipboard history stops working — they get suspicious nothing's being captured — they restart the app — by then the attacker has released the clipboard. The watcher recovers, and the attacker has *suppressed* clipboard logging during a specific window of time. Niche, but it's a primitive: a local attacker can briefly disable clipboard history at will, useful for staged credential theft where the attacker doesn't want their own activity in the user's logs.

---

## Tensions

### Tension 1: Maintainer F1 vs my F1 — delete the dead TS packages, but the dead packages contain dead exfil paths

Maintainer recommends deleting `packages/snk-ocr/` and `packages/snk-updater/` because they're unused. I agree — *and* I want to note: if my F1 (XSS) lands, those packages' presence in `node_modules` (transitively pulled by `pnpm install`) actually *expands* the attack surface inside the webview during dev/preview. Killing them helps both perspectives, but Maintainer's "delete for clarity" reason should be paired with "delete for attack surface" reason. Same fix, different *why*.

### Tension 2: Operator F4 (bundle Tesseract on macOS) vs my F11 (don't bundle Tesseract verbatim from chocolatey)

Operator wants more bundling (so macOS works); I want less bundling (so a supply-chain compromise of choco-tesseract doesn't land in the signed installer). The synthesis is: bundle on both, but bundle from a *verified source* with a pinned hash. Operator's fix paragraph at (a) suggests `brew install tesseract` then copy — same architectural risk as choco on Windows. Both fixes need to add hash-pinning + signature verification on the source distribution before copying. Otherwise we trade "Mac doesn't OCR" for "Mac silently distributes whatever brew shipped." Recommend the team adopt my F11's "vendor the bundle in-repo with a hash-pinned download step" for both platforms.

### Tension 3: Operator F12 (don't sign cmd.exe in the smoke test) vs Testing-Strategy's preference for more end-to-end signing verification

Operator dislikes the cmd.exe smoke as auditability noise. Testing-Strategy doesn't address the smoke directly but implies more signing-pipeline verification is needed. From my adversary lens, the smoke test is *necessary* — it catches Azure Trusted Signing outages before they corrupt a release. The fix is what Operator suggests (sign a known harmless test binary the team ships in the repo) — but that test binary should be *committed in the repo with a known hash* so the smoke test result is itself verifiable. Both perspectives compatible; the resolution is "smoke against a vendored, hashed test file."

### Tension 4: Testing-Strategy F11 (image clipboard items inserted but never pasteable) vs my F2 (clipboard captures too much)

Testing-Strategy says "either implement image paste or filter image rows from popup." I would say: **do not implement image paste**. The current behavior — image clipboard items are inserted into history but the only thing the user can do with them is *view* them — is mildly safer than the alternative where the popup can synthesize keyboard-injection of arbitrary clipboard images into the foreground window. My F2's call for a blocklist + sensitive flag would, if implemented, mean image paste of password-manager-copied images is an even stronger vector. The right v1 answer is "don't store image clipboard at all unless the user opts in via a setting" — which closes both the storage problem (my F2) and the paste-broken UX (Testing-Strategy F11) at once.

### Tension 5: Maintainer F8 (Migration error has hardcoded `to: 4`) vs my F7 (no version pinning)

Maintainer correctly notes `to: 4` is a stale literal that will be wrong after V005 lands. I would add: **`from: 0` is actively dangerous in combination with my F7.** If the updater pushes a build with a malicious migration, the recovery UI (whenever someone implements one) reads `from: 0` and may decide to "restore from before V001" — i.e., wipe the entire database. The hardcoded `from: 0` will gain weight as soon as anyone writes recovery code on top of it. Maintainer's fix (derive from `migrations().current_version()`) is also a security fix.

---

## New Insights

### N1. The "designed but not implemented" pattern is the meta-vulnerability

Across the four perspectives, the recurring shape is: design doc / README / PRIVACY.md / test fixtures / schema columns reference features that don't exist in code.

Verified instances:
- Adversary F5 + Operator F2: PRIVACY.md "disable update checks in Settings" — no setting.
- Adversary F5 + Operator F2: PRIVACY.md "Microsoft Store edition compiles out updater" — no build variant.
- Operator F3 + Adversary F6: design `clipboard-popup.json` capability file (§8.3) — file doesn't exist.
- Operator F4 + Maintainer F5: design / README OCR feature on macOS — Tesseract not bundled.
- Operator F5: design "pre-migration backups in `backups/`" + `recoverable` flag — never written; flag string-matches a substring that never appears.
- Operator F11: design §10.4 "Nightly: full E2E + `cargo audit` + `npm audit`" — no nightly job.
- Operator F13: spec promised "About panel with version/data-dir/logs" — Settings has no About row.
- Operator F14: design "macOS universal binary" — release ships two separate arch bundles.
- Maintainer F4: README "configurable eviction limit" — hardcoded `MAX_UNPINNED = 200`.
- Adversary F2 + Testing-Strategy F4 + Maintainer (settings test reference): `sensitive` column + `clipboard.app_blocklist` setting + test fixture — never read, never written.
- Testing-Strategy F1: spec §10.2 E2E layer via `tauri-driver` — does not exist.
- Testing-Strategy F11: schema column `kind = 'image'` allowed, paste rejects with error — feature half-shipped.

This pattern is the meta-vulnerability because **every external audit / due-diligence read of this codebase will form a security model based on the documented features, not the implemented ones.** A corporate IT vetting team reading PRIVACY.md + the spec gets a vastly different picture of risk than what ships. From the adversary perspective this is exploitable: I can craft a phishing-adjacent attack that relies on user beliefs derived from PRIVACY.md (e.g., "the app doesn't capture password manager clipboards, so I'll trust it next to my password manager") — and the user's belief is wrong.

**Recommendation:** Before v1.0, every aspirational claim in PRIVACY.md, README, and the spec should be either (a) implemented, (b) deleted from the public-facing doc, or (c) explicitly marked "roadmap, not v1.0." The current diff between "documented" and "implemented" is the largest single vector by surface area.

### N2. The dead/unused code zones double as silent persistence sites

Maintainer F1 (dead TS bindings), Maintainer F11 (snk-hotkeys' unused `thiserror`), Operator F9 (OCR queue startup sweep absent), Testing-Strategy F3 (watcher silent thread death) all have a shared adversary-friendly property: **they are zones where state change goes unnoticed.** A malicious dependency landing in a dead TS package can use its build-time hooks to read CI secrets without any code-review trigger (Maintainer F1). A malicious clipboard recovery in the silent-thread-death case (Testing-Strategy F14) can be made permanent by a local attacker who racetimes app startup. The OCR queue's missing startup sweep means the *absence* of OCR text for a capture from yesterday goes unexplained — an attacker who wants to suppress OCR indexing for specific captures can do so by terminating the worker between enqueue and processing, and the gap is invisible.

These are not classic vulnerabilities — they're absences of integrity checking. Pattern: anywhere the system says "fire and forget" or "if it fails, log and move on," an attacker can attack the forgetting.

### N3. Three independent perspectives flagged the lack of an opt-out toggle for the updater, but none of us flagged that the design doc promised it as a privacy feature

Re-reading the design `§13 row` (the decisions log) and PRIVACY.md together: the updater opt-out is *the* documented privacy mitigation that bridges "the app makes one network call" → "users with stricter requirements can disable it." Without the toggle, the privacy posture isn't "minimal network usage with user control" — it's "guaranteed network usage with no user control." That's a different product. The first regulatory-style review (CCPA / GDPR-adjacent inquiry, or even a corporate procurement checklist) will read PRIVACY.md, look for the toggle, fail to find it, and **flag the privacy policy as inaccurate.** That is a separate severity from the technical findings — it's reputational and possibly legal. None of the four Round 1 reports framed it this way; the team should.

### N4. The capability gap is a *defensive* opportunity that, if executed, mitigates several of my Round 1 findings simultaneously

Adversary F6 + Operator F3+F15 all converge on "split capabilities per window." Doing this competently:
- Closes most of the blast radius of my F1 (XSS) — if the library window has fewer commands than today, an XSS in search can't `paste_item` or `set_autostart`.
- Closes the autostart-persistence vector in my F9.
- Reduces the cross-window attack surface — an XSS in the smaller `clipboard-popup` (which renders clipboard text, partly attacker-controlled) can't touch the library.

This is a single ~half-day refactor that meaningfully shrinks the threat model. **Of all the v1.0 fixes, this is the highest ratio of (risk reduction) / (engineering hours).** I would recommend the team prioritize this even ahead of the F1 CSP/escape fix, because per-window capability is a structural defense that pays compounding dividends; the CSP fix is a single-point defense.

### N5. The same primitive Operator F11 calls "no SBOM" enables a different attack I missed

Without an SBOM and without `cargo audit` gating, the team's *response time* to a published CVE is bottlenecked on someone noticing the CVE through an out-of-band channel. The attack: file a low-key, plausible-looking GitHub issue against `snapper-keeper` claiming a bug in feature X. While the maintainer is investigating, drop a real CVE in a dep. The maintainer is distracted, no automated scanning catches it, the CVE-vulnerable version stays in production for the typical 1-3 week disclosure window — during which time the attacker exploits in the wild. This is **social-engineering of attention as a force multiplier on the missing CI gate.** For a one-person side project (per CLAUDE.md), this is a real risk model — the attacker isn't competing against a SOC, they're competing against one developer's calendar.

### N6. The release pipeline's signing job runs the *full* `cargo build` with secrets in env — implies any build-script can read them

`.github/workflows/release.yml:172-183` exports `TAURI_SIGNING_PRIVATE_KEY`, `APPLE_SIGNING_IDENTITY`, `AZURE_CLIENT_SECRET`, etc. into the env, then runs `pnpm tauri build`, which transitively runs `cargo build`, which executes every dependency's `build.rs`. A malicious `build.rs` in any transitive dep can `std::env::var("TAURI_SIGNING_PRIVATE_KEY")` and exfiltrate. Combined with my F10 (actions pinned by mutable tag) and my F11 (chocolatey-installed Tesseract bundled verbatim), and now Maintainer F1 + N2 (dead TS packages = expanded supply-chain surface), this stacks into **a credible attack path: compromise one transitive dep → exfiltrate Tauri minisign key → sign and ship a forged update → no rollback / no kill-switch (Operator F7 + my F7) → no log of users who received the malicious update (Operator F1) → no SBOM (Operator F11) so defenders can't trace which version was bad.**

This is what an end-to-end pre-mortem of the most-likely catastrophic attack chain actually looks like, weaving across all four perspectives. The team has built a series of locally-reasonable choices that compose into a fragile chain. The mitigations are all individually known:
- Run `cargo build` for release in a job that has *only* the minimum secrets needed for *that step* (split build from sign).
- Pin actions by SHA (my F10).
- Vendor Tesseract with hash pinning (my F11 + Tension 2 above).
- Add SBOM + audit gates (Operator F11).
- Sign `latest.json` (my F7).
- Add minimum-version floor (my F7).
- Add pre-migration backups (Operator F5).
- Add file-based logging + crash hook (Operator F1).

No single fix solves it; the chain is the threat model. The team should do at least the bottom three (file logging, pre-migration backups, latest.json signing) before v1.0 — they're cheap and they specifically close the "we'll find out from a user email" failure mode that everything else amplifies.

---

## Closing

My Round 1 stance was: hold v1.0; land F1+F2+F4+F5 minimum. After reading the three other perspectives, that stance hardens. The other perspectives collectively confirm:
- The XSS vector exists in a webview with no CSP, with the capability surface the design said to avoid.
- The clipboard sensitive-data feature is schema-only across three independent reads.
- The privacy policy contradicts the binary in ways operator and adversary both flag independently.
- The signing/release pipeline has integrity gaps (untested updater, no SBOM, no rollback, no manifest signing, no pre-migration backup, no audit gating) that compose into a credible end-to-end takeover path.

The narrowest pre-v1.0 cut would be: (1) implement OR delete the privacy claims that don't match code; (2) split capabilities per window (highest leverage defense); (3) set a real CSP and fix the dangerouslySetInnerHTML site; (4) add file-based logging + crash hook so post-incident forensics is possible; (5) sign latest.json. That's 5 items, all under a week of work for a single dev, and it converts the current "first public release will probably hold" posture into "first public release can be defended."
