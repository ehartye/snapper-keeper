# Adversary Perspective — Round 1

Lens: pre-mortem + red-team. The question is not "is this well-written?" — it's "where would I attack this, and what fails first under pressure?"

The bottom line up front: I would **not** ship this v1 today. There are two showstoppers (stored XSS into a CSP-less webview with full IPC; a clipboard watcher with no exclusions or `sensitive`-flag enforcement) plus three high-impact issues (an unbounded asset-protocol scope, two PRIVACY.md claims that are not implemented, and a `core:default` capability granted to fullscreen overlay windows). Several medium-severity issues stack on top.

---

## Findings

### F1 — Stored XSS via FTS snippet rendered through `dangerouslySetInnerHTML`, with CSP disabled

- **What:** The library search UI renders SQLite FTS5 `snippet()` output as raw HTML. The underlying text fed to `snippet()` includes OCR-extracted text (Tesseract output from arbitrary screen content), window titles (controlled by any process on the system), and clipboard text content (anything the user copied — including from web pages, terminals, hostile files). Combined with `csp: null` in `tauri.conf.json`, this is stored XSS with no mitigations.
- **Where:**
  - `app/src/windows/library/SearchBar.tsx:173` — `dangerouslySetInnerHTML={{ __html: result.snippet }}`
  - `crates/snk-library/src/search.rs:86` — `snippet(captures_fts, 3, '<mark>', '</mark>', '...', 32)` (col 3 = `ocr_text`)
  - `crates/snk-library/src/search.rs:104` — `snippet(clipboard_fts, 1, '<mark>', '</mark>', '...', 32)` (col 1 = `text_content`)
  - `app/src-tauri/tauri.conf.json:85` — `"csp": null`
  - No `escapeHtml`, `DOMPurify`, or sanitizer is imported anywhere in the frontend (grep across repo returns zero hits outside docs/plans).
- **Why it matters:** SQLite's `snippet()` does NOT HTML-encode the source text — it just inserts the prefix/suffix markers (`<mark>`/`</mark>`) around matching tokens and returns the rest verbatim. The end-to-end attack:
  1. Attacker creates a web page or file containing `<img src=x onerror="...">` or `<script>...</script>`.
  2. User screenshots the page (Tesseract OCRs the visible text — script tags or HTML in displayed source code do get OCR'd) OR copies the text to clipboard OR opens an app whose window title contains the payload.
  3. User later searches the library — the payload renders as live HTML inside the library window, which has the default capability set including `snk-library:default`, `snk-clipboard:default`, `snk-capture:default`, and `core:event:default`.
  4. The payload can now call any exposed Tauri command via `window.__TAURI__` (since `withGlobalTauri` isn't disabled and CSP is null): `hard_delete_capture`, `paste_item` (which **synthesizes a real keyboard Ctrl+V into the focused foreground window** — escape from the webview into the OS keyboard input stream), `set_setting` (poison settings), `purge_trash`, etc. It can also read every capture, every clipboard entry, and every OCR text via `list_captures`/`list_clipboard_items`/`get_capture`.
  5. `paste_item` is the worst escape: it writes attacker-chosen text into the OS clipboard and then synthesizes Ctrl+V into whatever app currently has focus (`crates/snk-clipboard/src/paste.rs:21-79`, Win32 `SendInput`). An XSS payload can stage an existing pinned malicious clipboard row and trigger a paste into the user's terminal, password manager, or browser address bar.
- **Confidence:** High. I traced every link in the chain. The only thing I didn't actually pop is a working PoC; the primitive is undeniable.
- **Suggested alternative:** Two layers, both required:
  1. Sanitize at the boundary: render snippets by splitting on the literal `<mark>`/`</mark>` markers in JS (or use a custom delimiter like `` that can't appear in HTML), then build the DOM with React elements — never `dangerouslySetInnerHTML`. The trivially-correct version is to drop the highlighting and render `result.snippet.replaceAll('<mark>','').replaceAll('</mark>','')` as a text node.
  2. Set a real CSP in `tauri.conf.json`: at minimum `default-src 'self'; img-src 'self' asset: data:; script-src 'self'; style-src 'self' 'unsafe-inline'`. Even if (1) regresses, CSP becomes defense in depth.

---

### F2 — Clipboard watcher captures everything, with no exclusion list and no sensitive-flag enforcement

- **What:** The clipboard polling thread snapshots every text and image clipboard change every 500ms and writes it to SQLite. There is no filter for password managers, no honoring of the Windows `ExcludeClipboardContentFromMonitors` / `CanIncludeInClipboardHistory` formats, no detection of OS "Concealed" clipboard data, no app-blocklist enforcement, and the `sensitive` column on `clipboard_items` is never set by any code path — it's a schema field that exists but is dead.
- **Where:**
  - `crates/snk-clipboard/src/watcher.rs:54-90` (`poll_text`) — accepts every non-empty text unconditionally.
  - `crates/snk-clipboard/src/watcher.rs:92-136` (`poll_image`) — same for images.
  - `crates/snk-library/migrations/V002__clipboard_items.sql:11` — `sensitive INTEGER NOT NULL DEFAULT 0` (declared, never written).
  - `crates/snk-library/src/clipboard.rs:131-143` — `ClipboardItem` always returns `sensitive: false` after insert; no setter exists.
  - `crates/snk-library/src/settings.rs:86` — `clipboard.app_blocklist` appears only in a test fixture; grep shows zero readers in production code.
  - `crates/snk-clipboard/src/watcher.rs:77-78` — `source_app` and `source_window_title` on the inserted item are hardcoded `None`, so even an opportunistic post-hoc filter can't work.
- **Why it matters:** A user who copies a password from 1Password / KeePass / Bitwarden / a browser password autofill, or copies a 2FA TOTP code, has it captured to plaintext SQLite within 500ms — and it persists across restarts. The retention cap is 200 unpinned items (`MAX_UNPINNED = 200` in `watcher.rs:14`), so a single power-user day's worth of credentials sits at rest until evicted. The DB file is unencrypted (`Db::open` calls `Connection::open` with no encryption pragma; no SQLCipher dep in Cargo.toml). A same-machine attacker, a malicious local process running under the user's UID, a stolen laptop, or a cloud-synced AppData (OneDrive, iCloud, Dropbox in `%APPDATA%`) all walk away with the cleartext clipboard archive. This is the single most common "screen-capture utility leaks creds" story.
- **Confidence:** High.
- **Suggested alternative:** Before the first public release, at minimum:
  1. Honor the Windows clipboard exclusion formats: skip text when `CanIncludeInClipboardHistory` is set to 0, or when `ExcludeClipboardContentFromMonitors` is present. (Win32: `RegisterClipboardFormatW(L"CanIncludeInClipboardHistory")` then `IsClipboardFormatAvailable`.)
  2. On macOS, check the `org.nspasteboard.ConcealedType` and `org.nspasteboard.AutoGeneratedType` types and skip them.
  3. Detect TOTP/credit-card/password-shaped text via a heuristic regex pass and either skip or auto-flag `sensitive=1`.
  4. Implement the documented `clipboard.app_blocklist` and ship a default that includes 1Password, KeePass, KeePassXC, Bitwarden, Dashlane, LastPass, Apple Passwords, Keeper, Proton Pass. Read the actual `source_app` (not `None`) so the blocklist can fire.
  5. Document explicitly in PRIVACY.md that the clipboard history is captured to disk in plaintext until any of the above lands.

---

### F3 — Asset-protocol scope is wide open inside `$APPDATA`/`$APPLOCALDATA`

- **What:** `tauri.conf.json` allows the webview to load any file matching `$APPDATA/**` or `$APPLOCALDATA/**` via the `asset:` protocol. Because the app identifier is `com.snapper-keeper.app`, this expands at runtime to a Tauri-resolved app-data root that, on Tauri 2, points at the application's own data dir — but the `**` glob makes every file under that root readable via `convertFileSrc(...)` from the webview.
- **Where:** `app/src-tauri/tauri.conf.json:86-91`
- **Why it matters:** Combined with F1, an XSS payload can `fetch('asset://...')` against arbitrary paths inside the app's data dir — including `snapper-keeper.db` (the entire SQLite file with all clipboard text + OCR text + capture metadata, even though it's WAL'd) and `*.db-wal` / `*.db-shm`. Even absent XSS, this is a wider surface than needed: thumbnails only need `captures/**` and `clipboard/**`. The current allowlist also pulls the WAL file, the SQLite main file, the preview cache `.preview.png`, the migrations log, and anything else that lands there. On Windows the app dir is `%APPDATA%\com.snapper-keeper.app\` which is not inside the user's `%LOCALAPPDATA%` low-IL boundary, but is still readable by any other process running as the user — that's not the issue; the issue is the webview's own surface.
- **Confidence:** High on the scope; Medium on exploitability (depends on F1 landing first).
- **Suggested alternative:** Tighten to the minimum required: `["$APPDATA/captures/**", "$APPDATA/clipboard/**"]`. The webview never needs to read the SQLite file or the preview-cache directly via asset://; data flows through Tauri commands instead.

---

### F4 — `csp: null` means no CSP at all, even for the legit frontend

- **What:** `tauri.conf.json:85` sets `"csp": null`. In Tauri 2, this means no Content-Security-Policy header is injected, so the webview falls back to whatever WebView2/WKWebView's permissive defaults are. Inline scripts work, inline event handlers work, eval works, remote script loading works, and there's no CSP report mechanism.
- **Where:** `app/src-tauri/tauri.conf.json:85`
- **Why it matters:** This is what turns F1 from "annoying" to "full remote code execution inside the webview." Even without F1, any future regression where a developer accidentally injects user-controllable strings into the DOM has zero defense. It also means a compromised dev dependency (a malicious npm package, e.g., during a future update) can phone home from inside the webview with no constraint.
- **Confidence:** High.
- **Suggested alternative:** Set a real CSP. Tauri's documented minimum for a SPA: `"default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' ipc: http://ipc.localhost https://github.com"`. Validate by booting and checking the network panel.

---

### F5 — PRIVACY.md makes two material claims that the code does not implement

- **What:**
  - "You can disable update checks in Settings." — There is no setting that the updater reads. `crates/snk-updater/src/plugin.rs:142-163` unconditionally spawns the 5-second-after-startup check and the 24-hour interval loop. Grep for `disable.*update` / `update.*enabled` / `check.*for.*update.*setting` returns zero hits outside of PRIVACY.md and the phase-7 plan.
  - "The Microsoft Store edition makes zero network requests. The in-app updater is compiled out." — There is no `#[cfg(feature = "...")]`, no Cargo feature, no build profile, and no conditional compilation gate that excludes the updater from the binary based on distribution channel. The Microsoft Store edition does not exist as a distinct build.
- **Where:**
  - `PRIVACY.md:25` ("You can disable update checks in Settings")
  - `PRIVACY.md:27-29` ("The Microsoft Store edition makes zero network requests")
  - `crates/snk-updater/src/plugin.rs:142-163` (unconditional startup + interval)
  - `app/src-tauri/Cargo.toml:13-23` (no feature flags on plugins)
  - `Cargo.toml` workspace (no feature flags)
- **Why it matters:** These are not minor docs drift — these are privacy-policy commitments. Shipping them as written when the code does not honor them is a regulatory / consumer-protection risk (FTC Section 5 deceptive-trade-practices framing; in the EU, potentially Article 5 GDPR transparency). An adversary writing a critical security writeup will lead with this because it's the most quotable issue: the privacy policy describes mitigations that do not exist.
- **Confidence:** High.
- **Suggested alternative:** Either (a) implement both before v1.0 (add a `updater.check_enabled` setting that the plugin reads on each tick, add a `--no-default-features` profile that the Store build uses, and verify the binary by `strings | grep -i github.com`), or (b) edit PRIVACY.md now to remove both claims and re-add only after they're actually shipped.

---

### F6 — Every window has full IPC, including the fullscreen overlay and the popup

- **What:** The single `default` capability in `app/src-tauri/capabilities/default.json` is granted to all six windows: `library`, `capture-overlay`, `capture-toolbar`, `annotate`, `clipboard-popup`, `settings`. That includes `snk-library:default` (delete captures, list clipboard items, set settings), `snk-clipboard:default` (paste-and-keystroke-injection), `snk-capture:default` (capture-anywhere), `snk-updater:default` (force update checks), and `autostart:default` (toggle autostart).
- **Where:** `app/src-tauri/capabilities/default.json:5-28`
- **Why it matters:** Two problems. (1) Architecturally, the `capture-overlay` and `clipboard-popup` windows have no legitimate need for `hard_delete_capture` or `set_setting` — least privilege is violated. (2) Operationally, if F1 ever fires in the smaller windows (the popup renders clipboard entries — clipboard text content is attacker-controlled), the blast radius is identical to the library window. The popup loads from `clipboard-popup/...` and the in-flight clipboard rows are rendered via React — confirm whether the popup's rendering path also uses `dangerouslySetInnerHTML` or string-templated HTML; even if not today, the current capability set means any future bug there is RCE-equivalent.
- **Confidence:** High on the over-broad grant; Medium on currently-exploitable XSS in the popup (didn't fully audit ClipboardPopupItem.tsx).
- **Suggested alternative:** Split capabilities per window. Sketch:
  - `library.json` — full library + clipboard read + capture write
  - `capture-overlay.json` — only capture commands + window control
  - `clipboard-popup.json` — only `list_clipboard_items`, `get_clipboard_item`, `paste_item`, `show_popup`
  - `annotate.json` — only `save_annotation`, `derive_capture`, `get_capture`
  - `settings.json` — only settings + updater + autostart

---

### F7 — Auto-updater has no rollback protection or version-pinning

- **What:** The updater plugin trusts whatever `latest.json` says (signed minisign blob protects the payload integrity, but the *decision to install* is made purely on "is the remote version > current"). There is no minimum-version floor stored locally, no list of revoked versions, no signature check on `latest.json` itself, and no user-visible "what version am I about to install" confirmation before `download_and_install` fires.
- **Where:**
  - `crates/snk-updater/src/plugin.rs:63-114` — on `Some(update)`, the plugin immediately spawns `download_and_install` with no user prompt.
  - `app/src-tauri/tauri.conf.json:110-112` — single endpoint `https://github.com/ehartye/snapper-keeper/releases/latest/download/latest.json`.
  - `.github/workflows/release.yml:259-306` — `latest.json` is generated and uploaded but never signed. The minisign signature only covers the bundle artifacts; the manifest itself is unsigned.
- **Why it matters:** Two related attacks.
  1. **GitHub account takeover or release-asset tampering:** if an attacker gets write access to the repo (compromised maintainer credential, stolen GITHUB_TOKEN in CI logs, malicious PR that modifies the release workflow), they can publish a new `latest.json` pointing at a malicious bundle. The minisign private key is in GitHub Actions secrets (`TAURI_SIGNING_PRIVATE_KEY`) — the same blast radius. There is no second factor (HSM signing, manual approval gate, two-person rule) on releases.
  2. **Downgrade attack via cherry-picked manifest:** even with intact signing, if `latest.json` can be served by a MITM (TLS pinning is not configured), an attacker could serve an old vulnerable signed version. Minisign verifies the bundle was signed by your key — it does NOT verify it's *the current version*. Tauri's updater compares against the running binary's version; if the user is on 1.5.0 and the attacker serves 1.4.0 with a valid signature, current code shows "no update" — but if the user just installed fresh 1.0.0, a MITM serving 1.4.0 (with a known CVE) signed cleanly will install silently.
  3. The 5-second-after-startup auto-trigger (`plugin.rs:147`) bypasses any "let me check first" instinct from the user; they don't even know it's happening.
- **Confidence:** High on the architectural gaps; Medium on field exploitability (MITM against GitHub requires getting past Let's Encrypt + GitHub's HSTS — non-trivial but not impossible against a compromised CA).
- **Suggested alternative:**
  1. Store the highest-ever-seen version in settings; refuse to install anything strictly less.
  2. Sign `latest.json` itself with the minisign key and verify the signature before trusting the contents. Tauri's updater can be told to load a `.sig` next to the manifest.
  3. Add a user confirmation dialog before `download_and_install` fires, with the version number and changelog link. Auto-install is hostile for a "share-friendly side project."
  4. Move the minisign signing key to a hardware-backed signer (Sigstore Cosign keyless via OIDC, or move signing into Azure Trusted Signing alongside Authenticode). Eliminate the long-lived secret.

---

### F8 — Updater verifies `download_and_install` errors only via `e.to_string()`; signature failures are reported to the UI as plain strings

- **What:** Both `update.check()` and `download_and_install` errors are stringified via `e.to_string()` and emitted to the frontend as `UpdateStatus::Error { detail }`. The frontend then renders that string. There is no distinction in the type system between "network failed" (benign, retry tomorrow) and "signature verification failed" (red alarm, the served bundle is forged or downgrade attempt).
- **Where:** `crates/snk-updater/src/plugin.rs:104-113`, `plugin.rs:124-133`
- **Why it matters:** Operationally, a user who sees "update download failed: signature mismatch" will not understand it's a security event. There is no logging to a separate sink, no notification, no telemetry (per privacy commitment), and no abort that prevents the next 24h tick from retrying — meaning if an attacker is briefly able to MITM, the app silently retries the next day with the same forged manifest, hoping the user hits a different network condition that lets it land. Worse, the only sign anything's wrong is a transient UI status.
- **Confidence:** Medium-High.
- **Suggested alternative:** Match on the error variant returned by `tauri_plugin_updater` (it distinguishes I/O vs. signature errors). Treat signature errors as terminal — disable the updater for the rest of the process lifetime, log to a dedicated security-event file, surface a non-dismissable banner. Don't auto-retry signature failures.

---

### F9 — Autostart capability granted to all windows; toggle is one IPC call from any window

- **What:** `autostart:default` is in the catch-all capability, so any window (including the fullscreen capture overlay and the popup) can invoke `autostart:enable` / `autostart:disable` via `window.__TAURI__.invoke`. There's no UX gate or user-prompted confirmation in the Rust plugin — the autostart plugin just writes the Windows registry / macOS LaunchAgent on demand.
- **Where:** `app/src-tauri/capabilities/default.json:27`
- **Why it matters:** Combined with F1, an XSS payload can flip the app to launch at login (or off), achieving persistence on a fresh boot. On Windows this writes to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` with a path under the user's install dir; not elevation, but persistence. The "no autostart by default" privacy posture is undermined by remote control of the toggle.
- **Confidence:** Medium (depends on XSS landing).
- **Suggested alternative:** Move autostart to a Settings-only capability and require the toggle to round-trip through a Tauri command that double-checks the call came from the `settings` window label.

---

### F10 — `dotnet sign` install uses `--prerelease`; CI actions pinned by mutable tag

- **What:**
  - `.github/workflows/release.yml:90` — `dotnet tool install --global --prerelease sign`. The `--prerelease` flag means a malicious or buggy prerelease of `Sign` can land in CI without notice.
  - Every action is pinned by tag (`actions/checkout@v4`, `pnpm/action-setup@v3`, `dtolnay/rust-toolchain@stable`, `softprops/action-gh-release@v2`, etc.). Tags are mutable; a compromised maintainer or upstream account takeover can force-push a tag to a malicious commit, and CI silently picks it up on the next run.
  - `release.yml:308-310` uses `softprops/action-gh-release@v2`, a third-party action that runs with `permissions: contents: write` — the keys to the kingdom. If that action is compromised, the attacker can replace the released bundle with their own (and the signing was already done — they just swap files on the release after upload).
- **Where:** `.github/workflows/release.yml` (multiple lines as noted), `.github/workflows/ci.yml` (similar pattern)
- **Why it matters:** Supply-chain compromise of any of these grants the attacker arbitrary code execution inside the signing job, which has access to `TAURI_SIGNING_PRIVATE_KEY`, `AZURE_CLIENT_SECRET`, `APPLE_CERTIFICATE_PASSWORD`, etc. They can sign and ship a malicious update on your behalf, signed with your real cert. End-game.
- **Confidence:** High on the gap; Medium on near-term exploit likelihood (these specific actions have decent reputations, but the systemic risk is real).
- **Suggested alternative:**
  1. Drop `--prerelease` from `dotnet tool install sign`; pin a specific stable version: `dotnet tool install --global sign --version 0.9.X`.
  2. Pin all GitHub Actions by full commit SHA (`actions/checkout@b4ffde65f...`). Use Renovate / Dependabot to update them and review each bump.
  3. Replace `softprops/action-gh-release@v2` with `gh release create` from the GitHub CLI inside an inline script — fewer hops, no third party.
  4. Split the release workflow: the build/sign job uploads artifacts; a separate job with `contents: write` publishes them, after a manual approval gate (`environment: production-release`).

---

### F11 — Choco-installed Tesseract is bundled verbatim into the installer without integrity verification

- **What:** `.github/workflows/release.yml:66-76` runs `choco install tesseract -y --no-progress` on the Windows runner and then copies `C:\Program Files\Tesseract-OCR\*` verbatim into `app/src-tauri/resources/tesseract/`. There is no hash check, no version pin, no signature verification on the chocolatey package. The whole tesseract distribution (executable + DLLs) then ships inside your signed installer.
- **Where:** `.github/workflows/release.yml:66-76`
- **Why it matters:** A compromised chocolatey upstream (or one of its mirrors) injects a malicious tesseract.exe / leptonica DLL — which is then bundled into your signed Authenticode-signed installer, with your reputation on it. You'd be unwittingly distributing malware under your code signature. This is exactly the Solarwinds-shape risk: trusted upstream + automated rebuild = supply-chain blast radius.
- **Confidence:** Medium (chocolatey itself is reasonably trusted; specific tesseract package mirror is the weak link). Impact: High.
- **Suggested alternative:** (a) Pin the chocolatey package to a specific version (`choco install tesseract --version=5.3.4.20240503 -y`); (b) Verify SHA256 of `tesseract.exe` against a known-good hash committed to the repo; (c) Long-term, build tesseract from source in CI from a pinned commit, or vendor the bundle in-repo with a hash-pinned download step.

---

### F12 — No process isolation for the Tesseract sidecar; runs with full app privileges

- **What:** Tesseract is spawned as a child process with the app's full token (no job-object restriction, no integrity-level downgrade on Windows, no sandbox-exec on macOS, no seccomp). The image path is passed as the first argv. The bundled binary's path is resolved by `OnceLock`-cached lookup.
- **Where:** `crates/snk-ocr/src/sidecar.rs:151-199`
- **Why it matters:** Tesseract historically has had image-parsing bugs in leptonica (CVEs over the years for buffer overflows in PNG/TIFF handling). A malicious PNG screenshot — and these can come from a screenshot of *anything*, including a webpage that serves crafted images — could trigger one of those bugs and get arbitrary code execution. With no sandboxing, that's RCE inside the app's process token. Not catastrophic on its own (low-likelihood vs. specific tesseract CVE), but the design choice is "fire-and-forget OCR on user input" — exactly the kind of code path that benefits from a child sandbox.
- **Confidence:** Medium (depends on an unpatched tesseract CVE).
- **Suggested alternative:** (a) On Windows, drop integrity level to Low when spawning the child via `CreateProcessAsUser` + restricted token (or simpler: use AppContainer + a job object with `JOB_OBJECT_LIMIT_ACTIVE_PROCESS=1`). (b) On macOS, wrap with `sandbox-exec -p '(version 1)(deny default)(allow file-read* (subpath "..."))'`. (c) At minimum, set a timeout — current code has retry delays but no per-invocation timeout — a hung tesseract holds the queue.

---

### F13 — Image clipboard captures and screenshots can include sensitive content with no per-app exclusion

- **What:** A capture-anything-by-hotkey UX combined with the clipboard watcher's image branch means anyone with shell access to the user's machine can shoulder-surf their entire on-screen / in-clipboard history. The Windows clipboard inbound polls every 500ms; the screenshot history is unlimited (no eviction policy in `captures.rs`, unlike clipboard's 200-cap).
- **Where:** `crates/snk-capture/src/orchestrate.rs:81-105` (no exclusion); `crates/snk-clipboard/src/watcher.rs:92-136`; no policy to cap captures table size.
- **Why it matters:** Unbounded growth of an unencrypted SQLite DB containing OCR'd text from every screen the user ever captured (their email, banking, medical portal, IDE with secrets, password reset emails, etc.). After a year of use this is a hostile-actor goldmine. The threat model "no servers, no telemetry" is correct but stops short of "and the local DB is itself a sensitive store" — there's no mention of encrypted DB option, no documented retention default, no purge-on-uninstall.
- **Confidence:** High (this is design intent — by-design data accumulation).
- **Suggested alternative:** (a) Add an SQLCipher option behind a setting (with a passphrase derived from OS keychain via `keyring` crate). At least offer it. (b) Document a default retention setting (e.g., auto-delete OCR text >180 days). (c) Make the threat model in PRIVACY.md explicit: "this app stores your captures and clipboard locally, in plaintext, with the same protection as any file in your user directory."

---

### F14 — `_app` parameter unused in `LibraryState` commands — windows can't be distinguished server-side

- **What:** Most `snk-library` and `snk-clipboard` commands accept `_app: tauri::AppHandle<R>` but don't read it. This means there's no per-window authorization check in the command handlers themselves — the only access control is the capability ACL, which (per F6) is uniform across all windows.
- **Where:** `crates/snk-library/src/commands.rs:11-186` (every command); `crates/snk-clipboard/src/commands.rs:21-53`
- **Why it matters:** Even if you fix F6 (split capabilities per window), defense in depth would suggest the destructive commands (`hard_delete_capture`, `purge_trash`, `set_setting`) double-check the caller's window label inside the handler. Today they don't, so a misconfiguration in capabilities silently fails open.
- **Confidence:** Medium (process improvement rather than active vulnerability).
- **Suggested alternative:** In the most destructive handlers, use `app.get_focused_window()` or a window-label parameter from the IPC envelope to assert the caller is `library` or `settings`. Reject otherwise. This is belt-and-suspenders to the capability fix.

---

### F15 — Annotation state stored as opaque JSON, never validated; image data from frontend trusted as PNG

- **What:** `save_annotation` and `derive_capture` accept `png_data: Vec<u8>` from the frontend and write it directly to disk via `write_atomic` with no validation that the bytes are actually a PNG. `state_json` is stored verbatim in the DB.
- **Where:** `crates/snk-annotate/src/commands.rs:8-37` (`save_annotation`); `crates/snk-annotate/src/commands.rs:39-79` (`derive_capture`)
- **Why it matters:** Limited blast radius on its own — these come from the same webview, so an XSS payload could store a fake "PNG" that's actually a Windows shortcut, executable, or HTML file, with a `.png` extension. Then `convertFileSrc` serves it back via `asset:` — but Tauri's asset protocol sniffs MIME, so it's image-only on read. The real risk is filesystem pollution: an attacker can use repeated `save_annotation` calls to fill the disk (no quota enforcement) or to drop content under the asset-protocol scope. Combined with F1, this is "stash payload on disk for later retrieval via `asset://`."
- **Confidence:** Low-Medium.
- **Suggested alternative:** Validate PNG magic bytes (`89 50 4E 47 0D 0A 1A 0A`) before writing; reject otherwise. Parse `state_json` with a serde struct and reject anything that fails schema validation. Set a max size on the payload (e.g., 50 MB).

---

### F16 — Tracing env-var override `SNK_LOG` could be used to log secrets to disk

- **What:** `main.rs:67-71` reads `SNK_LOG` env-var to configure tracing. If a user sets `SNK_LOG=trace`, the clipboard text content (logged in some paths) and OCR text could appear in stdout/stderr. If they redirect that to a file or to a shared log aggregator, it's a leak vector.
- **Where:** `app/src-tauri/src/main.rs:67-71`; tracing logs are sprinkled throughout (e.g., `crates/snk-updater/src/plugin.rs:66` logs version)
- **Why it matters:** Low-severity, but worth noting: documentation should explicitly say not to enable verbose logging on a shared machine. Some tracing calls (`tracing::info!(chars = output.text.len(), "ocr indexed")`) are fine; others elsewhere might leak content depending on future code drift.
- **Confidence:** Low.
- **Suggested alternative:** Define a `#[derive(Debug)]` policy — wrap any field that could be sensitive (clipboard text, OCR text) in a `Redacted<T>` newtype whose Debug impl prints `<redacted>`. Audit existing log call-sites.

---

## Summary

Two showstoppers (F1, F2) and one near-showstopper (F5 — false privacy claims) would each, on their own, justify holding the public release. Together they form a plausible single-incident chain: a screenshot of a hostile page → OCR'd payload in the FTS index → search-triggered XSS → exfil of the entire unencrypted clipboard log (which includes the user's passwords because there's no exclusion list) → either silent paste-injection into the foreground app or autostart persistence. This is not theoretical — every link is a real primitive in the current source.

The signing/notarization/updater pipeline (Phase 7) is competently built but has supply-chain (F10, F11) and rollback (F7, F8) gaps that should be addressed before the v1.0 tag goes out. The capability model (F6, F9, F14) is the easiest near-term win — refactoring `default.json` into per-window capabilities is a few hours' work and dramatically shrinks blast radius.

Recommendation: hold v1.0. Land F1+F4+F2+F5 fixes minimum, then ship as v0.x with a security note. Treat F6/F7/F9 as "v1.0 must-haves" and F3/F8/F10–F16 as "before-1.0 nice-to-haves."
