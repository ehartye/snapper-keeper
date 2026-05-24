#!/usr/bin/env bash
# Files GitHub issues from the 2026-05-24 pre-release review synthesis.
# Idempotent: re-running creates duplicates — only run once.
set -u
REPO="ehartye/snapper-keeper"
TAG="pre-release-review"
SRC="docs/superpowers/reviews/2026-05-24-prerelease/synthesis.md"

mk() {
  local sev="$1"; local areas="$2"; local title="$3"; local body="$4"
  local label_args="-l severity:$sev -l $TAG"
  IFS=',' read -ra A <<< "$areas"
  for a in "${A[@]}"; do label_args+=" -l area:$a"; done
  # shellcheck disable=SC2086
  gh issue create -R "$REPO" -t "$title" $label_args -b "$body" \
    || echo "FAIL: $title"
}

# --------------------------------------------------------------------
# BLOCKERS
# --------------------------------------------------------------------

mk blocker "security" \
'[security] Stored XSS in FTS snippet rendering — sanitize + enable CSP' \
'**Raised by:** Adversary (F1), Testing (R1), Operator (R-Adv F1), Maintainer (R3) — release-blocker consensus

**Files:** `app/src/windows/library/SearchBar.tsx:173`, `crates/snk-library/src/search.rs:86,104`, `app/src-tauri/tauri.conf.json:85` (`csp: null`).

**Scope:** Replace `dangerouslySetInnerHTML` with React elements built by splitting on literal `<mark>`/`</mark>` tokens; SQLite never emits real HTML. Set a real CSP in `tauri.conf.json` (e.g. `default-src ''self''; script-src ''self''; style-src ''self'' ''unsafe-inline''`).

**Why it matters:** OCR text, clipboard text, and window titles flow into FTS snippets rendered as HTML — any captured string with an embedded `<img onerror>` or `<script>` payload executes inside the webview. With `csp: null` and a single capability granting full IPC to every window (see also: per-window capability split), the XSS chain reaches `paste_item` (synthetic Ctrl+V), autostart toggle, and hard-delete commands.

**Source:** [synthesis.md § CB1](../blob/main/'"$SRC"'#cb1)'

mk blocker "privacy,clipboard" \
'[privacy] Implement (or strip) sensitive-clipboard exclusion before v1' \
'**Raised by:** Adversary (F2), Testing (F4), Maintainer (F4 derivative), Operator (R-Adv F2) — release-blocker consensus

**Files:** `crates/snk-clipboard/src/watcher.rs:54-136`, `crates/snk-library/migrations/V002__clipboard_items.sql:11` (`sensitive` column dead), `crates/snk-library/src/settings.rs:86` (orphaned `app_blocklist` test fixture).

**Scope:** Either implement the promised exclusion:
- Honor Windows `ExcludeClipboardContentFromMonitors` / `CanIncludeInClipboardHistory`
- Honor macOS `org.nspasteboard.ConcealedType`
- Wire `clipboard.app_blocklist` setting (default: 1Password, KeePass, Bitwarden)
- Set `sensitive=1` on detection; popup filters them

Or strip every reference: drop the schema column, delete the orphaned setting test fixture, edit `PRIVACY.md`, edit the design spec, mark the README claim removed.

**Why it matters:** Password-manager copies, MFA codes, and clipboard secrets are currently stored in plaintext SQLite forever, despite explicit privacy claims to the contrary. Five independent surfaces reference the feature; zero implement it.

**Source:** [synthesis.md § CB2](../blob/main/'"$SRC"'#cb2)'

mk blocker "privacy,docs" \
'[privacy] Reconcile PRIVACY.md updater-disable and Microsoft Store claims' \
'**Raised by:** Adversary (F5), Operator (#2), Testing (R2), Maintainer (F4/F5/F12) — release-blocker consensus

**Files:** `PRIVACY.md:25-29`, `app/src/windows/settings/SettingsWindow.tsx` (no updater toggle row), `crates/snk-updater/src/plugin.rs:142-160` (unconditional check), no Microsoft Store build variant exists.

**Scope:** Either:
- Implement an `updater.enabled` setting + add the Settings → Updates toggle + check the setting in `plugin.rs` before the periodic call, **and** add (or remove the claim of) a Microsoft Store build variant that compiles out the updater entirely, **or**
- Edit `PRIVACY.md` to remove both fictional sentences before tagging v1.0.

**Why it matters:** A public privacy policy that does not match the binary is a regulatory/reputational issue and undermines every other privacy claim in the codebase.

**Source:** [synthesis.md § CB3](../blob/main/'"$SRC"'#cb3)'

mk blocker "security,refactor" \
'[security] Split Tauri capabilities per window per design spec §8.3' \
'**Raised by:** Adversary (F6/F9/F14), Operator (#3/#15), Maintainer (R4/N5) — release-blocker consensus

**Files:** `app/src-tauri/capabilities/default.json:5-28` (single capability for all 6 windows). Design spec §8.3 lines 557-565 specified `clipboard-popup.json` separately; the file does not exist.

**Scope:** Split into per-window capability files:
- `library.json` — full read + soft-delete
- `capture-overlay.json` — capture commands only
- `clipboard-popup.json` — clipboard read + `paste_item`, no destructive commands
- `annotate.json` — annotate commands only
- `settings.json` — settings + autostart

Today the overlay window and clipboard popup share the library window’s IPC set, including `hard_delete_capture`, `purge_trash`, `set_setting`, and `paste_item`.

**Why it matters:** Highest-leverage single defense — combined with CB1 (XSS), CB4 collapses blast radius from "full IPC" to "popup’s narrow surface."

**Source:** [synthesis.md § CB4](../blob/main/'"$SRC"'#cb4)'

mk blocker "ops,logging,reliability" \
'[ops] Add file-based logging with daily rotation + panic hook' \
'**Raised by:** Operator (#1), Adversary (R-Op #1), Maintainer (R2), Testing (R7) — release-blocker consensus

**Files:** `app/src-tauri/src/main.rs:67-71` (only `tracing_subscriber::fmt()` → stdout); `main.rs:1` (`windows_subsystem = "windows"` — no console, no stdout); no `tracing-appender`; no panic hook anywhere.

**Scope:**
- Add `tracing-appender` daily-rotating file appender under `app.path().app_log_dir()` (alongside the existing stdout layer for dev).
- Add `std::panic::set_hook(...)` that writes the panic + backtrace to `crashes/<timestamp>.log` in the same dir.
- Add a "Open log folder" button in Settings → About.
- Add a `[logging] Redacted<T>` newtype helper (see also low-priority issue) for fields that must not be logged.

**Why it matters:** Without file logging, every other release issue (CB1 XSS chain, CB6 migration failure, CP5 silent watcher death, CP1 updater errors, Maintainer F7/F8) is invisible in the field. Post-incident forensics relies on "user emails the dev a screenshot" — not viable.

**Source:** [synthesis.md § CB5](../blob/main/'"$SRC"'#cb5)'

mk blocker "reliability,tests" \
'[reliability] Pre-migration backup + forward-compatibility test' \
'**Raised by:** Operator (#5), Testing (F7), Maintainer (F8), Adversary (R-Op F5) — release-blocker consensus

**Files:** `crates/snk-library/src/migrate.rs:15-23` (the `recoverable` flag is set by string-matching the word "Backup" which never appears anywhere in the codebase); `crates/snk-library/src/plugin.rs:42-48` (no recovery branch); no `backups/` directory in the data layout.

**Scope:**
- Before running pending migrations: `wal_checkpoint(TRUNCATE)` then file-copy DB to `backups/pre-vN-<UTC-iso8601>.db`.
- On migration failure: restore latest backup, surface a user-facing toast with "Open data folder" button.
- Replace hardcoded `Migration { from: 0, to: 4 }` error literal with `migrations().current_version()` so the error message tells the truth.
- Add `crates/snk-library/tests/migration_forward_compat.rs`: load a committed fixture DB at each prior schema version, run migrations, assert query path still works.

**Why it matters:** Per Operator NI1 this is the single highest-EV operational disaster: v1.1 ships, migration fails on real-world DB shape no fixture exercised, error message lies about version, no backup, no log file. CB6 + this test is the cheapest insurance in the entire project.

**Source:** [synthesis.md § CB6](../blob/main/'"$SRC"'#cb6)'

# --------------------------------------------------------------------
# HIGH
# --------------------------------------------------------------------

mk high "updater,security" \
'[updater] Sign latest.json + downgrade floor + signature-error terminal handling' \
'**Raised by:** Adversary (F7/F8), Operator (#6/#7), Testing (F2/R3)

**Files:** `crates/snk-updater/src/plugin.rs:63-134`, `app/src-tauri/tauri.conf.json:110-112`, `.github/workflows/release.yml:259-306` (`latest.json` published unsigned).

**Scope:**
- Sign `latest.json` itself with minisign (the Tauri updater bundle artifacts are signed; the manifest is not).
- Store highest-ever-seen version in app data; refuse strict downgrades unless user toggles an opt-in "allow rollback".
- Distinguish signature errors from network errors: signature failure = terminal for process lifetime (disable updater, log, surface in Settings → About).
- Document the kill-switch protocol: how to mark a released version as "do not auto-install" (e.g. tag a new `latest.json` with `force_min_version`).

**Source:** [synthesis.md § CP1](../blob/main/'"$SRC"'#cp1)'

mk high "ci,updater" \
'[ci] Re-derive pubkey from TAURI_SIGNING_PRIVATE_KEY and diff against tauri.conf.json before build' \
'**Raised by:** Testing (F2), reinforced by Adversary (R-Test F2), Operator (R-Test F2)

**Files:** `.github/workflows/release.yml`, `app/src-tauri/tauri.conf.json:109` (`pubkey` literal).

**Scope:** Add a CI step before any signing work runs:
```
PRIV=$(echo "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d)
DERIVED=$(tauri signer sign --print-pub-key --private-key /dev/stdin <<< "$PRIV")
EMBEDDED=$(jq -r .plugins.updater.pubkey app/src-tauri/tauri.conf.json)
[ "$DERIVED" = "$EMBEDDED" ] || { echo "pubkey drift"; exit 1; }
```

**Why it matters:** If `tauri.conf.json:109` pubkey diverges from `TAURI_SIGNING_PRIVATE_KEY` (rotation, accidental regenerate, secret-store recovery), every update silently fails forever — and without file logging (CB5) nobody notices. Cheapest insurance against the most operationally expensive disaster.

**Source:** [synthesis.md § NI7 + T-U1](../blob/main/'"$SRC"'#t-u1)'

mk high "ci,security" \
'[ci] Split build job (no secrets) from sign+publish job (minimum-needed secrets)' \
'**Raised by:** Adversary (Round 2 N6 / A-U8)

**Files:** `.github/workflows/release.yml:172-183` (build step has full secrets in env).

**Scope:**
- `build` job: runs `cargo build --release` and `pnpm build` with **no** signing secrets. Uploads unsigned artifacts.
- `sign-and-publish` job: downloads artifacts, runs signing tools with minimum-needed secrets (`TAURI_SIGNING_PRIVATE_KEY`, `AZURE_*`), publishes the release. Use `environment: production-release` with a required reviewer.

**Why it matters:** Any transitive `build.rs` can read process env. `TAURI_SIGNING_PRIVATE_KEY` exposed during `cargo build` means a single malicious dep ships forged updates to every user. End-to-end pipeline-compromise chain (CP3 + this) — the credible attack.

**Source:** [synthesis.md § A-U8 + AC5](../blob/main/'"$SRC"'#a-u8)'

mk high "ci" \
'[ci] Pin CI actions by SHA + pin/verify Tesseract chocolatey source' \
'**Raised by:** Adversary (F10/F11), Operator (R-Adv F11)

**Files:** `.github/workflows/release.yml:66-76,90,308-310`, `.github/workflows/ci.yml`.

**Scope:**
- Pin all third-party actions by full commit SHA (`uses: org/action@<40-char-sha>`), then floating-tag comment.
- Drop `--prerelease` from `dotnet tool install sign`.
- Pin Tesseract chocolatey package version + SHA256-verify the installer.
- Replace `softprops/action-gh-release` with `gh release create` in an inline script — third-party action holds `contents:write`.

**Source:** [synthesis.md § CP3](../blob/main/'"$SRC"'#cp3)'

mk high "ocr,release" \
'[ocr] Bundle Tesseract for macOS or surface missing-dependency banner' \
'**Raised by:** Operator (#4), Maintainer (F5), Testing (R4)

**Files:** `.github/workflows/release.yml:66-76` (Tesseract bundled Windows-only), `crates/snk-ocr/src/sidecar.rs:80-86` (macOS fallback paths all require brew/macports), `README.md:24-27,56` (no Mac OCR caveat).

**Scope:** Pick one:
1. Bundle Tesseract on macOS in the release workflow (with hash-pinning per `[ci] Pin CI actions`).
2. Surface a first-run banner when `resolve_tesseract()` returns `None`: "OCR unavailable — install Tesseract via `brew install tesseract` and restart."
3. Adopt Apple Vision OCR on macOS (eliminates the sidecar dependency entirely).

**Why it matters:** OCR is a headline feature. Today macOS users without Homebrew get silent OCR failure with no UI indication. The library window will return empty search results for image-only content with no explanation.

**Source:** [synthesis.md § CP2](../blob/main/'"$SRC"'#cp2)'

mk high "security" \
'[security] Set a real Content-Security-Policy in tauri.conf.json' \
'**Raised by:** Adversary (F4)

**Files:** `app/src-tauri/tauri.conf.json:85` (`"csp": null`).

**Scope:** Set:
```
"csp": "default-src ''self''; script-src ''self''; style-src ''self'' ''unsafe-inline''; img-src ''self'' data: asset:; connect-src ''self'' ipc: tauri:"
```

**Why it matters:** Defense-in-depth even after CB1 fix. `csp: null` is "explicit opt-out of CSP"; no inline-script restriction, no image-src restriction, no connect-src restriction. Combined with the asset-protocol scope (A-U1) and dangerouslySetInnerHTML, the webview is exposed.

**Source:** [synthesis.md § A-U2](../blob/main/'"$SRC"'#a-u2)'

mk high "security" \
'[security] Tighten assetProtocol scope to captures/ and clipboard/ subdirs only' \
'**Raised by:** Adversary (F3)

**Files:** `app/src-tauri/tauri.conf.json:86-91`.

**Scope:** Restrict `assetProtocol.scope` from `$APPDATA/**` to specifically:
- `$APPDATA/captures/**`
- `$APPDATA/clipboard/**`
- `$APPDATA/annotations/**`

Explicitly exclude `library.db`, `library.db-wal`, `library.db-shm`, `backups/**`, `logs/**`.

**Why it matters:** With current scope `$APPDATA/**`, any XSS chain (CB1) can `fetch("asset://localhost/$APPDATA/library.db-wal")` and read the entire SQLite + WAL file, exfiltrating every screenshot, OCR text, clipboard entry, and tag.

**Source:** [synthesis.md § A-U1](../blob/main/'"$SRC"'#a-u1)'

# --------------------------------------------------------------------
# MEDIUM
# --------------------------------------------------------------------

mk medium "ops,logging,security" \
'[ops] Add security-events.log (sig failures, capability denials, paste_item, autostart toggles, panic captures)' \
'**Raised by:** Operator (Round 2 NI3)

**Scope:** Append-only, never-rotated log file separate from the general rotating log (CB5). Emit one line per:
- Updater signature verification failure
- Capability check denial
- `paste_item` invocation (window + content type)
- Autostart toggle
- Panic capture
- Hard-delete with item counts

Surface "Open security log" in Settings → About.

**Source:** [synthesis.md § O-U7](../blob/main/'"$SRC"'#o-u7)'

mk medium "ops,security,ipc" \
'[ops] IPC audit log for paste_item + destructive commands with source window label' \
'**Raised by:** Operator (Round 2 NI2/NI5), Adversary (compounds with A-U5)

**Scope:** Use the currently-unused `_app: AppHandle` parameter on destructive commands (`hard_delete_capture`, `purge_trash`, `paste_item`, `set_setting`) to log:
- Source window label (`app.get_webview_window_label()` equivalent)
- Command name
- Args fingerprint

Writes to security-events.log (see related issue).

**Why it matters:** Defense-in-depth + diagnostic surface. Combined with per-window capability split (CB4), this gives a tripwire for "popup tried to call hard_delete."

**Source:** [synthesis.md § O-U8 + A-U5](../blob/main/'"$SRC"'#o-u8)'

mk medium "ux,ops" \
'[ux] Add Settings → About panel with version + paths + updater status' \
'**Raised by:** Operator (#13), Maintainer (F1 amplified)

**Scope:** New Settings tab "About":
- App version (from Cargo.toml + git short-sha)
- Data dir + "Open" button (xdg-open / explorer / `open`)
- Log dir + "Open" button
- Updater: last check timestamp, current pubkey fingerprint, last update result
- Privacy link, License attribution link

**Why it matters:** First thing a user asks when reporting a bug is "what version am I on" and "where are my files." Currently neither has a UI affordance.

**Source:** [synthesis.md § CP7](../blob/main/'"$SRC"'#cp7)'

mk medium "clipboard,reliability" \
'[clipboard] Retry watcher init + expose health event for offline state' \
'**Raised by:** Operator (#10), Testing (F3/F14)

**Files:** `crates/snk-clipboard/src/watcher.rs:22-30`.

**Scope:**
- On `Clipboard::new()` failure: retry with exponential backoff (cap 60s).
- Emit `clipboard:unavailable` Tauri event so the popup can render an offline banner.
- Add a `clipboard_status` command returning `{ available: bool, last_error: Option<String> }`.

**Why it matters:** Today the watcher thread silently dies on init failure and never retries. User opens the popup expecting history; sees nothing; no feedback.

**Source:** [synthesis.md § CP5](../blob/main/'"$SRC"'#cp5)'

mk medium "clipboard,refactor,tests" \
'[clipboard] Replace SKIP_NEXT AtomicBool with Mutex<HashSet<hash>> for testable skip semantics' \
'**Raised by:** Maintainer (F13), Testing (NI6)

**Files:** `crates/snk-clipboard/src/watcher.rs:16-20`.

**Scope:** The global `SKIP_NEXT: AtomicBool` is racy (capture writes "skip" but watcher already processed the next event) and untestable. Replace with `Mutex<HashSet<u64>>` of content hashes recently emitted by us; watcher skips entries matching a hash within a small TTL window.

**Source:** [synthesis.md § M-U6](../blob/main/'"$SRC"'#m-u6)'

mk medium "clipboard,ux" \
'[clipboard] Either implement image paste OR hide image rows in popup with test' \
'**Raised by:** Testing (F11), Adversary (TT4 resolution: hide, not implement)

**Files:** `crates/snk-clipboard/src/commands.rs:29-35`.

**Scope:** Image rows are inserted into history but `paste_item` does not support images. Preferred resolution (per Adversary TT4): hide image rows from the popup entirely with a one-line filter + a test that asserts the popup query excludes `kind=image`. Do not add a paste-image path — keystroke-injecting arbitrary clipboard images is a worse blast radius than the current broken UX.

Optionally add a "View image" affordance from the library window.

**Source:** [synthesis.md § TT4 + T-U6](../blob/main/'"$SRC"'#tt4)'

mk medium "ocr,reliability" \
'[ocr] Bounded queue + startup sweep for captures missing ocr_text' \
'**Raised by:** Operator (#9)

**Files:** `crates/snk-ocr/src/queue.rs:21,41-110`.

**Scope:**
- Replace unbounded channel with `mpsc::channel(100)`; on full, drop oldest enqueued and emit `ocr:dropped` event.
- On app startup: `SELECT id FROM captures WHERE id NOT IN (SELECT capture_id FROM ocr_text)` and re-enqueue.

**Why it matters:** Quit-mid-queue captures get no OCR ever; on restart the work is lost. Burst capture (timed mode) can blow memory.

**Source:** [synthesis.md § O-U2](../blob/main/'"$SRC"'#o-u2)'

mk medium "ocr,security" \
'[ocr] Sandbox Tesseract sidecar (job object / sandbox-exec) + per-invocation timeout' \
'**Raised by:** Adversary (F12)

**Files:** `crates/snk-ocr/src/sidecar.rs:151-199`.

**Scope:**
- Run Tesseract in a restricted job object (Windows) / `sandbox-exec` profile (macOS) with no network and no filesystem access outside the temp input directory.
- Per-invocation timeout (e.g. 30s) — kill the child if it exceeds.

**Why it matters:** Tesseract has had CVEs (image-parsing). Today a malicious PNG processed by OCR runs with the full app token.

**Source:** [synthesis.md § A-U3](../blob/main/'"$SRC"'#a-u3)'

mk medium "release" \
'[release] Build macOS universal binary (--target universal-apple-darwin)' \
'**Raised by:** Operator (#14)

**Files:** `.github/workflows/release.yml:21-26`.

**Scope:** Tauri supports `--target universal-apple-darwin` to produce a single fat binary. Today the release publishes per-arch `.dmg` files. The design spec promised universal binary.

**Why it matters:** One artifact to distribute (no "which Mac do I have" friction), halves macOS sign+notarize CI time, eliminates the `.app.tar.gz` arch-collision problem captured in `MEMORY.md`. Keep per-arch builds in CI matrix for test exercise (see TT6); only collapse the release artifact.

**Source:** [synthesis.md § O-U5 + TT6](../blob/main/'"$SRC"'#o-u5)'

mk medium "release,docs" \
'[release] Document first-tag-must-be-plain-SemVer + add fallback endpoint' \
'**Raised by:** Operator (#6)

**Files:** `app/src-tauri/tauri.conf.json:110-112`.

**Scope:**
- Document in `docs/superpowers/` and `README.md`: first user-facing tag MUST be a non-prerelease SemVer; the updater endpoint resolves `/releases/latest/` which excludes prereleases.
- Add a fallback endpoint (e.g. raw URL to `releases/download/latest/latest.json`) so a missing GitHub Releases response does not break the updater.

**Source:** [synthesis.md § O-U6](../blob/main/'"$SRC"'#o-u6)'

mk medium "release,ci" \
'[release] Convert spec §10.5 manual checklist to enforced release-readiness gate' \
'**Raised by:** Testing (Round 2 NI3), Operator

**Scope:** Pick one:
- A GitHub issue template per checklist line, required-checked before tagging.
- `scripts/release-readiness.sh` that runs each gate as an actual command and prints a pass/fail table; the release workflow refuses to run unless `release-readiness.sh` was committed within the last 24h.

**Why it matters:** Today the spec §10.5 release checklist is a doc nobody reads at tag time. Without enforcement it has near-zero behavioral effect.

**Source:** [synthesis.md § T-U9](../blob/main/'"$SRC"'#t-u9)'

mk medium "ci,tests" \
'[ci] Split coverage reporting between pure-logic and IPC surfaces' \
'**Raised by:** Testing (F13), Operator (R-Test F13), Adversary (R-Test F13)

**Files:** `.github/workflows/ci.yml:82-85` (the regex exclusion list grew to include `plugin.rs|commands.rs|caret.rs|paste.rs|watcher.rs|queue.rs|...`).

**Scope:** Either drop the 90% gate or split:
- Logic coverage: 90% gate on pure functions (current behavior, but document what is excluded and why).
- IPC surface coverage: reported, not gated, with quarterly target.

**Why it matters:** Current threshold tells maintainers "coverage is good" while the IPC perimeter (where every user-facing failure lives) is excluded.

**Source:** [synthesis.md § CP4](../blob/main/'"$SRC"'#cp4)'

mk medium "ci,security,tests" \
'[ci] Add nightly cargo-audit + pnpm-audit + per-release SBOM (cyclonedx)' \
'**Raised by:** Operator (#11), Testing

**Scope:**
- Nightly workflow runs `cargo audit` and `pnpm audit`; opens an issue on new vulnerabilities.
- Release workflow runs `cargo cyclonedx` and `pnpm cyclonedx`; uploads `sbom.cdx.json` as a release asset alongside binaries.

**Why it matters:** Design §10.4 committed to this; supply-chain layer of the test pyramid (per T-U10). Without it, the "designed but not implemented" pattern continues into operations.

**Source:** [synthesis.md § O-U3](../blob/main/'"$SRC"'#o-u3)'

mk medium "ci,tests" \
'[ci] Add minimal E2E smoke per OS with uploaded runtime artifact' \
'**Raised by:** Testing (F1), Operator (R-Test F1), Maintainer (T1 reconciliation)

**Scope:**
- Test whether `windows-latest` runners support enough of an interactive desktop to launch Tauri (per CLAUDE.md the dev environment requires this; CI is currently compile-only).
- macOS: full E2E via `tauri-driver` (interactive desktop available).
- Windows: minimum "binary starts + library window paints + no panic" if interactive enough; otherwise stick with compile + sign + verify.
- Upload run artifact per OS: app log + screenshot of library window.

**Source:** [synthesis.md § CP8](../blob/main/'"$SRC"'#cp8)'

mk medium "ipc,refactor" \
'[ipc] Generate TS error types from Rust + enforce typed-error rule' \
'**Raised by:** Maintainer (F3), Testing (F12), Adversary (R-M F3)

**Files:** `crates/snk-ocr/src/plugin.rs:15` (`Result<String, String>`), `crates/snk-updater/src/plugin.rs:47,52`, `crates/snk-capture/src/commands.rs:64-85`, all `packages/*/src/types.ts` (no Error types exported).

**Scope:**
- Adopt `ts-rs` or `specta` to generate TS types from Rust error enums into `packages/*/src/types.ts`.
- Add `OcrError` / `UpdaterError` enums OR formally allow `Result<_, String>` for status-only commands with a comment.
- Add CI script: `grep -rn ''Result<.*, String>'' crates/*/src/{plugin,commands}.rs` must equal a known allowlist.

**Source:** [synthesis.md § CP10](../blob/main/'"$SRC"'#cp10)'

mk medium "security,ipc" \
'[ipc] Add per-window authorization middleware for destructive commands' \
'**Raised by:** Adversary (F14 / A-U5)

**Files:** `crates/snk-library/src/commands.rs:11-186`, `crates/snk-clipboard/src/commands.rs:21-53`.

**Scope:** Today 14+ commands take `_app: AppHandle` and ignore it. Wire a middleware that:
- Extracts caller window label
- For destructive commands (`hard_delete_capture`, `purge_trash`, `set_setting`, `paste_item`), asserts the label is in an allowed set
- Returns a typed `Unauthorized` error otherwise (and logs to security-events.log)

Companion to CB4 (per-window capabilities) — capabilities are the first gate, this is defense-in-depth.

**Source:** [synthesis.md § A-U5 + O-U8](../blob/main/'"$SRC"'#a-u5)'

mk medium "errors,logging" \
'[errors] LibraryError::Io should preserve OS message; consider removing From<io::Error>' \
'**Raised by:** Maintainer (F7)

**Files:** `crates/snk-library/src/error.rs:39-46`.

**Scope:** Today `From<io::Error>` blanks the path and stores only the `ErrorKind`. Capture the OS message + path via explicit construction at call sites, or remove the blanket `From` and force callers to wrap with context.

**Why it matters:** Compounds with CB5 (no logging). Land this **after** CB5 so the improvements show up in logs nobody currently sees.

**Source:** [synthesis.md § M-U2](../blob/main/'"$SRC"'#m-u2)'

mk medium "updater,arch" \
'[updater] Design unified state machine for signature-rejection and policy-suppression before implementing' \
'**Raised by:** Maintainer (Round 2 N4)

**Scope:** Before implementing CP1 / TT1 (two-mode auto-update), design the `UpdateStatus` enum so it covers:
- `Idle`
- `Checking`
- `Available { version, urgency }`
- `Downloading { progress }`
- `Ready { version }`
- `Installing`
- `RejectedBySignature` (terminal for process lifetime)
- `SuppressedByPolicy { reason }` (user toggled off, store edition, etc.)
- `Skipped { until: Instant }`

Avoids PR collision and bolt-on flags later. Single design doc, then implement.

**Source:** [synthesis.md § M-U10](../blob/main/'"$SRC"'#m-u10)'

mk medium "reliability" \
'[reliability] Panic boundary around plugin setup() with user-facing error' \
'**Raised by:** Testing (NI4) / Maintainer derivative

**Scope:** Today a plugin `setup()` panic = silent process death (the Tauri builder propagates). Wrap each plugin builder’s `setup()` body in `catch_unwind`; on panic, surface a user-facing modal "Plugin <name> failed to start — please file a bug" with a "Copy diagnostics" button (covers CB5 dependency).

**Source:** [synthesis.md § M-U9](../blob/main/'"$SRC"'#m-u9)'

mk medium "tests,ipc" \
'[tests] Add cross-plugin event protocol test using tauri::test::mock_app' \
'**Raised by:** Testing (F5)

**Files:** Emitters: `crates/snk-capture/src/commands.rs:13,24,40`, `crates/snk-annotate/src/commands.rs:76`. Consumer: `crates/snk-ocr/src/plugin.rs:43` (and the frontend `app/src/...`).

**Scope:** Use `tauri::test::mock_app` to register all three emitters + the OCR consumer, then assert payload shape compatibility for `capture:saved` (currently `trim_matches(''"'')` on the receiving side is the entire contract).

**Source:** [synthesis.md § T-U2](../blob/main/'"$SRC"'#t-u2)'

mk medium "tests,ipc" \
'[tests] Rust-side test that fires actual TS-binding JSON through tauri::test::get_ipc_response' \
'**Raised by:** Testing (F6)

**Scope:** TS bindings currently test that they call the right name; Rust doesn’t test that the JSON it accepts matches what TS sends. Add `tauri::test::get_ipc_response` tests per crate that fire a representative TS-shape payload through the Tauri IPC mock.

Locks IPC argument naming against silent renaming on either side.

**Source:** [synthesis.md § T-U3](../blob/main/'"$SRC"'#t-u3)'

mk medium "tests,ocr" \
'[tests] Commit hello-world OCR fixture + assert text extraction quality' \
'**Raised by:** Testing (F10)

**Files:** `crates/snk-ocr/tests/integration_test.rs:16-45`.

**Scope:** Today the integration test feeds a blank PNG and asserts "no panic." Commit a 256x64 PNG with the literal text "hello world" and assert extracted text contains "hello world" (case-insensitive, fuzz-tolerant).

**Source:** [synthesis.md § T-U5](../blob/main/'"$SRC"'#t-u5)'

mk medium "tests,errors" \
'[tests] Snapshot-test wire shape of Library(LibraryError) wrapper across crates' \
'**Raised by:** Testing (F12)

**Scope:** `LibraryError` is wrapped as `CaptureError::Library(LibraryError)`, `ClipboardError::Library(...)`, etc. Each wrapper crate must serialize the inner enum identically across the IPC boundary. Add insta snapshot tests per crate for the serialized wire shape; CI fails on drift.

**Source:** [synthesis.md § T-U7](../blob/main/'"$SRC"'#t-u7)'

mk medium "arch,tests" \
'[arch] Extract worker_step from background spawn blocks for unit testability' \
'**Raised by:** Testing (Round 2 NI2)

**Scope:** Every `tokio::spawn` / `std::thread::spawn` in the codebase wraps an entire `loop { ... }` body. Extract `fn worker_step(state: &mut State) -> StepResult` from each so the spawn block becomes a 3-line wrapper around a testable unit. Apply to: clipboard watcher, OCR queue, updater interval.

**Source:** [synthesis.md § T-U8 + NI2](../blob/main/'"$SRC"'#ni2)'

mk medium "security,tests" \
'[security] OWASP XSS corpus property test against SearchBar snippet rendering' \
'**Raised by:** Testing (Round 2 NI7), Adversary cross-pollination

**Scope:** Single React/jsdom property test:
- For each string in an OWASP XSS cheat-sheet corpus, render through `SearchBar` snippet path.
- Assert `window.__TAURI__` is undefined OR the IPC mock received no invocation.

Single test catches CB1 + CB4 + A-U7 chain in one assertion. Infrastructure already exists in the test suite.

**Source:** [synthesis.md § NI6](../blob/main/'"$SRC"'#ni6)'

mk medium "ci,docs" \
'[ci] verify-docs.sh CI gate — PRIVACY.md / README / spec §10.5 claims map to code or test' \
'**Raised by:** Testing (Round 2 NI1), pattern surfaced by all four perspectives

**Scope:** New `scripts/verify-docs.sh` that:
- Parses bullet claims from PRIVACY.md
- Parses feature claims from README
- Parses spec §10.5 manual checklist
- For each, asserts a matching tagged code path or `#[test]` exists (e.g. `// VERIFIED: privacy-md/sensitive-clipboard`).
- CI runs the script; fails on missing tag.

**Why it matters:** Three of four perspectives independently identified doc/code drift (NI1 meta-finding). This is the systemic fix that catches the next instance.

**Source:** [synthesis.md § NI4 + NI1](../blob/main/'"$SRC"'#ni4)'

mk medium "privacy,docs" \
'[privacy] Document retention defaults + offer SQLCipher opt-in for local data at rest' \
'**Raised by:** Adversary (F13)

**Scope:**
- Document default retention in PRIVACY.md: captures = forever, clipboard = N items / N days, OCR = lives with capture.
- Add Settings → Privacy → "Encrypt library" (SQLCipher with OS-keychain-derived key). Off by default (per TT5 — preserves "send me your DB file" support workflow).
- If enabled, document in About panel: "Encryption is on; support requests cannot include DB."

**Source:** [synthesis.md § A-U4 + TT5](../blob/main/'"$SRC"'#a-u4)'

mk medium "validation,security" \
'[validation] Validate PNG magic bytes + state_json schema + max payload size' \
'**Raised by:** Adversary (F15)

**Files:** `crates/snk-annotate/src/commands.rs:8-37,39-79`.

**Scope:** `save_annotation` and `derive_capture` accept arbitrary `Vec<u8>` and treat as PNG; arbitrary JSON for `state_json`. Add:
- PNG: assert first 8 bytes match the PNG signature; reject otherwise.
- `state_json`: typed deserialize against an `AnnotationState` struct (already partially exists).
- Max payload: e.g. 16 MiB; reject larger.

**Source:** [synthesis.md § A-U6](../blob/main/'"$SRC"'#a-u6)'

mk medium "cleanup,ipc" \
'[cleanup] Delete or wire dead @snk/ocr and @snk/updater TS packages' \
'**Raised by:** Maintainer (F1), Testing (R6), Adversary (R-M F1), Operator (R-M F1)

**Files:** `packages/snk-ocr/`, `packages/snk-updater/`, `app/vitest.config.ts:41-42`.

**Scope:** Per TT2: build the About panel first (`[ux] Add Settings → About panel`); if the About panel consumes these packages for diagnostics, keep them. Otherwise delete the packages, prune the vitest config entry, and drop from the workspace.

End the "ships but unused" state either way.

**Source:** [synthesis.md § CP6 + TT2](../blob/main/'"$SRC"'#cp6)'

# --------------------------------------------------------------------
# LOW
# --------------------------------------------------------------------

mk low "refactor,ci" \
'[refactor] Use snk_library::LibraryState re-export + lint for ::plugin:: reach-ins' \
'**Raised by:** Maintainer (F2), Testing (R5), Adversary (R-M F2)

**Files:** `crates/snk-annotate/src/commands.rs:3`, `crates/snk-clipboard/src/commands.rs:9`, `crates/snk-clipboard/src/plugin.rs:13`, `crates/snk-ocr/src/plugin.rs:6`. Clean re-export already at `crates/snk-library/src/lib.rs:21`.

**Scope:** Mechanical sweep to use `snk_library::LibraryState` instead of `snk_library::plugin::LibraryState`. Add CI check: `grep -rn ''snk_library::plugin::'' crates/` must be empty.

**Source:** [synthesis.md § CP9](../blob/main/'"$SRC"'#cp9)'

mk low "tests,cleanup" \
'[tests] Shared test_support fresh_db helper to remove tempdir leaks' \
'**Raised by:** Maintainer (F10), Testing (F9)

**Files:** `crates/snk-library/src/captures.rs:332-337`, `clipboard.rs:252-256`, `search.rs:171-176`, `settings.rs:52-57`, `crates/snk-capture/src/orchestrate.rs:112-117`, `ocr.rs:66-70`, `tags.rs:173-177`.

**Scope:** Extract `pub(crate) fn fresh_db() -> (TempDir, Db)` in a shared `test_support` module; return both so `Drop` runs at end of test. Remove the `mem::forget(dir)` workaround copy-pasted across 7 sites.

**Source:** [synthesis.md § CP11](../blob/main/'"$SRC"'#cp11)'

mk low "updater" \
'[updater] Set MissedTickBehavior::Delay + collapse redundant first tick' \
'**Raised by:** Operator (#8)

**Files:** `crates/snk-updater/src/plugin.rs:144-160`.

**Scope:** `tokio::time::interval` defaults to `Burst` behavior, meaning a flaky network burst will fire repeated checks rapidly. Set `interval.set_missed_tick_behavior(MissedTickBehavior::Delay)`. Also: the first tick fires immediately + the explicit "check on startup" call duplicates it; pick one.

**Source:** [synthesis.md § O-U1](../blob/main/'"$SRC"'#o-u1)'

mk low "refactor" \
'[refactor] Split AnnotateCanvas.tsx into shape-specific modules under shapes/' \
'**Raised by:** Maintainer (F6)

**Files:** `app/src/windows/annotate/AnnotateCanvas.tsx` (523 LoC > 500 ceiling per CLAUDE.md).

**Scope:** Extract per-shape modules: `shapes/rect.ts`, `shapes/arrow.ts`, `shapes/text.ts`, `shapes/blur.ts`. Canvas component becomes a dispatcher.

**Source:** [synthesis.md § M-U1](../blob/main/'"$SRC"'#m-u1)'

mk low "refactor,ci" \
'[refactor] Load tray icons from disk OR add CI script verifying THEMES keys match Rust constants' \
'**Raised by:** Maintainer (F9)

**Files:** `app/src-tauri/src/main.rs:12-37`, `app/src/lib/theme.ts:9-31`.

**Scope:** Theme registration is fanned out across 5 places. Pick one:
- Load tray icons from disk by theme key at runtime (collapse Rust constants).
- CI script: `THEMES` exported keys in `theme.ts` must equal the const array in `main.rs`.

**Source:** [synthesis.md § M-U3](../blob/main/'"$SRC"'#m-u3)'

mk low "cleanup" \
'[cleanup] Drop unused thiserror dep + resolve "later phase" comment in snk-hotkeys' \
'**Raised by:** Maintainer (F11)

**Files:** `crates/snk-hotkeys/Cargo.toml:13`.

**Scope:** `thiserror` is in the dep list but no `derive(Error)` macros appear in the crate. Drop the dep. Also: a "later phase" comment in the crate is stale; resolve or remove.

**Source:** [synthesis.md § M-U4](../blob/main/'"$SRC"'#m-u4)'

mk low "docs" \
'[docs] Declare Linux as dev-only; add "Builds on Linux for dev convenience" note' \
'**Raised by:** Maintainer (F12)

**Files:** `README.md:53-62`, `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md:9`.

**Scope:** README mentions Linux build steps; design spec excludes Linux as a target. Add explicit "Linux is supported as a dev convenience only; no signing/release pipeline" note in README to reconcile.

**Source:** [synthesis.md § M-U5](../blob/main/'"$SRC"'#m-u5)'

mk low "docs" \
'[docs] Consolidate dev-setup; CLAUDE.md and README cross-link to single source' \
'**Raised by:** Maintainer (F14)

**Scope:** Today the dev-setup content is duplicated between `README.md` and `CLAUDE.md` with platform tips in only one. Pick one as canonical; the other links to it. Reduces doc drift surface.

**Source:** [synthesis.md § M-U7](../blob/main/'"$SRC"'#m-u7)'

mk low "docs" \
'[docs] Document UUIDv7 invariant (monotonicity + no HTML/JSON specials) in snk-library/src/lib.rs' \
'**Raised by:** Maintainer (Round 2 N2)

**Scope:** UUIDv7 monotonicity + that the format contains no HTML or JSON special characters is an implicit invariant relied on at 4+ call sites (`captures.rs`, `clipboard.rs`, search ranking, event payloads). Add a module-level docstring in `snk-library/src/lib.rs` documenting both, so a future contributor cannot accidentally swap in a different ID scheme.

**Source:** [synthesis.md § M-U8](../blob/main/'"$SRC"'#m-u8)'

mk low "logging,security" \
'[logging] Redacted<T> newtype for sensitive fields in tracing output' \
'**Raised by:** Adversary (F16)

**Files:** `app/src-tauri/src/main.rs:67-71` (env-controlled `SNK_LOG`).

**Scope:** Add a `Redacted<T>` newtype whose `Display`/`Debug` impl prints `<redacted>` regardless of the inner value. Wrap clipboard content, OCR text, file paths-as-data in `Redacted<...>` at tracing call sites. Cheap guardrail against accidental leakage to a user-controlled log sink.

**Source:** [synthesis.md § A-U7](../blob/main/'"$SRC"'#a-u7)'

mk low "tests" \
'[tests] Serialize env-mutating tests in snk-ocr::sidecar (Mutex or serial_test)' \
'**Raised by:** Testing (F8)

**Files:** `crates/snk-ocr/src/sidecar.rs:266-269`.

**Scope:** Tests mutate `OnceLock`/process env; flake risk acknowledged in comments but unaddressed. Use `serial_test::serial` or a shared `Mutex` to serialize.

**Source:** [synthesis.md § T-U4](../blob/main/'"$SRC"'#t-u4)'

mk low "tests" \
'[tests] Assert migration count matches latest version applied' \
'**Raised by:** Testing (R8)

**Scope:** Add a test asserting `migrations().current_version() == migrations().count()` so future hardcoded version drift (see CB6) is caught at test time.

**Source:** [synthesis.md § T-U11](../blob/main/'"$SRC"'#t-u11)'

mk low "ci,release" \
'[ci] Replace cmd.exe smoke with vendored test artifact + verify roundtrip' \
'**Raised by:** Operator (#12)

**Files:** `.github/workflows/release.yml:118-142`.

**Scope:** The current smoke test signs `cmd.exe` (sketchy auditability; will trigger antivirus). Vendor a small test artifact (a stub exe with our own embedded manifest) and roundtrip: sign it then `signtool verify` against the cert. Also gives a natural insertion point for NI8 (verify-against-our-pubkey).

**Source:** [synthesis.md § O-U4 + NI8](../blob/main/'"$SRC"'#o-u4)'

mk low "release" \
'[release] Canary procedure for first user-facing tag' \
'**Raised by:** Operator (Round 2 NI4)

**Scope:** Cut v0.1.0 as `prerelease: true` first. Use it for ~1-2 weeks of self-installation. Promote to non-prerelease tag only after at least one full upgrade cycle confirms the updater works. Document the canary process in `docs/superpowers/release-process.md`.

**Source:** [synthesis.md § O-U9](../blob/main/'"$SRC"'#o-u9)'

mk low "docs" \
'[docs] Update spec §10.1 to include Supply-chain layer in test pyramid' \
'**Raised by:** Testing (Round 2 NI5)

**Scope:** Test pyramid in spec §10.1 enumerates unit/integration/e2e/manual. Add Supply-chain as a fourth layer (covers cargo-audit, pnpm-audit, SBOM). Companion doc change to `[ci] Add nightly cargo-audit + pnpm-audit`.

**Source:** [synthesis.md § T-U10](../blob/main/'"$SRC"'#t-u10)'

# --------------------------------------------------------------------
# Eric add-on
# --------------------------------------------------------------------

mk high "capture,ux,privacy" \
'[capture] Hide snapper-keeper own windows during capture (setting, enabled by default)' \
'**Raised by:** Eric (post-synthesis add-on)

**Scope:** New setting `capture.hide_own_windows` (default: `true`). When set, every snapper-keeper-owned window (library, settings, popup, annotate, region-select overlay itself when not the active capture) is hidden BEFORE the capture pixel grab and restored AFTER.

**Behavior matrix:**
- Region select: overlay stays visible (it IS the capture UI); other windows hidden.
- Full screen / window capture: ALL snapper-keeper windows hidden before grab; restored to prior visibility after.
- Timed capture: same as full screen for each frame.
- Hidden state should be transient — restore must happen even if the capture throws.

**Why it matters:** Today a user pressing the capture hotkey while the library window is open will see the library window itself in the capture. This is a basic capture-quality bug and a small privacy leak (a screenshot intended for sharing may include indexed thumbnails of prior captures).

**Files (likely):** `crates/snk-capture/src/orchestrate.rs` + window-management call into Tauri `AppHandle`.

**Tests:** Unit-testable once windows are passed via dependency injection (see `[arch] Extract worker_step`).'

echo
echo "DONE."
