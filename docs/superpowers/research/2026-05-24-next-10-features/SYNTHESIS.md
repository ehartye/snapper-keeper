# Synthesis — Next 10 Features for snapper-keeper after Phase 7

**Date:** 2026-05-24
**Inputs:** 4 perspectives × 2 rounds (Business/Strategy, User/Consumer, Maintainer, Competitive Landscape)
**Question:** Identify the next 10 enhancements/features to build for snapper-keeper after phase 7, scored on (a) Product fit, (b) Market reach / competitive advantage, (c) Complexity (higher = cheaper), (d) Overall value.

---

## Executive Summary

After cross-pollination, four perspectives converged on a remarkably coherent picture: the highest-ROI work is *not* a single big competitive parity bet (Snagit-killer scrolling capture), but rather **a bundle of cheap features that compound the existing OCR, clipboard, and library investments**, paired with **one defensive distribution fix (EV cert)** and **one identity-defining headline feature (scrolling capture)**. The most under-priced finding in the whole exercise — flagged independently by three perspectives — is **PII auto-redact**: it reuses 90% of shipped infrastructure (Tesseract bounding boxes + the existing blur tool) and is the kind of feature competitors with cloud-AI redaction structurally can't match without breaking their own privacy stories.

The unified top 10 (highest value first): **(1) Scrolling capture, (2) EV code-signing cert, (3) PII auto-redact suggestions, (4) macOS Vision OCR backend, (5) Post-capture "Text Actions" OCR overlay, (6) BYO-bucket share-link upload, (7) Paste-as-plain-text + paste-as-image modifiers, (8) GH-issue crash funnel, (9) URL-scheme / deep-link automation, (10) Eyedropper + pixel-measure annotation tools.** Confidence: **High** on items 1–5 and 7–9 (3+ perspectives independently aligned after cross-pollination); **Medium** on 6 (right architecture, real complexity); **Medium** on 10 (cheap and useful, but lower reach than the others).

This list deliberately *drops* four candidates that any single perspective would have included: cloud sync (rejected unanimously), Microsoft Store distribution (redundant with EV cert, sandbox-hostile), full plugin API (forever maintenance contract — URL-scheme delivers 80% at 5% cost), and Linux support (Wayland's synthetic-input block is a dealbreaker for caret-anchored auto-paste — the product's defining UX). Video/GIF survives as a runner-up only in the GIF-only-via-gifski form, with a license caveat flagged by the Competitive perspective.

---

## Positions Summary

### Business / Strategy
- **Stance:** Funnel-first. Defensive moves (EV cert, crash telemetry) precede feature-building because they amplify every later feature. Beware identity drift toward "do-everything productivity suite."
- **Top 5 (Round 1):** EV cert, Scrolling capture, macOS Vision OCR, Text Actions OCR overlay, GIF recording (gifski-only).
- **Key differences:** Only perspective putting EV cert at #1. Split video into GIF-only (in) and MP4 (out). Reframed seed's "cloud sync" as LAN-only.
- **Top risks raised:** Identity drift; per-OS maintenance compounding; EV cert lock-in with annual recurring cost; GIF as slippery slope to video.

### User / Consumer
- **Stance:** Personas-first. Walk seven concrete journeys; features must *eliminate friction users already feel*, not add capability for its own sake. Privacy posture is non-negotiable.
- **Top 5 (Round 1):** Scrolling capture, PII auto-redact, Eyedropper + pixel-measure, Paste-as-plain-text, Quick-share targets.
- **Key differences:** Only perspective surfacing PII auto-redact (then adopted by all in R2). Excluded EV cert from list ("operational, not user-facing"). Preferred ephemeral LAN-QR share over persistent sync.
- **Top risks raised:** Scope creep on video; "just add AI" pressure; cross-platform divergence drift; PII false negatives if framed as automatic.

### Maintainer
- **Stance:** Six-months-from-now triage. Every feature is a permanent tax — CVE tracking, OS API drift, signing pipeline complexity. Calibrated from real phase 5 + phase 7 plan-vs-actual data.
- **Top 5 (Round 1):** EV cert, macOS Vision OCR, Scrolling capture, LAN clipboard sync, Video/GIF.
- **Key differences:** Reframed crash reporting as "one-click GH-issue funnel" (no servers, no network code) — adopted by all in R2. Introduced URL-scheme deep-links instead of plugin API. Honest 6–10 week estimate for video deflated everyone else's complexity scores.
- **Top risks raised:** Linux Wayland fundamentally hostile to auto-paste; every new sidecar pays the signing-pipeline tax again; xcap/Tesseract dependency lock-in; CI matrix compounding.

### Competitive Landscape
- **Stance:** Map every competitor's killer feature; pick gaps that close parity *or* sharpen differentiation. The moat is the *combination*, not individual features.
- **Top 5 (Round 1):** Scrolling capture, Video/GIF, Local share-link via BYO uploader, Multi-image gallery, Cross-device sync.
- **Key differences:** Surfaced "Snagit at $63 / CleanShot at $29+$8/mo cloud" pricing anchor. Originated BYO-bucket share-link (adopted by all). Concrete competitor citation per item.
- **Top risks raised:** Competitive parity is a trap (be the differentiated tool, not a worse Snagit); video changes product identity (Loom, not Snagit); plugin API is forever maintenance contract; gifski AGPL/commercial licensing wrinkle.

---

## Cross-Pollination Results

### Hybrid Approaches

1. **"OCR Surfaces" mini-phase.** Three perspectives independently identified that phase 5's OCR investment has at least 3–4 distinct user-facing features still untapped: PII auto-redact (User #2), Text Actions selectable overlay (Business #4), Selective-area OCR (Maintainer runner-up), and the upgrade to macOS Vision (all four). Maintainer and User both noted in R2 these should be evaluated as one coherent phase — ~3 weeks ships 3 high-value, demo-friendly capabilities that compound an investment we've already paid for.

2. **"Post-capture toolbar action row" unifying frame.** User's R2 Insight #5 ties together four candidates that all individually appeared in different perspectives: Copy text (OCR), Copy as link (BYO bucket), Copy as image (default), Save to folder X. Combined with eyedropper and pixel-measure on the canvas side, this becomes a single 1–2 week phase that materially increases the perceived feature density of the post-capture flow.

3. **"BYO is a posture, not a feature."** Business's R2 Insight #4: Competitive's BYO bucket share-link, Maintainer's iCloud/OneDrive backup-only, and User's LAN QR share all share the pattern "user supplies the infrastructure, snapper-keeper supplies the integration." Promoting this from coincidence to explicit product positioning unifies 3+ candidates and sharpens the differentiation against Snagit/CleanShot (their cloud) and ShareX (BYO but sysadmin-only).

4. **"Phased polish + headline" sequencing.** Business R2 Insight #7 and Maintainer R2 Insight #4 independently propose abandoning the flat-ranking frame in favor of a phasing plan: a quick polish/defensive phase (EV cert + paste-as-plain-text + PII redact + GH-funnel + eyedropper, ~2–3 weeks) followed by one identity-defining feature per phase (scrolling capture, then BYO share, then OCR surfaces).

5. **"GH-issue funnel" beats both Sentry and silence.** Maintainer's reframe — local panic-hook JSON dump + one-click "Submit crash report" button that opens a pre-filled GitHub issue in the browser — got adopted by Business (replacing their self-hosted Sentry) and Competitive (which had rejected crash reporting entirely). Zero servers, zero network code in the app, zero recurring cost, full user consent, *and* turns crashes into public artifacts that help the GitHub project look alive.

### Challenges & Rebuttals

1. **EV cert ranking.** Business ranked #1; User dropped it entirely. The R2 resolution: User conceded it's a *prerequisite* (Journey 0: "user installs at all"), not a competing feature, and should ship *alongside* the first user-visible work rather than competing with it for slot #1. Synthesis treats it as the funnel-multiplier sitting above the visible-feature list.

2. **Plugin / extensibility API.** Business and Competitive included scoped versions; Maintainer hard-rejected; User deferred indefinitely. In R2, Business and Competitive both conceded Maintainer's "URL-scheme deep links deliver 80% of value at 5% of cost without the forever-API-compat tax." Strong rebuttal — plugin API drops out of synthesis entirely.

3. **Microsoft Store.** Only Business included it. User, Maintainer, and Competitive each independently flagged that Store sandbox models break the product's two core OS integrations (global hotkeys + accessibility-API auto-paste). Business conceded in R2: "Three perspectives independently say the sandbox model breaks the product's core flows. They are correct on the technical merits; I underweighted the sandbox compat problem." Drops from synthesis.

4. **Linux.** Three perspectives included it in 6–10 range; User dropped it. Maintainer's R2 Insight #5 surfaced the killer fact missed in R1: **Wayland blocks synthetic-input by design**, which means caret-anchored auto-paste (a defining feature) cannot work on Wayland without compositor-specific portals that don't universally exist. After cross-pollination, all four converge on "drop from top 10; ship compile-only CI as a freebie."

5. **Video framing.** User and Competitive had combined video+GIF; Business split GIF-only as cheaper; Maintainer's R2 calibration data showed even GIF-only is 3–4 weeks (not 1–2). Resolution: GIF-only-via-gifski is the only acceptable scope, MP4 is out; *and* the gifski AGPL/commercial license wrinkle (Competitive R2 Insight #7) must be resolved before committing. This pushes video to runner-up rather than top 10.

6. **Sentry vs. GH-issue funnel.** Business and Competitive each had self-hosted Sentry or rejected the candidate; Maintainer's "one-click GH-issue" reframe won unanimously in R2.

### Converging Themes

These are the highest-confidence picks because multiple perspectives independently or reactively aligned:

| Feature | Confidence Driver |
|---|---|
| **Scrolling capture** | 3/4 ranked #1, 4/4 ranked top 3. Anchored by Snagit/CleanShot/ShareX/Shottr/Screenpresso parity gap. |
| **EV cert** | 3/4 ranked top 3 (User the only dissent — and conceded in R2 as Journey 0 prerequisite). |
| **macOS Vision OCR** | 4/4 in top 6. Compounds phase 5 OCR investment, removes Tesseract sidecar from Mac, ~50 MB installer reduction. |
| **PII auto-redact** | User originated. R2: Business called it "strategic gold," Maintainer called it "the best single suggestion in any of the three rounds." Competitive: "top 3 in synthesis." All four converge upward in R2. |
| **Paste-as-plain-text** | User originated. R2: Maintainer called it "highest-leverage-per-hour item on any list." All others adopt. |
| **GH-issue crash funnel** | Maintainer originated. All others adopt in R2; replaces both self-hosted Sentry (Business) and rejection (Competitive). |
| **URL-scheme deep links** | Maintainer originated. Business and Competitive replace their plugin-API candidates with this in R2. |
| **Text Actions OCR overlay** | Business originated. User: "polish that turns OCR from background search to demo-in-5-seconds." Competitive: "single most under-priced item across all three positions." |
| **BYO-bucket share-link** | Competitive originated. Maintainer in R2: "strictly better" than backup-only-mode. User and Business both upgrade their own share-related candidates to this framing. |
| **No cloud sync (full bidirectional)** | 4/4 reject. User R2 Insight #8: "the strongest cross-perspective NO in the round." Should be invalidated outright in decisions log. |

---

## Final Unified Top 10

### 1. Scrolling capture (region + window, diff-stitch)
**Fit: 5/5 · Reach: 5/5 · Complexity (higher=cheaper): 2/5 · Value: 5/5**
**Confidence: High**

**Reasoning:** Three of four perspectives ranked this #1. The #1 feature competitors charge $63 for. Every paid Mac/Win competitor ships it (Snagit, CleanShot, Shottr, ShareX, Screenpresso). Reviewers anchor on it. ShareX's diff-stitch algorithm is OS-agnostic enough that we don't need separate per-OS scroll-synthesis paths — programmatic scroll injection via existing hotkey + image-similarity stitching is the proven approach. Single new `snk-scroll-capture` crate, no new sidecar, no new schema. v1 should explicitly *not* attempt sticky-header detection or lazy-load awareness (CleanShot polish); accept "good enough on browsers and docs, fragile on canvas/virtualized lists." This is the feature that turns snapper-keeper from "decent" to "I can switch from Snagit." Maintainer's honest estimate: 3–4 weeks.

**Key risk:** Per-OS scroll API drift requires ongoing maintenance. Once shipped, every new feature must consider scrolling-capture compatibility.

### 2. EV code-signing cert (Windows SmartScreen)
**Fit: 5/5 · Reach: 4/5 · Complexity (higher=cheaper): 5/5 · Value: 5/5**
**Confidence: High**

**Reasoning:** Phase 7 ships signed installers but they're not *reputation-trusted* until ~3000 installs. EV certs are trusted day one. The dotnet `sign` path, Azure profile, and Tauri `signCommand` are already wired (`tauri.conf.json:104`) — switching to an EV cert is procurement + identity verification + a one-line config change. Zero engineering. Approximately $300–600/year recurring. This is a *prerequisite* to feature work, not a feature competing with it — ship it alongside (or just before) the first user-visible item. Without it, the SmartScreen "Run anyway" dialog kills the Windows funnel for every other feature on this list.

**Key risk:** Annual procurement event; HSM/token binds you to one issuer; if Eric stops paying, every existing user's auto-updater chain breaks at the next signing (publisher mismatch). Mitigation: factor `signCommand` so issuer is a config change, not a code change (already mostly true).

### 3. PII auto-redact suggestions
**Fit: 5/5 · Reach: 4/5 · Complexity (higher=cheaper): 4/5 · Value: 5/5**
**Confidence: High**

**Reasoning:** The highest-ROI feature in the entire research package. Phase 5 already produces OCR text + bounding boxes per capture. The blur tool already exists in the annotator (from the 2026-05-22 pixelation polish phase). Pattern-matching for emails, phone, credit-card-shaped, SSN-shaped, IP addresses is ~200 LoC per category. Wiring "Suggest redactions" as a button in the annotate window that pre-populates blur shapes (user confirms before save) is ~3–5 days of work. This is the *brand-defining* feature that lands in every "snapper-keeper does X that Snagit doesn't attempt" comparison post — and it's structurally hard for cloud-AI competitors to match without breaking their own privacy stories. UI must frame as "suggestions, you confirm" with non-dismissable confirmation (never auto-apply: false negatives leak data).

**Key risk:** False-negative trust trap — if the badge says "redacted" and we miss a non-standard format, users leak data while believing they're safe. Always require explicit user confirmation; never auto-apply.

### 4. macOS Vision OCR backend (replace Tesseract on Mac)
**Fit: 5/5 · Reach: 3/5 · Complexity (higher=cheaper): 3/5 · Value: 4/5**
**Confidence: High**

**Reasoning:** Apple Vision is dramatically more accurate than Tesseract on UI screenshots (mixed fonts, antialiasing, low-res, handwriting, multilingual). Ships in the OS — no sidecar, no tessdata. `snk-ocr` already has a queue + backend abstraction; a `MacOcrBackend` is a contained add via `objc2-vision` or a tiny Swift sidecar. Wins twice: (a) materially better OCR = better library search = the whole product feels smarter, (b) deletes ~50 MB of Tesseract bundle from the Mac installer + permanently removes a class of phase-7-style sidecar bundling fragility. Tesseract stays as the Windows / cross-platform fallback. Per-OS quality divergence is real but acceptable; mitigate by re-OCR-ing the library on first run when backend changes so search results stay consistent on each machine.

**Key risk:** Cross-OS search divergence (Mac users find text Windows users don't, and vice versa). `objc2` ecosystem churn — fallback plan is a 50-line Swift sidecar.

### 5. Post-capture "Text Actions" OCR overlay
**Fit: 5/5 · Reach: 5/5 · Complexity (higher=cheaper): 4/5 · Value: 4/5**
**Confidence: High**

**Reasoning:** Win11 Snipping Tool's headline 2025 feature; macOS Live Text's headline feature; PowerToys Text Extractor's whole product. We *already* pay the OCR cost in phase 5 — we just don't surface it. Display OCR text as a selectable overlay on the captured image plus a "Copy text" button on the post-capture toolbar. Sub-1-week effort, mostly frontend (canvas overlay + selection state). Microsoft just spent millions of dollars normalizing this UX — fast-follow on it. Compounds with #3 (redaction) and #4 (Vision OCR backend) since all three reuse the same OCR pipeline and bounding-box data.

**Key risk:** Quality of selectable overlay depends on OCR accuracy. Less of an issue once #4 lands on Mac.

### 6. BYO-bucket share-link upload (S3 / R2 / Backblaze / custom endpoint)
**Fit: 5/5 · Reach: 4/5 · Complexity (higher=cheaper): 4/5 · Value: 4/5**
**Confidence: Medium**

**Reasoning:** Steals the most viral CleanShot Cloud workflow ("send Slack a link, not a PNG") while preserving the no-servers identity. User configures credentials once for their own S3-compatible bucket; post-capture toolbar gets `Copy share link`. ShareX's SXCU model proves the pattern; we wrap it in a polished consumer UX instead of XML-config. Engineering: upload abstraction trait + 2–3 backend implementations (S3-compatible covers most) + OS keyring credential storage + Settings UI. Maintainer estimate: ~2 weeks. Compared to alternative "cloud sync" framings, this is uniquely defensible: user owns the bucket, no monthly fee, no third party reads images.

**Key risk:** Onboarding friction (users who don't have an S3 account won't use it). Mitigation: bundle a "no setup required" zero-config option later (e.g., LAN ephemeral QR share — see runner-ups). Document clearly that bucket security is user's responsibility.

### 7. Paste-as-plain-text + paste-as-image modifiers
**Fit: 5/5 · Reach: 5/5 · Complexity (higher=cheaper): 5/5 · Value: 4/5**
**Confidence: High**

**Reasoning:** The cheapest legitimate feature on any of the four perspective lists. Adding a modifier to the existing clipboard popup paste action (Shift+Enter strips RTF/HTML before `arboard.set()`; Cmd/Ctrl+Shift+Enter pastes as an image of the text) is ~50 LoC plus key bindings — half-day to one-day max. Maccy, Paste, Alfred, Raycast all ship it; it's table stakes for any "premium clipboard manager" comparison. Maintainer R2: "highest-leverage-per-hour item on any list." Ships as a velocity-filler between bigger phases.

**Key risk:** None material. Pure win.

### 8. GH-issue crash funnel (panic_hook → JSON dump → one-click pre-filled issue)
**Fit: 4/5 · Reach: 2/5 · Complexity (higher=cheaper): 5/5 · Value: 4/5**
**Confidence: High**

**Reasoning:** Maintainer's reframe of "crash reporting" beats every alternative. Local `panic_hook` writes a JSON crash dump (already specified in design §9.1); a "Submit crash report" button in the library opens a pre-filled GitHub issue via the user's browser. **Zero servers, zero network code in the app, zero recurring cost, full user consent every time.** Today the project flies blind on crashes — every silent crash becomes an unreproducible future issue. Cost: ~3–4 days. Bonus: turns crashes into public artifacts on the GitHub project (marketing-as-changelog effect). Survives every perspective's privacy objection because the user actively initiates the network request from their browser.

**Key risk:** Lower-traffic projects may have GH-issue spam from copy-paste errors. Mitigation: link to a template with structured fields so issues are self-categorizing.

### 9. URL-scheme / deep-link automation (`snapper-keeper://capture/region` etc.)
**Fit: 4/5 · Reach: 4/5 · Complexity (higher=cheaper): 4/5 · Value: 4/5**
**Confidence: High**

**Reasoning:** Wins the plugin-API tension three ways: cheaper, safer, and gets us into the integration ecosystems we don't have to build. Register snapper-keeper as a custom URL scheme handler (Tauri 2's `tauri-plugin-deep-link`); expose a small whitelisted command surface (trigger captures, open library to a tag, paste a clipboard item by id). Lets snapper-keeper plug into Raycast, Alfred, Stream Deck, AutoHotkey, Hammerspoon, Shortcuts.app — multiplying perceived value via integrations *we don't build*. Each of those communities writes "best X integrations" listicles for free. ~1 week + careful input validation. Strictly preferable to a full plugin API because it has no compat contract (we don't ship code into another app's process).

**Key risk:** URL injection — strictly whitelist commands and validate arg shapes. Document the URL grammar publicly and version it.

### 10. Eyedropper + pixel-measure annotation tools
**Fit: 5/5 · Reach: 3/5 · Complexity (higher=cheaper): 5/5 · Value: 3/5**
**Confidence: Medium**

**Reasoning:** Two small additions to the existing Konva canvas. Eyedropper reads a pixel on the underlying image and copies hex to clipboard (or appends to a callout). Pixel-measure draws a labelled line/rectangle showing pixel distance. Both <500 LoC each. Table stakes for designers and UI engineers — the audience most likely to upgrade from native screenshot tools. CleanShot has the loupe, Shottr has the ruler, nobody combines both cleanly. Velocity-filler that compounds the annotator's lead. Lower reach than other items because keyboard-heavy users won't use them — but the cost is so low that the value calculus still wins.

**Key risk:** None material.

---

## Runner-ups (11–15)

11. **GIF-only recording via gifski sidecar** — Honest 3–4 weeks. Resolves the "competitor parity" pressure on video without paying the full FFmpeg/MP4 cost. **Blocker:** gifski is AGPLv3 / commercial dual-license; bundling as sidecar in MIT/Apache snapper-keeper may require source disclosure or commercial license. Resolve before commit. If gifski is unviable, the `image` crate's GIF encoder is a slower fallback.

12. **LAN ephemeral QR share** — One-shot localhost HTTPS server bound to the LAN interface + QR-rendered URL, link dies in 5 minutes. Pragmatic answer to "send this one capture to my phone right now" without pairing UX or persistent sync. ~1–2 weeks. Cheaper than full LAN sync; potentially more universal than BYO bucket. Self-signed cert UX on a phone is the main wart.

13. **iCloud / OneDrive backup-only mode** — One-way file mirror of capture PNGs (not the SQLite DB) to a user-selected synced folder. ~1 week, no new deps. Honest about being "backup, not sync." Addresses the most common "I want cloud sync" subtext: "don't lose my screenshots when my laptop dies." Lower-priority than BYO bucket but cheaper and arguably more universally useful.

14. **Multi-image gallery / contact-sheet stitching** — Select N library items, produce a single stitched/sheet image with optional captions and step numbering. Pairs naturally with the existing step markers. Hits Snagit's "documentation templates" use case without building the whole templates engine. Cheap (Konva is in the bundle).

15. **Smart-section / saved-search rules in library** — "Auto-tag everything from Figma," "screenshots older than 30 days move to archive." Compounds phase 6's library investment at near-zero cost. Quality-of-life polish; lower reach than features 1–10 but the right thing to ship in any quiet release.

---

## Recommendation

### Build order (proposed phasing)

**Phase 8 — Polish + defensive (~2–3 weeks, single release)**
- EV code-signing cert (#2)
- GH-issue crash funnel (#8)
- Paste-as-plain-text modifiers (#7)
- Eyedropper + pixel-measure annotation tools (#10)
- Compile-only Linux CI job (free, catches portability bugs)

Ship as a single release labeled "polish + install legitimacy." Five items in 2–3 weeks materially upgrade the changelog narrative without consuming any large engineering investment.
**Confidence: High.**

**Phase 9 — Headline competitive parity (~4–6 weeks)**
- Scrolling capture (#1)

Single big feature. The competitive gap-closer that turns snapper-keeper from "decent" to "switch from Snagit." Ship alone — its complexity warrants the entire phase budget.
**Confidence: High.**

**Phase 10 — OCR surfaces compounding phase 5 (~3 weeks)**
- PII auto-redact suggestions (#3)
- Text Actions OCR overlay (#5)
- macOS Vision OCR backend (#4)

Three features that share infrastructure (OCR pipeline + bounding boxes + canvas overlay) ship together with maximum architectural reuse. Brand-amplifying — privacy-first identity feature in #3, Microsoft-validated UX in #5, materially better Mac product in #4.
**Confidence: High.**

**Phase 11 — BYO posture for sharing (~3 weeks)**
- BYO-bucket share-link upload (#6)
- URL-scheme / deep-link automation (#9)

Establishes "BYO" as an explicit strategic theme tying snapper-keeper to existing ecosystems (your S3, your Raycast, your Stream Deck). Sharpens differentiation against Snagit/CleanShot (their cloud) and ShareX (BYO but sysadmin-only).
**Confidence: Medium** — BYO bucket onboarding friction is real; URL-scheme value depends on Raycast/Alfred user appetite.

**Phase 12+ — Stretch (defer to user signal)**
- Runner-ups 11–15 above. Pick based on what arrives via GitHub Issues after Phase 11 lands and the user base is no longer zero.

### Key tradeoffs to accept

- **Identity over breadth.** We're deliberately dropping video MP4, plugin API, Microsoft Store, and Linux — all of which had ≥1 perspective rooting for them — to keep "local-first, privacy, capture + clipboard, no servers" as a sharp positioning rather than blurring into a productivity suite.
- **Cross-OS divergence is acceptable on OCR.** Vision OCR creates a quality gap between Mac and Windows search results. We accept this in exchange for materially better Mac search + a smaller Mac installer.
- **Annual recurring cost (~$300–600/yr for EV cert).** First time the project has a vendor contract. Worth it for the funnel multiplier; mitigate by factoring `signCommand` for issuer-portability.
- **No telemetry remains as-is.** "Zero user signal" remains the prioritization debt all four perspectives flagged. Accept that future ranking is still partly speculation; ship features that compound existing identity (lower regret) over speculative big bets.

### Mitigations for top risks

- **Scrolling capture maintenance tax:** Limit v1 to programmatic scroll-injection + diff-stitch; explicitly defer sticky-header detection and lazy-load awareness to v2. Document scope as "good enough for docs/browsers/chat threads; fragile on virtualized lists."
- **PII redact false negatives:** UI must be "Suggested redactions — confirm or adjust each." Never auto-apply. Surface confidence levels per detection. Always require explicit user confirmation before save.
- **EV cert vendor lock-in:** Factor `signCommand` in `tauri.conf.json` so swapping issuers is config, not code (already mostly true post-phase 7).
- **gifski licensing (runner-up #11):** Resolve license compatibility before commit. Alternatives: `image` crate GIF encoder, custom encoder, or paying for commercial license.
- **URL-scheme injection (#9):** Whitelist commands strictly; validate arg shapes; version the URL grammar; document publicly.
- **Cross-OS OCR divergence (#4):** Re-OCR the library on first run when backend changes so per-machine search stays consistent.

### Investigate further before deciding

1. **Eric's actual time budget per week** — phasing above assumes ~10 hrs/week sustained. If smaller, Phase 8 alone may take a quarter. If larger, Phase 9+10 can be parallel.
2. **gifski license compatibility** with intended snapper-keeper license — needs answer before runner-up #11 is committable.
3. **"Personal utility" vs. "product seeking users"** — User R2 Insight #7 flagged this. The phasing above weights toward "product seeking users" (EV cert + Text Actions + share link). If "personal utility" wins, drop EV cert and share link, promote smart-sections + Vision OCR + redact.
4. **One-time opt-in install counter** — strict zero-telemetry is creating prioritization debt. Worth raising as a posture-revisit decision (even if the answer is still no) so it stops blocking every future planning exercise.

---

## Decision Record (ADR)

# ADR: Next 10 Features for snapper-keeper (post Phase 7)

## Status
Proposed — 2026-05-24

## Context

Phase 7 (signing, notarization, auto-updater, release pipeline) shipped to `main`. The project is now in a position to add user-visible enhancements but faces several constraints: single-maintainer side-project economics, audience-B positioning ("share-friendly side project — no servers, no accounts, no telemetry"), zero current user signal (no analytics by design), and a tight architecture (one plugin per feature, `snk-library` owns all persistence) that punishes any feature requiring servers or breaking the local-first posture.

Four perspectives — Business/Strategy, User/Consumer, Maintainer, Competitive Landscape — independently surveyed candidates in Round 1, then cross-pollinated in Round 2. The strongest convergence patterns: (a) scrolling capture is the single largest user-visible competitive gap, (b) EV code-signing cert is the single largest install-funnel friction on Windows, (c) phase 5's OCR investment is materially under-leveraged with at least three distinct user-facing features still untapped, (d) "BYO storage / user supplies infrastructure" is a coherent posture that ties multiple cross-device candidates into one identity-amplifying theme, (e) full plugin API, Microsoft Store distribution, Linux support, and full cloud sync should all be dropped despite each having ≥1 advocate, because each conflicts with the product's defining identity or with a single-maintainer cost ceiling.

The deliverable is a phased build order (5 sub-phases over an estimated 14–17 weeks of effort) plus an explicit drop list with reasoning.

## Decision

The next 10 features for snapper-keeper, in build order:

**Phase 8 — Polish + Defensive (single release, ~2–3 weeks)**
1. **EV code-signing cert** (procurement + `signCommand` config swap)
2. **GH-issue crash funnel** (panic-hook JSON dump + browser-opened pre-filled issue)
3. **Paste-as-plain-text + paste-as-image modifiers** on clipboard popup
4. **Eyedropper + pixel-measure annotation tools** on Konva canvas
5. *(Bonus, free)* Compile-only Linux CI job

**Phase 9 — Headline (single release, ~4–6 weeks)**
6. **Scrolling capture** (region + window, diff-stitch v1, no sticky-header / lazy-load polish)

**Phase 10 — OCR Surfaces (single release, ~3 weeks)**
7. **PII auto-redact suggestions** (Tesseract bounding boxes + existing blur tool + pattern matching, user-confirmation required)
8. **Post-capture Text Actions OCR overlay** (selectable text overlay + Copy text button)
9. **macOS Vision OCR backend** (`MacOcrBackend` behind existing `snk-ocr` interface; deletes Tesseract from Mac bundle)

**Phase 11 — BYO Posture (single release, ~3 weeks)**
10. **BYO-bucket share-link upload** (S3-compatible + custom endpoint + OS keyring)
11. **URL-scheme / deep-link automation** (`snapper-keeper://`, whitelisted commands, integrates with Raycast/Alfred/Stream Deck/Hammerspoon)

*(Per the original question, the unified "top 10" is items 1–10 above; URL-scheme automation slots into Phase 11 as the 10th user-visible item, with EV cert serving as funnel-prerequisite #2 in the strict ranking but #1 in the build order alongside Polish phase.)*

## Alternatives Considered

**Considered and dropped (with reasoning):**

- **Full plugin / extensibility API (JS sandbox or otherwise).** 3 of 4 perspectives concluded URL-scheme deep links deliver 80% of the value at 5% of the cost without the forever-API-compat tax. Plugin API would impose a stable wire-format contract that constrains every future internal refactor. Replaced by item 11 (URL-scheme automation).

- **Microsoft Store distribution.** 3 of 4 perspectives concluded the Store sandbox model is incompatible with the product's two defining OS integrations (global hotkeys + accessibility-API auto-paste). Sandbox rewrite would ship a crippled product under the same name. EV cert alone solves the SmartScreen problem (item 1).

- **Linux support (full Wayland + X11 installers).** All 4 perspectives converged on "drop" after Maintainer R2 surfaced the killer fact: Wayland blocks synthetic-input by design, which means caret-anchored auto-paste — the product's defining UX — cannot work on Wayland without compositor-specific portals that don't universally exist. Compile-only CI is the maximum commitment.

- **Full cloud sync (bidirectional, with servers).** Unanimously rejected — the strongest cross-perspective NO in the entire research package. Violates audience-B posture (no servers, no accounts, no telemetry). Should be invalidated outright in the project's decision log so it stops appearing in future planning exercises.

- **Self-hosted Sentry crash telemetry.** Maintainer's "GH-issue funnel" reframe is strictly better on every axis (cost, infra, privacy, marketing-via-public-issues). Adopted as item 2.

- **Video / MP4 recording with audio.** All 4 perspectives concluded the scope (6–10 weeks honest) and identity drift ("now we're Loom, not Snagit") outweigh the reach. Survives only as runner-up #11 (GIF-only via gifski, with license caveat to resolve).

- **LAN-only persistent clipboard sync.** Demoted to runner-up #12 (LAN ephemeral QR share) because the ephemeral form solves the most common use case ("send this one capture to my phone") without pairing UX, persistent state, or a sync protocol — and at ~30% the cost.

- **iCloud / OneDrive backup-only mode.** Demoted to runner-up #13. BYO-bucket share-link (item 10) is strictly more powerful — it solves both "don't lose my screenshots" and "send a link to Slack" — though backup-only is cheaper and could ship first if BYO is too ambitious.

- **Multi-image gallery / capture templates / step-tutorial builder.** Demoted to runner-up #14. Strong identity fit (compounds existing step markers + tags + library) but Maintainer's "real product effort, not a feature" calibration deflated complexity scores. Worth shipping eventually; not in the next 10.

- **Smart-section / saved-search rules in library.** Runner-up #15. Quality-of-life polish; lower reach than the top 10 but right thing to ship in any quiet release.

## Consequences

### Positive
- **Clean funnel:** EV cert removes the Windows SmartScreen friction that gates every other feature's reach.
- **Crisis-aware:** GH-issue crash funnel gives the maintainer first visibility into real-world failures.
- **Identity-amplifying:** PII redact, Text Actions overlay, Vision OCR all compound the phase 5 investment instead of building new product areas. Each one strengthens the "screenshot tool that respects you" pitch.
- **Differentiation over parity:** Scrolling capture closes the parity gap; BYO posture (items 10–11) sharpens the differentiation against Snagit/CleanShot/Paste (their cloud) and ShareX (BYO but sysadmin-only).
- **Phasing discipline:** Each phase is one release with a coherent narrative — Polish + Defensive, Headline, OCR Surfaces, BYO Posture. Easier to write release notes; easier to maintain; easier to demo.

### Negative
- **Annual recurring cost begins** (~$300–600/yr for EV cert). First vendor contract on the project.
- **Per-OS divergence on OCR.** Mac users will see different search results than Windows users. Mitigated by re-OCR on backend change, but the divergence is real.
- **Estimated 14–17 weeks** total for Phases 8–11. Long calendar window for a side project; risk of stalling between phases.
- **No video/GIF in the top 10.** A real competitive feature (Snagit, CleanShot, Snipping Tool all ship it) is intentionally deferred. Will be the #1 ask from any user who arrives expecting it.
- **No Linux.** Closes the door on a small-but-vocal Linux dev audience. Compile-only CI keeps the option alive but ships nothing usable on Linux.
- **No telemetry remains** — the prioritization debt all four perspectives flagged is unresolved. Future planning rounds will continue to be speculation-driven.

### Risks
- **Scrolling capture v1 fragility on virtualized lists / canvas content.** Will produce some legitimately bad outputs. Document the limitation; don't claim "works everywhere."
- **PII redact false negatives.** Users may trust the "Suggested redactions" badge and not double-check. UI must require explicit confirmation per detection. Never auto-apply.
- **EV cert publisher mismatch on lapse.** If the cert is not renewed, every existing user's auto-updater chain breaks. Annual procurement is a hard deadline event.
- **BYO bucket onboarding friction.** Users without an existing S3/R2/Backblaze account won't use item 10. Mitigation: pair with runner-up #12 (LAN ephemeral QR share) as a zero-config alternative.
- **URL-scheme injection (item 11).** Strict whitelist + arg validation required from day one; documentation must be unambiguous.
- **Identity drift.** Each phase as designed compounds the existing identity, but the cumulative "are we still a focused tool?" question must be re-asked at the end of Phase 11.
- **gifski licensing** (runner-up #11) — resolve before commit. AGPLv3 vs. project license compatibility may force `image` crate fallback.
