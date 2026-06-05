# Pre-release hardening cluster — implementation plan

**Date:** 2026-06-05
**Branch:** `hardening/pre-release`
**Goal:** Resolve the outstanding `pre-release-review` issues, then cut a new signed Windows release package.

This plan closes the open `pre-release-review` issues filed from the 2026-05-24 prerelease
synthesis. Triage (2026-06-05) verified each against current `main`; several issue texts were
stale. Verdicts below reflect the *current* tree, not the issue text.

## Triage outcome

| # | Sev | Verdict on `main` | Disposition |
|---|-----|-------------------|-------------|
| 41 | med | **OBSOLETE** — Tesseract sidecar deleted in Phase 10 (PR #135); OCR is Vision/WinOcr native | Close, no code |
| 159 | med | **LIVE (dev-only)** — vitest 3.2.4 < 4.1.0 advisory range; devDependency, never shipped | Bump to ^4.x |
| 27 | high | **PARTIAL** — `RejectedBySignature` state exists+terminal+tested but never *set*; manifest unsigned; no downgrade floor; no kill-switch docs | Implement (see design) |
| 49 | med | **NOT STARTED** — `_app` ignored; no `Unauthorized` type. Per-window capabilities (#146) are first gate | Implement (defense-in-depth) |
| 35 | med | **NOT STARTED** — `SECURITY_LOG_TARGET` wired but unused | Implement (with #49) |
| 37 | med | **NOT STARTED** — watcher dies silently on init failure | Implement |
| 44 | med | **PARTIAL** — 7-job pipeline + verify-*.sh exist; no release-readiness gate | Implement |
| 53 | med | **NOT STARTED** — no mock_app cross-plugin event test | Implement |
| 54 | med | **NOT STARTED** — no get_ipc_response IPC arg test | Implement |
| 60 | med | **NOT STARTED** — PRIVACY.md lacks retention docs; no SQLCipher | Implement (see design) |

## Decisions (locked 2026-06-05)

- **Decision A → A2.** Skip custom `latest.json` signing; implement downgrade floor +
  signature-terminal + kill-switch docs. **Store-review rationale:** the updater is compiled out of
  the `store-edition` build (PR #148), so a store reviewer never sees update logic; Apple
  notarization is an automated malware/hardened-runtime scan, not a manifest-crypto review.
  Manifest-signing therefore reduces store/notarization friction by zero.
- **Decision B → B2.** PRIVACY.md retention docs now; SQLCipher split into its own tracked issue.
  **Store-review rationale:** stores don't require encryption-at-rest; adding SQLCipher *triggers*
  Apple export-compliance (`ITSAppUsesNonExemptEncryption`) and adds native code for notarization to
  scan — a small net *increase* in review friction, not a decrease.

## Two design decisions (analysis that led to the locked decisions above)

### Decision A — #27 item 1: how far to take "sign latest.json"

**Current guarantee (already in place):** the release pipeline minisigns each artifact
(`*.app.tar.gz.sig`, `*-setup.exe.sig`) and embeds those signatures in `latest.json`. Tauri's
updater verifies the artifact signature against the embedded pubkey
(`tauri.conf.json` → `plugins.updater.pubkey`) before installing. An attacker who tampers with
`latest.json` **cannot** forge a valid artifact signature without the private key — so injecting
malware via a doctored manifest is already blocked.

**What manifest-signing would add:** protection against (a) **rollback** — serving an older,
validly-signed `latest.json` to pin users to a known-vulnerable version — and (b) version-string /
release-notes spoofing. (a) is *also* closed by the downgrade floor (item 2). (b) is cosmetic.

**Constraint:** Tauri fetches the endpoint manifest internally; there is no clean hook to verify a
detached `latest.json.minisig` before Tauri consumes it. Doing it means a custom pre-fetch +
minisign-verify step that re-downloads and re-parses the manifest ourselves — meaningful complexity
that duplicates a guarantee Tauri already provides for the part that matters (the binary).

**Options:**
- **A1 (full):** implement custom manifest fetch + detached-minisig verification before
  `updater.check()`; publish `latest.json.minisig` from the pipeline. Matches the issue literally.
- **A2 (recommended):** treat artifact-signature + downgrade-floor + HTTPS as sufficient for
  manifest integrity; **skip** custom manifest-signing; document the threat model + why in
  `docs/` and the kill-switch protocol. Spend the saved effort on the downgrade floor and
  signature-terminal wiring, which are the real gaps. Close #27 item 1 as "won't-do, superseded by
  downgrade floor" with rationale.

**Recommendation: A2.** Implement items 2, 3, 4 fully; document item 1 as deliberately scoped out.

### Decision B — #60: SQLCipher now, or docs-now + defer encryption

`snk-library` opens a plain `rusqlite::Connection` (`db.rs:open`). SQLCipher means:
swapping to the `bundled-sqlcipher` build (pulls OpenSSL/SQLCipher into the build on all 3 OSes),
deriving a key from the OS keychain, `PRAGMA key` on every open path
(`open`, `open_no_migrate`, `open_with_custom_migrations`), and a one-way plaintext→encrypted
migration with backup/restore. It ships **off by default**, so day-one user value is ~zero, and it
adds a heavyweight native dependency to the release that the user just asked to cut.

**Options:**
- **B1 (full):** implement the SQLCipher opt-in this cluster.
- **B2 (recommended):** ship the **cheap, high-value half now** — document retention defaults in
  PRIVACY.md (the actually-missing user-facing gap) — and **split SQLCipher into its own tracked
  issue** for a later, focused effort with its own design doc. Keeps the release lean.

**Recommendation: B2.** Closes the documentation gap that matters; defers the heavy dep.

## Execution sequence

Grouped into logical commits/PRs. Order chosen so mechanical/low-risk lands first and the release
isn't blocked on the heaviest items.

1. **Cleanup** — close #41 (comment, no code); bump vitest #159 to ^4.x, run suite, fix 3→4 fallout.
2. **#37 clipboard resilience** — retry+backoff on `Clipboard::new()` failure; `clipboard:unavailable`
   event; `clipboard_status` command (three-file rule: invoke_handler + build.rs COMMANDS +
   permissions/default.toml). TDD.
3. **#49 + #35 IPC auth + audit** — window-label extraction; `Unauthorized` typed error; allowed-set
   assertion on destructive commands; emit to `SECURITY_LOG_TARGET` (label + command + args
   fingerprint). TDD.
4. **#53 + #54 protocol tests** — `mock_app` cross-plugin `capture:saved` shape test;
   `get_ipc_response` per-crate IPC-arg tests.
5. **#27 updater hardening** — downgrade floor (persist highest-seen version, refuse strict
   downgrade unless allow-rollback toggled, emit `SuppressedByPolicy { DowngradeBlocked }`); wire
   signature failures to terminal `RejectedBySignature` (inspect Tauri updater error, distinguish
   signature from network); kill-switch protocol doc. Item 1 per Decision A.
6. **#60 privacy** — PRIVACY.md retention defaults. SQLCipher per Decision B.
7. **#44 release-readiness gate** — `scripts/release-readiness.sh` runs §10.5 gates, prints
   pass/fail; release.yml refuses to run unless committed recently. (Last, so it gates the release.)
8. **Release** — version bump, tag `v*`, drive 7-job pipeline, approve `production-release` gate,
   verify signed Windows installer + latest.json publish.

## Discipline

- TDD on every code change (failing test first).
- One task = one commit; Conventional Commits; explicit `git add <path>`.
- Files >500 lines = split.
- New plugin commands = three-file edit (invoke_handler, build.rs COMMANDS, permissions/default.toml).
- Plan-as-source-of-truth: if a step is wrong, fix this doc first, then implement.
