# Operator perspective — Round 1

Reviewing snapper-keeper at the cut-the-first-public-release moment. Lens: 3am incident response, deploy/monitor/debug/scale on an unattended desktop fleet I can't SSH into.

---

## Findings

### 1. No file-based logging anywhere — when a user says "it crashed" I have nothing

- **What:** `main.rs:67-71` initializes `tracing_subscriber::fmt()` only — that writes to stdout. A Tauri 2 Windows bundle is built with `windows_subsystem = "windows"` (`main.rs:1`), which means there is **no console, so stdout is discarded**. On macOS launched from Finder there is also no terminal. There is no `tracing_appender`, no file appender, no per-day rotation, nothing in `Cargo.toml` (`app/src-tauri/Cargo.toml:26-27` lists `tracing` and `tracing-subscriber` but no `tracing-appender`). I greps the whole `crates/` tree — zero matches for `rolling`, `file_appender`, `tracing_appender`. The design doc explicitly promised the opposite (`docs/superpowers/specs/2026-05-20-snapper-keeper-design.md:637`): *"Rust: tracing with file appender → `library/logs/snapper-keeper.log` · daily rotate · 14-day retention."* and the error spec (§9.1) promises a `logs/crash-<ts>.json` for panics. Neither exists.
- **Where:** `app/src-tauri/src/main.rs:67-71` (logging init); design doc §9.3 line 637; no panic hook anywhere.
- **Why it matters:** 3am scenario: a user files an issue "the app froze after I copied a 200MB image" — there is literally no artifact to ask them for. Every `warn!`/`error!` already scattered throughout the code (`crates/snk-ocr/src/queue.rs:36, 76, 90, 95, 102, 105`; `crates/snk-updater/src/plugin.rs:103, 111, 125, 149, 157`; `crates/snk-clipboard/src/watcher.rs:27, 84, 87, 117, 131, 134`) goes to `/dev/null` once the app is installed. The OCR worker can silently die, the clipboard watcher can crash on a malformed clipboard, the updater can be in a permanent `Error` state — all invisible.
- **Confidence:** High.
- **Suggested alternative:** Before v1, add `tracing-appender = "0.2"` to `app/src-tauri/Cargo.toml` and layer a `RollingFileAppender::new(Rotation::DAILY, log_dir, "snk.log")` over the existing stdout layer. Compute `log_dir` from `app.path().app_log_dir()` (Tauri exposes the platform-correct location). Add a `tray:open-log-folder` menu item — the design doc promised "Settings → Open log folder for support" (`docs/superpowers/specs/...:641`). Add a `std::panic::set_hook` that writes the panic + backtrace to a `crashes/` subfolder with timestamped filename so a user can zip-and-send.

---

### 2. PRIVACY.md makes a promise the binary cannot keep

- **What:** `PRIVACY.md:25` states: *"You can disable update checks in Settings."* The Settings window source (`app/src/windows/settings/SettingsWindow.tsx`) has rows for clipboard tracking, OCR, autostart, theme — **no row for updater enable/disable**. `getSetting` lookups in that file: `clipboard.track_files`, `ocr.enabled`, autostart — no `updater.enabled` key plumbed. The updater plugin (`crates/snk-updater/src/plugin.rs:144-160`) unconditionally schedules the startup + 24h timer with no gate.
- **Where:** `PRIVACY.md:25` (the promise); `crates/snk-updater/src/plugin.rs:144-160` (no opt-out); `app/src/windows/settings/SettingsWindow.tsx` (no UI). Also `PRIVACY.md:27-29` claims a "Microsoft Store edition makes zero network requests" with the updater "compiled out" — there is no Microsoft Store edition in this codebase, no feature flag, no build matrix variant.
- **Why it matters:** This is a *legal/policy* gap, not a code gap. A privacy-conscious user reads the policy, doesn't see a toggle in Settings, files a public complaint, and the policy now reads as misleading. On a corporate fleet, IT will read PRIVACY.md to decide whether to allow this binary and then discover the toggle is fictional.
- **Confidence:** High.
- **Suggested alternative:** Either (a) implement a `updater.enabled` setting and gate the timer/check on it before v1, or (b) edit PRIVACY.md before tagging to remove the "disable in Settings" sentence and the Microsoft Store paragraph. Option (a) is cheap — 15 lines including a checkbox row.

---

### 3. The capability file gives the clipboard popup full plugin access — design said "privilege isolation"

- **What:** `app/src-tauri/capabilities/default.json` is the *only* capability file and lists ALL windows (`library`, `capture-overlay`, `capture-toolbar`, `annotate`, `clipboard-popup`, `settings`) against the union of all plugin permissions (`snk-library:default`, `snk-capture:default`, `snk-annotate:default`, `snk-clipboard:default`, `snk-ocr:default`, `snk-updater:default`). The design (`docs/superpowers/specs/2026-05-20-snapper-keeper-design.md:540`) explicitly committed to: *"The clipboard popup window only has access to `snk-library:read` and `snk-clipboard:paste` — it cannot mutate captures, run OCR, or change hotkeys. Privilege isolation is enforced by the framework, not by frontend convention."* and showed a separate `app/capabilities/clipboard-popup.json` (§8.3 lines 557-565). That file doesn't exist (`ls capabilities/` returns only `default.json`).
- **Where:** `app/src-tauri/capabilities/default.json:5`; missing `app/src-tauri/capabilities/clipboard-popup.json`; design §8.3.
- **Why it matters:** Not the most operationally on-fire issue but it has clear blast-radius implications. If a future renderer-side XSS-style bug ever lands in the popup (Tauri webview is not airtight against all CSP gaps; `csp: null` is set at `tauri.conf.json:85`), the popup can invoke `snk_library::soft_delete_capture`, `hard_delete_capture`, `purge_trash`, etc. The popup is the smallest, most-frequently-shown surface; design correctly identified it should be the most locked-down.
- **Confidence:** High.
- **Suggested alternative:** Split into `default.json` (everything but popup) and `clipboard-popup.json` per the design. Even if the actual delete-from-popup blast radius is small for v1, the discipline of "one window, one capability" pays off the first time you add a plugin.

---

### 4. macOS users get OCR silently broken — Tesseract is only bundled for Windows

- **What:** Release workflow `release.yml:66-76` bundles tesseract on Windows only (`if: runner.os == 'Windows'`). No equivalent step for `macos-latest` or `macos-15-intel`. The resolver `crates/snk-ocr/src/sidecar.rs:48-98` falls through bundled → `which` → common install paths. On macOS the fallbacks are `/opt/homebrew/bin/tesseract`, `/usr/local/bin/tesseract`, `/opt/local/bin/tesseract` — none of which exist on a stock Mac. README:54-57 documents installing tesseract for *dev*, not for *users*, but a user who installs the signed `.dmg` will never see that page.
- **Where:** `.github/workflows/release.yml:66-76` (Windows-only branch); `crates/snk-ocr/src/sidecar.rs:80-86` (macOS fallbacks all require brew/macports); design `§13 row 11` promised: *"Tesseract sidecar for OCR; async on capture — Cross-platform, no native deps; UI never blocked"*.
- **Why it matters:** Failure mode is *silent*. OCR is fire-and-forget (`crates/snk-ocr/src/plugin.rs:43-61`). When tesseract is missing, `run_tesseract` retries 3x with 0/1/3-second backoffs (`sidecar.rs:122-149`), each attempt logs a `warn!` to a log file we don't have (finding #1). The capture saves, search returns no hits for the text in the image, and the user concludes "search is broken" — never realizing OCR didn't run. The README headline-features OCR + search as one of four key capabilities. macOS users get a partial product with no in-app explanation.
- **Confidence:** High.
- **Suggested alternative:** Either (a) add a tesseract-bundling step in the macOS release jobs (`brew install tesseract` then copy the brew prefix into `resources/tesseract/`), (b) surface a one-time first-run banner on macOS when `resolve_tesseract()` returns None ("OCR needs Tesseract — install with `brew install tesseract`"), or (c) flip to Apple's native Vision OCR on macOS (the design called this out as a v1.1 enhancement at line 711 but it would land here cheaper than discovering broken-OCR-tickets post-release).

---

### 5. No SQLite backup before migrations — design committed to it, code doesn't do it

- **What:** Design §9.1 row "Migration failure": *"Auto-rollback (transaction) · pre-migration snapshot in `backups/` · block start with explainer + restore option"*. Actual implementation (`crates/snk-library/src/migrate.rs:15-23`) calls `migrations().to_latest(conn)` and on failure returns a `LibraryError::Migration { recoverable: e.to_string().contains("Backup") }`. The `recoverable` flag is set by **string-matching the error message for the word "Backup"** — which never appears in `rusqlite_migration` errors. No backup file is ever written. No `backups/` directory exists or is created. `crates/snk-library/src/plugin.rs:42-43` calls `Db::open` directly, which calls `migrate` immediately; if it fails, `init` returns a string error and the plugin setup itself fails — at which point the app probably crashes (no upstream handler catches that into a user-visible "your library is corrupted, restore from..." dialog).
- **Where:** `crates/snk-library/src/migrate.rs:15-23` (no backup, lies about recoverable); `crates/snk-library/src/plugin.rs:42-48` (no recovery path on failure); design §9.1.
- **Why it matters:** A failed migration on a 200MB library with months of captures bricks the app and silently loses everything (data is still on disk but the user has no path to recover). The current `recoverable` flag pretending to be useful is worse than not having one — it gives the UI a value to read and act on that is structurally always false. 3am scenario: user updates from v1.0 to v1.1 with a new migration; migration fails on a corner case the dev didn't test; app refuses to start; user opens an issue; only the user's own filesystem has the data; no in-app restore path.
- **Confidence:** High.
- **Suggested alternative:** Before any migration runs, copy the `.db` file to `backups/pre-vN-YYYYMMDD-HHMMSS.db` (use `std::fs::copy` after acquiring an `EXCLUSIVE` lock and forcing a `PRAGMA wal_checkpoint(TRUNCATE)` so the copy is consistent). Keep last 5. On migration failure, restore the latest backup, log loudly, and surface a startup banner. Drop the broken `recoverable` heuristic.

---

### 6. Updater endpoint URL pattern depends on GitHub "latest release" semantics — first release must be tagged carefully or every install breaks

- **What:** `app/src-tauri/tauri.conf.json:110-112` hard-codes the endpoint: `https://github.com/ehartye/snapper-keeper/releases/latest/download/latest.json`. GitHub's `/releases/latest/` redirect resolves only to the most recent release that is **not a draft and not a prerelease**. The release job sets `prerelease: false` (`release.yml:312`). But the existing tag is `v0.0.0-test13` — a SemVer prerelease (`-test13` suffix), which `actions-gh-release@v2` may or may not auto-detect as prerelease depending on its heuristic. If the first tag pushed is prerelease-flavored and `prerelease: false` is set, that release becomes "latest" and pins every installed binary to it forever (until a non-prerelease tag is cut). If the team ever cuts a prerelease deliberately (e.g. `v1.1.0-beta.1`) with `prerelease: true`, that release silently gets *no* update flow because `/releases/latest/` skips it.
- **Where:** `app/src-tauri/tauri.conf.json:110-112`; `release.yml:312`; design §11 promises "stable only for v1" but does not document the "first tag must be a non-prerelease" prerequisite.
- **Why it matters:** Cutting v1.0.0 is the only chance to do this right. If the team's first public tag is something like `v0.1.0-rc1` to be cautious, every user installs a build that thinks `/releases/latest/` is RC1 forever and the eventual `v0.1.0` real release never reaches them. The detection of the failure mode is slow because the updater silently treats "no newer version" as healthy (`updater.check() → Ok(None)` at `plugin.rs:118-122`).
- **Confidence:** Medium-High (depends on the team's release-naming discipline, which the current `v0.0.0-test13` history hints is loose).
- **Suggested alternative:** Add a release checklist line: "first user-facing tag MUST be plain SemVer (`v0.1.0`, not `v0.1.0-rc1`)." Also harden the endpoint by changing it to a versioned URL pattern OR adding a second fallback endpoint that points at the latest stable tag explicitly. For prereleases, document explicitly that they go out-of-band.

---

### 7. Updater has no kill switch / no way to unship a bad build

- **What:** Once `latest.json` is published with a higher version, the in-process updater (`snk-updater/src/plugin.rs:63-117`) immediately downloads and stages it on launch + every 24h. The only safety check is the Ed25519 signature on the bundle (good). But if a signed v1.0.1 ships and crashes on launch (e.g. migration bug per finding #5), the user can't roll back: their installed binary already updated to v1.0.1, and the team has no way to "yank" the release. Deleting the GitHub release/asset would just leave clients failing the update check (which they handle gracefully — `Ok(None)` path) but doesn't downgrade anyone. The user would need a v1.0.2 hot-fix that supersedes v1.0.1.
- **Where:** `crates/snk-updater/src/plugin.rs:63-134` (no version-skip or yank list); release pipeline has no rollback story; no docs of the rollback procedure anywhere in `docs/`.
- **Why it matters:** This is the classic auto-update operator question. The mitigation is "ship a hotfix and let auto-update push it within 24h" — but the hotfix may not be ready for hours. Some users will be stuck on the broken version while devs debug. The signed-installer model can never *force* a downgrade.
- **Confidence:** High.
- **Suggested alternative:** Document the kill-switch protocol explicitly: (1) cut a hotfix release (vNext.NEXT.patch) ASAP; (2) edit `latest.json` body asset on the previous (broken) release to point its `version` field to the *new* version so any client still on the broken build forces forward; (3) if the breakage prevents launching at all, document that "user must download the installer manually from the Releases page" — and put that link in PRIVACY.md / README. Consider adding a *server-side* deny-list endpoint as a future hardening step (out of scope for v1 but worth noting).

---

### 8. Updater scheduler has a redundant first tick that silently double-checks at startup

- **What:** `crates/snk-updater/src/plugin.rs:144-160`:
  ```
  sleep(5s)
  do_update_check()           // first explicit check
  let interval = tokio::time::interval(24h)
  interval.tick().await        // tokio::time::interval's first tick fires IMMEDIATELY
  loop { interval.tick().await; do_update_check() }
  ```
  `tokio::time::interval` documents that the first `tick()` resolves immediately (epoch tick). The code calls one `interval.tick()` *before* the loop, *consuming* the immediate tick — but then the loop's first `interval.tick()` fires 24h later. So total wall-clock: check at +5s, idle, check at +24h05s, then every 24h. That's actually correct behavior, but the code reads as if the author was unsure about the first-tick semantics (the comment-less double-tick is a smell). More concerning: if `do_update_check` itself takes a long time (e.g. download stage 80 MB of installer on a slow link), the interval tick is `MissedTickBehavior::Burst` by default, which can cause a queued tick to fire immediately when the current call returns.
- **Where:** `crates/snk-updater/src/plugin.rs:147-159`.
- **Why it matters:** Low-likelihood but worth noting: a network-flaky user could end up with the updater hammering check-then-download in a tight loop when their connection becomes intermittent during a download. Combined with finding #1 (no log file), they won't see the symptom — they'll just have CPU and network usage they can't explain.
- **Confidence:** Medium (default `MissedTickBehavior::Burst` is the documented Tokio behavior; the failure pattern is theoretical but real).
- **Suggested alternative:** Set `interval.set_missed_tick_behavior(MissedTickBehavior::Delay)`. Also collapse the two ticks: `sleep(5s); do_check(); loop { sleep(24h); do_check(); }` reads cleaner.

---

### 9. OCR queue is unbounded — a quick capture spree can backlog memory forever

- **What:** `crates/snk-ocr/src/queue.rs:21` uses `mpsc::unbounded_channel()`. A capture event (`capture:saved`) triggers an enqueue via the listener at `crates/snk-ocr/src/plugin.rs:43-61`. Tesseract is single-threaded per invocation and runs sequentially in `worker`. A user who hits the capture hotkey 50 times in 30 seconds (e.g. a power user documenting a multi-step bug) enqueues 50 jobs; Tesseract on a typical screenshot is ~1-3s; the queue drains over 50-150 seconds. Each queued `OcrJob` carries a `String` (capture_id) + a `PathBuf` (image_path) + a `String` (language) — small per-item, BUT the queue holds references to image *paths*, and each capture file is 1-5 MB on disk. So the *memory* footprint per pending job is small, but disk consumption races ahead of OCR. Worse: if the queue is processing image N when the user quits, image N+1..N+M never get OCR'd — they're indexed in SQLite but FTS5 has no text for them. There's no resumption on next launch.
- **Where:** `crates/snk-ocr/src/queue.rs:21` (unbounded channel); `crates/snk-ocr/src/queue.rs:41-110` (worker has no persistence between runs); no startup sweep that looks for captures missing OCR text.
- **Why it matters:** Symptoms cluster around "search doesn't find my screenshots from last Tuesday" — and the user has no way to trigger a re-OCR. Also: hitting capture-hotkey while the queue is backed up means newly captured images appear in the gallery instantly but are unsearchable for minutes.
- **Confidence:** Medium-High (architectural rather than incident-likely, but the recovery story is missing).
- **Suggested alternative:** (a) Replace unbounded with `mpsc::channel(100)` and surface a tray badge when the queue is at capacity. (b) On startup, run `SELECT id FROM captures WHERE id NOT IN (SELECT capture_id FROM ocr_text)` and enqueue those — handles the "user quit mid-queue" case. (c) Expose queue depth as a debug field (`ocr_status` already exists in `snk-ocr/src/plugin.rs:14-17` but returns a hardcoded `"running"` — actually report the depth).

---

### 10. Clipboard watcher silently dies on a single `Clipboard::new()` failure

- **What:** `crates/snk-clipboard/src/watcher.rs:24-30`: if opening the clipboard fails at startup (e.g. another app holds the clipboard exclusively, X11 not ready, init race during fast login), the thread logs `error!` and `return`s. **The thread is never restarted.** For the rest of the session, clipboard history captures nothing; the popup window opens to an empty list. No retry, no exponential backoff, no health signal exposed to the rest of the app.
- **Where:** `crates/snk-clipboard/src/watcher.rs:22-50`.
- **Why it matters:** This is the most-used feature (per design). A transient init failure → user reports "clipboard history is empty" → there's no log file (finding #1) → support has no way to confirm whether the watcher is alive. The README headline-features clipboard.
- **Confidence:** High.
- **Suggested alternative:** Wrap the open + loop in a retry-forever with exponential backoff (cap at 60s), and surface a `clipboard-status` command/event so the popup can show "Clipboard watcher offline — see logs". Also handle the case where `clip.get_text()` / `clip.get_image()` returns a *permanent* error mid-loop (the current code falls through silently).

---

### 11. The release workflow has no `cargo audit` / `pnpm audit` / SBOM step — vulnerability disclosure is reactive

- **What:** `release.yml` builds, signs, notarizes, publishes. CI's `ci.yml` runs lint/test but no `cargo audit` or `pnpm audit`. The design's testing section (`§10.4`) explicitly promised: *"Nightly: full E2E + `cargo audit` + `npm audit`"*. There is no nightly job. There is no SBOM generation. The Tauri dependency tree pulls in `webview2`, `wry`, `tao`, `reqwest`, `rusqlite`, `xcap`, `arboard`, etc. — a large enough attack surface that a CVE in any of them needs a known path to a hotfix release.
- **Where:** `.github/workflows/ci.yml` (no audit jobs); design §10.4 line 683.
- **Why it matters:** When a CVE drops (e.g. `reqwest` TLS bug, `webkitgtk` RCE), the operator question is *"are we vulnerable, in which release, and how fast can we ship the fix?"*. Without audit gating, the team learns about CVEs from the Renovate inbox at best, from a public advisory at worst. Without an SBOM published per release, security-aware users (or corp IT) can't tell which release shipped which deps.
- **Confidence:** High.
- **Suggested alternative:** Add a nightly job that runs `cargo audit` + `pnpm audit --prod` and opens an issue on failure. Add `cargo-sbom` or `cargo cyclonedx` to the release workflow to attach an SBOM to each GitHub Release.

---

### 12. The release workflow's smoke-test of the sign tool leaves an Authenticode-signed copy of cmd.exe on disk and uploads nothing about it

- **What:** `release.yml:118-142` copies `C:\Windows\System32\cmd.exe`, signs it with Azure Artifact Signing as a "smoke test," then `Remove-Item`s it. This is harmless in normal flow, but: (a) if the run is cancelled between sign and remove, the runner workspace contains an un-revoked, Microsoft-cert-signed-by-snapper-keeper-csp binary that anyone with workflow logs can grep filenames for. (b) The comment ("we never distribute it") is true today but is one accidental glob away from being uploaded as an artifact (the upload step `release.yml:223-233` is path-explicit so currently safe). (c) More fundamentally: signing a third-party Microsoft binary with our cert is something that, if a corp policy team sees in the logs, looks weird. It's not malicious — it's a clever smoke test — but it's the kind of thing that gets flagged.
- **Where:** `.github/workflows/release.yml:118-142`.
- **Why it matters:** Low operational risk today, but the workflow runs on every tag and the smoke-test always runs *before* the actual build. If Azure Code Signing is down, the smoke fails fast (good intent). The cost is auditability — a security auditor reading the workflow sees "we sign random Windows binaries to test our cert."
- **Confidence:** Low (this is more taste than 3am-incident).
- **Suggested alternative:** Sign a known harmless test binary the team ships in the repo (e.g. a 1KB Rust-built executable) rather than `cmd.exe`. Or replace the smoke with `sign --help` against the Azure endpoint via a non-binary auth-only check if the sign CLI supports one.

---

### 13. No diagnostic / "about" command, no version surfacing in-app

- **What:** Settings window has no "About" panel or version display (`app/src/windows/settings/SettingsWindow.tsx` was greppable — no "version" or "About" string). The tray menu (`main.rs:143-157`) has no "About" item. The `tauri.conf.json:4` version is `0.0.1`. There is no command to show installed version vs. latest checked version vs. log path vs. data directory.
- **Where:** `app/src/windows/settings/SettingsWindow.tsx`; `app/src-tauri/src/main.rs:143-157`.
- **Why it matters:** When a user opens an issue, the very first thing support asks is "what version are you on?" — and a user without a CLI background has no way to find out. They can't tell `0.0.1` from `0.0.5`. The app has no self-diagnostic. They can't even tell *where* the data is stored (`%APPDATA%\com.snapper-keeper.app\`) without reading the README.
- **Confidence:** High.
- **Suggested alternative:** Add an "About" section to Settings showing: app version (read from `app.package_info().version`), data directory path with "Open" button, log directory path with "Open" button, updater status, last update check timestamp. This is ~20 lines and dramatically improves the support inbox.

---

### 14. macOS workflow does not produce a universal binary — design committed to one

- **What:** Design §11 line 700: *"Cross-architecture: macOS universal binary (aarch64 + x86_64)"*. Actual `release.yml:19-29` ships **two separate** macOS bundles (`Snapper.Keeper_aarch64.app.tar.gz` and `Snapper.Keeper_x86_64.app.tar.gz`) and `latest.json` ships `darwin-aarch64` + `darwin-x86_64` as separate platform keys. This is *fine* for the updater (it picks per arch) but means the GitHub release page shows two `.dmg` files and the user has to know which one to download. Tauri 2 supports `--target universal-apple-darwin` directly; the design explicitly chose it.
- **Where:** `.github/workflows/release.yml:21-26`; design §11 line 700.
- **Why it matters:** First-time-user friction. The download page question *"which one do I want?"* is exactly the kind of thing the design tried to avoid for the "share-friendly side project" audience. Intel-Mac users picking the aarch64 dmg get a `cannot be opened` error from Gatekeeper. The two-bundle approach also doubles macOS CI time and Apple notarization quota.
- **Confidence:** Medium-High (a deliberate divergence from the design might have a reason that isn't documented).
- **Suggested alternative:** Switch to `pnpm tauri build --target universal-apple-darwin` (Tauri 2 invokes lipo automatically). Halves macOS CI time. Single `.dmg` for the user. Updater manifest has one `darwin-universal` entry (the updater plugin understands this).

---

### 15. The `clipboard-popup` and other capture windows have a default capability that includes `core:default` — which grants e.g. `core:event:default` to all windows, but more importantly the popup likely can navigate the webview

- **What:** Following on finding #3, `core:default` includes a broad set of permissions — `core:webview:default`, `core:window:default`, etc. The clipboard popup with `core:default` can in principle call any unrestricted core command including potentially `core:webview:create_webview_window` or navigation calls (depending on Tauri 2 default permission set, which has shifted between minor versions). The capability granting is the system-of-record for what a misbehaving webview can do; the current "one capability for all windows" gives the popup the same blast radius as the library window.
- **Where:** `app/src-tauri/capabilities/default.json` (all permissions to all windows).
- **Why it matters:** Compounds finding #3. The popup is the smallest window with the most-frequent surface; design called for minimum permissions; current code gives it maximum.
- **Confidence:** Medium (depends on exact Tauri 2 default permission set, which is version-dependent).
- **Suggested alternative:** Same as #3 — split capability files per window with the minimum permission set each window actually needs. The library window probably needs ~80% of the current set; the popup needs ~10%.

---

## Summary

This is a well-designed system whose **operational instrumentation is missing**. The single most-critical gap is **no file-based logging and no panic hook** (finding #1) — every other operator concern downstream (debug, monitor, support, post-mortem) is gated on this. The second most-critical gap is the **migration story** (finding #5) — first version-bump release that touches schema can brick libraries with no recovery path. Third, the **release pipeline is brittle in the failure modes that matter most** (no kill switch, first-tag-must-be-stable trap, PRIVACY.md vs reality mismatch — findings #2/6/7).

The team has spent the last 15 commits chasing CI signing infrastructure (visible in `git log`), which is the right thing to do — but means the "what happens after we ship?" surface is undertested. I would not cut v1 against the GitHub Releases endpoint until at least findings #1, #2, #5, and #13 are fixed. The rest can ship and be patched in v1.0.1.

Specifically: as the on-call engineer for this fleet, I would not be able to debug a single user issue today, because no logs reach disk.
