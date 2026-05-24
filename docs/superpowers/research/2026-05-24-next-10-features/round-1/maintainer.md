# Round 1 — Maintainer Perspective

**Lens.** Six months from now, I'm the one fixing this on a part-time Saturday. Every new feature is a permanent tax: cross-platform parity, signing/notarization implications, new dependencies whose CVEs I have to track, OS APIs that drift between major releases, and a test surface I have to keep passing on 3 CI runners (mac arm64, mac x64, win x64 — no Linux today).

**Calibration anchors from the repo:**
- Phase 1 (foundation + vertical slice across 3 plugins): ~2750 plan lines, ~30 tasks, multi-week.
- Phase 5 (OCR + FTS5 + tesseract sidecar bundling): ~1720 plan lines, ~3 weeks effective.
- Phase 7 (signing + notarization + updater): ~1220 plan lines, but multiple follow-up PRs were needed (commits 15–19 on `main` are all release-signing fixes). The "happy path" plan understated real CI friction by ~2x.
- The whole codebase is ~5.2k lines of Rust today across 7 plugins. The largest single file (`snk-library/captures.rs`) is 991 lines — already over the 500-line red-flag threshold once.
- We have **no Linux** in the matrix. Adding Linux is a real platform, not a footnote.

I judge complexity at **5 = ~1 week / 1 crate / 0 new platform code / 0 new sidecars** and **1 = multi-month / new platform / new sidecar binary or backend / signing implications.**

---

## Position

### 1. EV code-signing cert (Windows SmartScreen)
   Fit: 5/5  Reach: 4/5  Complexity (higher=cheaper): 5/5  Value: 5/5
   Reasoning: For an audience-B "share-friendly" app, the single biggest install-friction killer on Windows is SmartScreen's "Microsoft Defender SmartScreen prevented an unrecognized app from starting" dialog. A standard cert (we have one) is *not* immediately trusted — reputation builds slowly with installs. An EV cert is trusted on day one. Almost no engineering: the Azure Trusted Signing `signCommand` path already wired in phase 7 stays the same; we swap the cert profile. The lift is procurement + identity verification + an HSM-or-Azure-managed key, plus a one-line tauri.conf.json update. This is the highest-leverage *non-code* enhancement available.
   Implementation cost estimate: 0 crates, 0 new deps, 0 OS-specific code, ~0.5 dev-week (most of which is calendar wait on cert issuance). Annual: $300+.

### 2. macOS Vision OCR (replace Tesseract on Mac)
   Fit: 5/5  Reach: 3/5  Complexity (higher=cheaper): 3/5  Value: 4/5
   Reasoning: Vision is dramatically more accurate than Tesseract on screenshots (UI fonts, antialiasing) and ships in the OS — we delete the Tesseract sidecar from the macOS bundle, removing the GPL-via-Apache-2.0-licensed-tesseract footprint, ~50 MB of payload, and a class of CI bundling bugs we've already hit. The cost is a trampoline crate: a thin Objective-C/Swift shim or `objc2`-based Rust wrapper around `VNRecognizeTextRequest`. `snk-ocr` gains a `cfg(target_os = "macos")` path that bypasses the sidecar entirely. No new schema, no UI work. Risk: `objc2` ecosystem churn is real but mature; alternatively a tiny Swift sidecar compiled in CI is dead simple.
   Implementation cost estimate: 1 crate (modify `snk-ocr`), 1 new dep (`objc2-vision` or a Swift sidecar), macOS-only code, ~1.5–2 weeks. Reduces Mac installer size + removes a bundle-fragility class permanently.

### 3. Scrolling capture (Windows + macOS)
   Fit: 5/5  Reach: 5/5  Complexity (higher=cheaper): 2/5  Value: 4/5
   Reasoning: This is the #1 feature competitors (Snagit, ShareX, CleanShot) ship that we don't. It's the most-requested deferred item. Implementation is genuinely hard but bounded: programmatic scroll synthesis (`SendInput` WHEEL events on Windows, `CGEventCreateScrollWheelEvent` on macOS) + repeated `xcap` capture + stitching with image-correlation overlap detection (the standard ~10–15% bottom-strip cross-correlation algorithm — there are MIT-licensed Rust crates for this, e.g. `imageproc`). The accuracy ceiling is "good enough on browsers and docs, fragile on canvas/virtualized lists" — that's the universal honest answer; nobody nails it. Single new crate (`snk-scroll-capture`), no new sidecar, no new schema (final stitched PNG is just another capture). Maintenance risk is real because scroll APIs are quirky, but the *surface* is contained.
   Implementation cost estimate: 1 new crate, +1–2 deps (`imageproc`, possibly `windows`/`objc2` crates we may already pull transitively), per-OS code, ~3–4 weeks. New 500-700 line file probable — keep modular.

### 4. Synced clipboard between devices (LAN-only, no cloud)
   Fit: 4/5  Reach: 4/5  Complexity (higher=cheaper): 3/5  Value: 4/5
   Reasoning: The audience-B "no servers" rule rules out cloud. But **LAN-only, mDNS-discovered, end-to-end-encrypted device-to-device clipboard sync** between two snapper-keeper instances on the same network preserves the posture: no accounts, no backend, no telemetry, no central servers. This is genuinely differentiated (1Password and Apple do *device-to-device* sync but no general clipboard manager I know of offers it without a cloud). Stack: `mdns-sd` (mature, MIT) + `iroh` or simpler raw `quinn`/QUIC + Ed25519 device-pairing-by-QR-code. Crucially: skip the "what if the device is on cellular" problem entirely. Single new plugin (`snk-sync`), opt-in, default off — zero impact on users who don't enable it. Maintenance risk: network code is forever a bug magnet, but a LAN-only protocol stays small.
   Implementation cost estimate: 1 new crate, 3–4 new deps (`mdns-sd`, `quinn` or `iroh`, `ed25519-dalek`, `qrcode`), no OS-specific code, ~3 weeks. Default off makes regression risk asymmetric.

### 5. Video / GIF recording (short clips, MP4 + animated WebP/GIF export)
   Fit: 4/5  Reach: 5/5  Complexity (higher=cheaper): 1/5  Value: 3/5
   Reasoning: Massive market reach — "record a 10-second screen clip" is the second-most-requested feature in this category after scrolling capture. Cost is the highest in the list and I want to be brutally honest. We'd need an encoder; the only realistic options are: (a) a bundled FFmpeg sidecar (~30 MB compressed, GPL/LGPL licensing review required for redistribution, and the binary needs separate Windows code-signing for SmartScreen reputation), or (b) `ac-ffmpeg`/`openh264` Rust bindings, or (c) hardware encoders via `MediaFoundation` (Win) / `VideoToolbox` (Mac) which means deep platform code. xcap doesn't do video; we'd need `scap` (mac) and `windows-capture` (win) — both real crates but the *unified* abstraction layer is ours to build. Then UI for record/pause/stop, trim, region selection (similar to capture overlay but persistent), audio gating, etc. Probably 6-10 weeks honest estimate. The reason it still ranks 5th is the user value is genuinely huge; I'd just want to scope it down to "no audio, MP4 only, 30 fps cap, region or window only" before approving.
   Implementation cost estimate: 1 new crate + significant `snk-capture` rework, 4-6 new deps, per-OS encoder code, sidecar binary likely, ~6–10 weeks. Bundle size grows ~30 MB.

### 6. Linux support (Wayland + X11)
   Fit: 3/5  Reach: 3/5  Complexity (higher=cheaper): 2/5  Value: 3/5
   Reasoning: Tauri 2 supports Linux. xcap supports Linux. Tesseract works fine on Linux. arboard works on Linux. The plugin contracts compile. **What doesn't just work**: Wayland screen capture requires the `xdg-desktop-portal` flow (user picks the window/screen via a system picker — fundamentally different UX from our region overlay), Wayland global hotkeys require the Wayland-protocol portal (limited support, varies by compositor), clipboard on Wayland has wl-clipboard quirks, and auto-paste via synthetic input is *blocked by design* on Wayland (security boundary). On X11 most things work, but X11 is end-of-life on major distros. Packaging means .deb, .rpm, and AppImage (Tauri can do this) plus a third CI runner. The fundamental problem isn't "compile for Linux" — it's that **Wayland materially changes the UX of two of our core flows**. I'd defer until there's user signal; meanwhile add a third CI build job to keep the code compiling.
   Implementation cost estimate: ~1–2 weeks for "compiles on Linux" plus 3–5 weeks for "Wayland UX is acceptable", per-OS code in 3 plugins, +1 CI runner. Permanent maintenance tax for what's likely <10% of audience-B users.

### 7. Crash report uploads (self-hosted Sentry or simpler GH issue funnel)
   Fit: 3/5  Reach: 2/5  Complexity (higher=cheaper): 4/5  Value: 3/5
   Reasoning: Currently we have **zero visibility** into crashes in the field. For a side-project maintainer this is the most expensive missing thing — every silent crash becomes a future GitHub issue with no reproduction. Self-hosting Sentry to preserve the "no servers" posture is genuinely ironic; the lower-friction path is the `panic_hook` writes a JSON crash dump locally (already specified in design §9.1) + a **one-click "submit this crash report" button** in the library that opens a pre-filled GitHub issue via the user's browser (no network code in the app at all, no upload, full user consent). Engineering cost is tiny; the user-trust posture stays clean. I'd downrank "self-hosted Sentry" hard but uprank "manual GH-issue funnel" as the realistic and shippable form.
   Implementation cost estimate: 0 new crates (extend `snk-library`/app shell), 0 new deps, no OS-specific code, ~3–4 days. Major maintenance ROI per hour spent.

### 8. CLI / URL-scheme automation hooks (`snapper-keeper://capture/region`)
   Fit: 4/5  Reach: 3/5  Complexity (higher=cheaper): 4/5  Value: 3/5
   Reasoning: A power-user enhancement that's deeply on-brand for an audience-B side project. Register the app as a custom URL scheme handler on both OSes (Tauri 2 has first-class support: `tauri-plugin-deep-link`) and expose a small command surface: trigger captures, open the library to a tag, paste a clipboard item by id. Lets users wire snapper-keeper into Raycast, Alfred, Stream Deck, AutoHotkey, Hammerspoon, Shortcuts.app — instantly multiplying perceived value via integrations *we don't build*. Single plugin (`snk-automation`) wraps the deep-link plugin and routes URLs to existing commands. No new schema, no new windows. Risk: URL injection — must whitelist commands and validate args strictly.
   Implementation cost estimate: 1 small new crate, 1 new dep (`tauri-plugin-deep-link`), trivial OS-specific registration, ~1 week.

### 9. iCloud / OneDrive backup-only mode (point library at a sync folder, captures-only)
   Fit: 3/5  Reach: 3/5  Complexity (higher=cheaper): 4/5  Value: 3/5
   Reasoning: Full cloud sync is rightly deferred (SQLite + cloud-folder sync = corruption). But a **backup-only** mode where the *capture PNG files* (not the DB) get mirrored to a user-selected folder under iCloud Drive / OneDrive / Dropbox is harmless: it's one-way file copy, no SQLite involvement. Settings adds a "Mirror captures to folder…" toggle; the library's GC sweep gets a paired "mirror in lockstep" pass. Doesn't solve "sync clipboard across devices" — but it satisfies the most common request behind "cloud sync" which is really "don't lose my screenshots if my laptop dies." Honest about what it is: backup, not multi-device sync. Tradeoff: confusing if users assume bidirectional sync — needs clear UI labeling.
   Implementation cost estimate: 0 new crates (extends `snk-library/files.rs`), 0 new deps, no OS-specific code, ~1 week. Test surface: filesystem latency edge cases.

### 10. Built-in screen recorder *trimmer*-only (post-capture only, no live record)
   Fit: 3/5  Reach: 2/5  Complexity (higher=cheaper): 4/5  Value: 2/5
   Reasoning: If we don't ship #5 (full video recording), the smaller-scope alternative is: drop a video file onto the library window, trim it, export. Implementation uses `<video>` element for preview + a tiny FFmpeg sidecar invocation for the trim cut (no encoding, just stream copy). Punts on the "record the screen" hard problem entirely. Honest about value: this is a 6 / 10 use case that doesn't move the needle the way scrolling capture does, but it's cheap and it lets us ship "video support" with a fraction of the effort. Lower-ranked because it's most useful if #5 doesn't happen.
   Implementation cost estimate: 0 new crates (extends `snk-library` schema with `kind` column), bundled FFmpeg sidecar (~20 MB after stripping), no OS-specific code, ~1.5 weeks.

---

## Alternatives Proposed

**Considered and rejected:**

- **Cloud sync (full bidirectional, multi-device).** Rejected. Violates audience-B posture; requires backend infrastructure, accounts, conflict resolution. The maintenance tax alone is multi-engineer-quarter. The reach gain is real but the cost is incompatible with a side-project life. (LAN-only sync in #4 captures most of the value without any of the lock-in.)
- **App Store / Microsoft Store distribution.** Rejected. Each store imposes its own review cycle, sandbox model, signing identity (Microsoft Store requires its own cert; macOS App Store sandboxing breaks our Accessibility-API auto-paste — it's literally incompatible with our core feature). Audience-B explicitly opts out of stores. The packaging changes alone (App Store: hardened runtime + entitlements review + sandboxing rewrite of clipboard/hotkeys; Microsoft Store: MSIX repackage + Partner Center maintenance) are not justifiable.
- **Self-hosted Sentry (full pipeline).** Rejected as proposed. We don't have infrastructure; running Sentry costs $20–50/mo + a VPS + TLS + maintenance + abuse handling. The user-facing GH-issue funnel form in #7 gets 80% of the value at 0% of the infra cost.
- **Plugin / extensibility API for end users.** Rejected. Sounds great, very expensive in reality: requires a stable wire-format API (which then constrains internal refactors permanently), a permission model (third-party plugins shouldn't get raw DB access), distribution + discovery, sandbox/isolation, and *forever* backwards compatibility. Every internal change becomes a compat review. The deep-link/URL-scheme approach in #8 satisfies 80% of "let me automate this" without any of these tradeoffs.
- **Auto-suggest tags from OCR text + filename heuristics.** Runner-up. Cheap (~3 days), useful, but not in top 10 because the tag UX is already decent and this is a polish item, not a feature.
- **Right-click "search image on web" / "OCR-then-search" / "translate text in this region".** Runner-up. Cheap individually but each one is a small thing; collectively could be a "context menu actions" mini-phase.
- **AI describe / AI alt-text on captures (local model via candle or remote API).** Rejected. Local model (candle/llama.cpp) adds 1–4 GB of weights to the install — a non-starter for an audience-B installer. Remote API requires accounts, API keys, costs money per call, and violates the no-telemetry posture. Maybe revisitable when good < 200 MB local models exist that run on CPU.

---

## Risks Identified

**Features that look small but explode:**

- **Linux support (#6).** The visible cost is "add a CI runner." The hidden cost is Wayland: it forces a UX rework of two flagship flows (region select, auto-paste). Once shipped, every bug report from "Wayland on KDE Plasma 6.7 with NVIDIA driver" becomes our problem. Defer until user signal.
- **Cloud sync / Microsoft Store / Apple App Store.** Each one quietly turns the project from "side project with installers" into a service business. Cloud sync needs uptime guarantees; stores need responsiveness to review feedback within SLA. None of these are okay for part-time maintenance.
- **Video recording (#5) without scope discipline.** "Record screen" is the user-facing 10% of the work; encoder integration, audio device enumeration, hardware encoder fallbacks, file-size tuning, pause-resume, cursor highlighting, and "why does my recording have green frames on this one Lenovo laptop" are the other 90%. Ship a brutally minimal v1 (no audio, single codec, no pause) or don't ship it.

**Features that create lock-in to a specific API/library:**

- **xcap.** Already a dependency lock-in for capture. If xcap breaks on a future macOS version (Sequoia → Tahoe → ...), we're forking. Worth tracking xcap's maintenance posture (single maintainer, mostly active, mostly fine — but a real risk).
- **Tesseract.** GPL-via-its-language-data-files distribution gets murky around language packs; the sidecar bundling already cost us multiple phase-7 fix commits. The Vision OCR replacement in #2 specifically de-risks this on macOS.
- **`tauri-plugin-global-shortcut`.** Wayland exposes hotkeys via a portal with limited compositor support; our hotkey model assumes "always works" which is X11/Win/Mac behavior.
- **EV cert (#1).** Locks us into one issuer + their HSM workflow; cert renewals are annual procurement events. Mitigation: factor `signCommand` so a different issuer is a settings change, not a code change. (Already largely true in the current `tauri.conf.json`.)

**Cross-cutting fragility:**

- **Auto-updater + signing.** Every new sidecar binary, every new plugin, every new bundle target adds a new artifact to sign, hash, and include in `latest.json`. Phase 7 already showed how fragile this is (5 follow-up commits to get Windows signing right). Each video/OCR/scrolling-capture feature that adds a sidecar adds another way for the release pipeline to break silently.
- **CI matrix.** We're at 3 runners (mac-arm64, mac-x64, win-x64). Linux makes 4. Adding video means another runner-specific dependency (hardware encoder availability). Every multiplier compounds.

---

## Open Questions

1. **What's the actual user signal?** We have no telemetry, so any prioritization is gut-feel about what *we'd* want. Is there a GitHub Issues/Discussions read on what users are asking for? If not, "ship #1 + #2 + #7" before guessing on #3 / #5 might be the honest move.
2. **What's the budget posture?** EV cert is ~$300/yr ongoing. Cloud-anything would be ongoing infra cost. LAN-sync in #4 has zero cost but development time. Should I assume "money is cheaper than time" or vice versa?
3. **How religious is the "no servers" rule?** A GH-issue-funnel for crash reports (#7) is technically just a URL the user clicks — zero servers. But if user signal eventually says "I'd really like to opt into anonymous crash reporting," that's a different conversation.
4. **Will we add Linux to keep the workflow honest?** Even without shipping Linux installers, a *compile-only* Linux job would catch a lot of bugs cheaply. It's worth its weight in CI minutes.
5. **What's the desired pace?** If the goal is 2–3 features over 6 months, #1, #2, #7 are the no-brainers and we deeply consider only one of #3/#4/#5. If the goal is 8 features in a year, the picture changes.
6. **Vision OCR (#2): `objc2` Rust bindings or a tiny Swift sidecar?** The Swift sidecar is more robust against `objc2` ecosystem churn but adds yet another binary to sign+bundle. I'd prefer `objc2-vision` if maintained, fall back to a 50-line Swift sidecar if not.
