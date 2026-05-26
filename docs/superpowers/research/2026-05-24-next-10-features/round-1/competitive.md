# Competitive Landscape — Round 1

Perspective: **Competitive Landscape**
Date: 2026-05-24
Lens: What do direct competitors ship, what do their fans love, and where does snapper-keeper trail or lead them?

## Competitive Map

### Screen capture

| Tool | Platform | Price | Killer features | Audience | Weakness |
|---|---|---|---|---|---|
| **Snagit** | Win + Mac | Paid (~$63 one-time, perpetual + paid upgrades) | Scrolling capture, Grab Text OCR, templates ("create from templates" → visual docs), step tool, video → GIF, Screentelligence layout AI | Pro doc/tech-comms users, support teams, trainers | Heavy, slow startup, "enterprise feel," paid upgrades |
| **CleanShot X** | Mac only | $29 one-time + optional cloud sub ($8/user/mo team) | Best-in-class scrolling capture (sticky headers, lazy-load aware), screen freeze, self-destructing cloud links, GIF/video, instant share URLs, hide desktop icons | Mac power users, designers, devs | Mac only, cloud tier is the upsell |
| **ShareX** | Win only | Free OSS | Kitchen sink: custom uploaders (any S3/imgur/your-server via SXCU JSON), workflows (different after-capture chains per hotkey), scrolling capture (diff-based), video + GIF, OCR, ARM64 in 2026, Avalonia editor rewrite | Power users, tinkerers, support agents | Cluttered UX, Windows only, intimidating for casual users |
| **Greenshot** | Win primarily | Free OSS | Quick region capture, Office plugin (paste into Word/PowerPoint/Outlook), classic stability | Long-tail Windows users | Last meaningful release years old; rough Mac support; basic editor |
| **Flameshot** | Linux + Mac + Win | Free OSS | Minimal in-overlay annotation, CLI-first, ImgUR upload | Linux devs | Annotation-only model; no library; no OCR |
| **Lightshot** | Win + Mac | Free (ads + prntscr.com share) | Single hotkey → annotate → "share link" workflow | Casual sharers | Privacy concerns (prntscr.com public), no library, no OCR |
| **Shottr** | Mac only | $8 Basic / $30 Pro one-time | Scrolling capture, OCR, QR-code reading, pixel ruler, "annoying fellow" nag instead of hard paywall | Mac users who want CleanShot-lite | Mac only |
| **Screenpresso** | Win | Freemium | Capture + scrolling + workflows + video | Win business users | Free tier limits, dated UI |
| **macOS native (⌘⇧5)** | Mac | Free | Region/window/full/video, "thumbnail markup" overlay | Everyone on Mac | No history, no OCR, no scrolling |
| **Win Snipping Tool** | Win | Free | Capture + delay + recent video recording + Win11 "Text Actions" OCR + "Image Eraser" + "Redact" | Everyone on Win | Limited history, no library, no scrolling, no smart organization |

### Clipboard

| Tool | Platform | Price | Killer features | Weakness |
|---|---|---|---|---|
| **Maccy** | Mac | Free OSS | Blazing-fast type-to-search popup, native UI, password-manager-aware | Mac only, text-focused |
| **Paste** | Mac + iOS + iPadOS | $29.99/yr (subscription) | iCloud sync across Apple devices, card UI, pinboards (now Shared Pinboards for teams), Power Search inside OCR'd images | Mac/iOS only, subscription |
| **Raycast** | Mac + Windows (beta 2026) | Free; Pro $10/mo | Clipboard inside launcher, AI commands, Cloud Sync across devices, growing Win parity | Account/cloud-leaning, launcher-shaped not popup-shaped |
| **Ditto** | Win only | Free OSS GPL | 500+ items, all clipboard types, group sync across machines on LAN, very stable | Win only, dated UI |
| **Win+V native** | Win 10/11 | Free | Built-in, optional cloud sync to MS account | Limited search, no rich types |
| **1Clipboard** | Win + Mac | Free | Google Drive sync, cross-platform | Stale, basic |
| **ClipboardFusion** | Win + Mac (some) | Freemium | Macros / scrubbing of HTML | Win-centric |

### Adjacent / video

| Tool | What it does | Why mention |
|---|---|---|
| **Loom** | Cloud-hosted video walkthroughs | Sets the bar for "share a video link to a process" |
| **Kap** | OSS Mac GIF/video recorder | Lightweight; reminds us video doesn't have to be Loom |
| **CleanShot Cloud** | Hosted share URLs | The link-share workflow that Cleanshot/Lightshot users miss without a server |

### Where snapper-keeper sits today

Currently shipped:
- Region/window/full/timed capture, annotation editor (arrow, rect, ellipse, pen, highlighter, text, blur, crop, numbered steps), OCR + FTS5 search, clipboard popup with caret anchoring + sensitive filter, tags, signed installers + Ed25519 auto-update, no telemetry/no servers/no accounts.

Already-shipped differentiators against the field:
1. **Combined capture + clipboard in one tray app** — nobody else does both well. Snagit/CleanShot don't do clipboard; Maccy/Paste/Ditto don't do capture. The OS natives (Win+V, ⌘⇧5) do both but separately and without OCR/library/popup-quality.
2. **OCR-indexed local search (FTS5)** — Snagit/Shottr have Grab Text on a single image; Paste has "search text in images." Nobody combines a captured-image library + clipboard history + OCR index in a single searchable surface.
3. **Caret-anchored clipboard popup** — Maccy and Paste anchor to active window/cursor only; caret-anchored is a measurable UX edge for prose-heavy use cases.
4. **Zero servers/accounts/telemetry** — versus Paste (iCloud), Raycast (Pro cloud sync), CleanShot Cloud, Loom. Real differentiator for privacy buyers.
5. **Ed25519-signed auto-update on a free side project** — most free OSS competitors lag updates badly (Greenshot, Ditto); most that update fast are paid.

Notable gaps vs. competitors:
- **No scrolling capture** — every paid Mac/Win tool ships this; only Greenshot's lone IE-style implementation among the free Win OSS.
- **No video / GIF** — Snagit, CleanShot, ShareX, Snipping Tool, Kap, Loom all ship this.
- **No share-link workflow** — CleanShot/Lightshot/Loom built whole identities around "share a URL."
- **No cross-device sync** — Paste, Raycast, Win+V, 1Clipboard, Ditto LAN all have something.
- **No Linux build** — Flameshot/Greenshot fans frequently want a polished cross-distro tool with library + OCR.
- **No EV cert / SmartScreen ramp on Windows** — most paid Win competitors have it; OSS doesn't.

## Position

Ranked top 10 candidates, highest overall value first. Complexity scoring uses higher=cheaper as instructed.

### 1. Scrolling capture (region-scroll, diff-stitch)
   Fit: 5/5  Reach: 5/5  Complexity (higher=cheaper): 2/5  Value: 5/5
   Competitors that ship it: Snagit, CleanShot X, Shottr, ShareX, Screenpresso, Greenshot (IE-only).
   Reasoning: **This is the single biggest gap in market positioning.** Every paid competitor across both OSes ships it. Reviewers anchor on it ("does it do scrolling?"). The diff-stitch approach (ShareX's algorithm — visible-region screenshot, programmatic scroll, image-similarity stitch) is cross-platform without OS-specific scroll injection; we already have xcap and a Konva canvas for stitch preview. Hits Snagit, CleanShot, Shottr, Screenpresso users in one shot. Engineering cost is real (scroll synthesis is finicky, sticky-header detection is hard, multi-monitor adds edge cases), but **this is the feature that turns snapper-keeper from "decent" to "I can finally switch from Snagit."** If we ship only one thing from this list, ship this.

### 2. Video / GIF recording (region + window, MP4 + GIF, no audio v1)
   Fit: 4/5  Reach: 5/5  Complexity (higher=cheaper): 1/5  Value: 4/5
   Competitors that ship it: Snagit, CleanShot X, ShareX, Kap, Loom, native Snipping Tool, native macOS ⌘⇧5.
   Reasoning: Largest market-expansion lever. "Capture tool without video recording in 2026" is a real friction point for buyers — every paid competitor has it and the Win11 Snipping Tool added it for free. FFmpeg sidecar (mirrors our tesseract pattern) is the cleanest path. Scope is genuinely large: encoder packaging on both OSes, cursor highlighting, file-size pragma, GIF re-encode pipeline. Defer audio capture to a v2 of the feature. **Real risk:** turns the app from "fast utility" into "video-and-capture suite" — affects identity. Lower complexity score reflects that this is probably 2× a single phase of work.

### 3. Local share-link workflow ("Copy share link" via self-hosted optional endpoint)
   Fit: 5/5  Reach: 4/5  Complexity (higher=cheaper): 4/5  Value: 4/5
   Competitors that ship it: CleanShot Cloud, Lightshot (prntscr.com), Loom, Imgur uploader (ShareX), Paste shared pinboards.
   Reasoning: A killer competitor workflow that we currently can't match. The privacy-correct version: **point-and-configure to a user's own S3/R2/Backblaze/private server** (think ShareX's SXCU). User pastes credentials once, gets `Copy share link` in the post-capture toolbar. Snapper-keeper still runs no servers, but unlocks the "send Slack a link, not a PNG" workflow. **Differentiation vs. CleanShot Cloud:** users own the bucket, no monthly fee, no third party reads images. **Vs. ShareX:** wraps the same idea in a polished consumer flow rather than an XML-config experience. Low-medium complexity because the bulk is just an upload abstraction.

### 4. Scrolling capture's natural sibling: Multi-image gallery / contact-sheet stitching
   Fit: 4/5  Reach: 3/5  Complexity (higher=cheaper): 4/5  Value: 4/5
   Competitors that ship it: Snagit ("create from templates" / Combine images), Shottr (composite), CleanShot (stacks).
   Reasoning: Pairs with scrolling but useful even without it: select 2-N captures in the library → produce a single stitched/sheet image with optional captions and step numbering. Hits the Snagit-templates use case (visual docs, tutorials, bug reports) without building Snagit's whole templates engine. Cheap-ish because the canvas (Konva) is already there; mostly compositing logic + a "multi-select → action" library UI flow. Doubles down on the **library + annotation** identity we already have.

### 5. Cross-device sync (clipboard + capture library) — opt-in, end-to-end-encrypted, peer-to-peer (Tailscale-style) or BYO storage
   Fit: 3/5  Reach: 5/5  Complexity (higher=cheaper): 1/5  Value: 3/5
   Competitors that ship it: Paste (iCloud), Raycast (Cloud), Win+V (MS), 1Clipboard (Google Drive), Ditto (LAN), Maccy (no).
   Reasoning: Huge competitive ask but **structurally conflicts with the no-servers stance.** Two compatible paths: (a) BYO sync folder (Syncthing / iCloud / Dropbox) with explicit "queue + merge" model that doesn't corrupt SQLite — likely a journal-style append log per device, periodically compacted; (b) peer-to-peer over Tailscale or direct LAN. Either way, real complexity: conflict resolution, encrypted-at-rest export, attachment movement. **Why I still rank it #5:** competitors are leaning HARD into multi-device. If we ignore it, "no sync" becomes the deal-breaker review point for the Paste/Raycast cohort. Mark this one as a v2 *unless* a smaller-scope LAN-only mode (Ditto-style) is viable.

### 6. macOS Vision OCR (replace Tesseract on Mac; keep Tesseract on Win)
   Fit: 5/5  Reach: 3/5  Complexity (higher=cheaper): 4/5  Value: 4/5
   Competitors that ship it: Shottr (Vision), CleanShot (Vision), Paste (Vision-indexed Power Search), Snagit (proprietary "Grab Text"), Win11 Snipping Tool (Text Actions).
   Reasoning: Vision is dramatically better than Tesseract on Mac (multilingual, handwriting, in-the-wild text). Tesseract being the OCR backbone is a known weakness when stacked next to Shottr/CleanShot/Paste on Mac. We can swap the OCR backend per-OS behind the existing `snk-ocr` plugin interface — fairly contained. **Doesn't broaden audience much** (we're not Mac-only) but **closes a quality gap reviewers will note** when comparing to Shottr/CleanShot.

### 7. Linux support (Tauri build + Wayland + X11 capture + tesseract bundle)
   Fit: 3/5  Reach: 4/5  Complexity (higher=cheaper): 1/5  Value: 3/5
   Competitors that ship it: Flameshot (full), Shutter, Ksnip, Gradia, Spectacle.
   Reasoning: Reach is high because **no competitor combines library + OCR + clipboard popup with quality on Linux.** Flameshot is annotation-only; Maccy is Mac-only; CopyQ exists but is utilitarian. We could own a niche. **Complexity is brutal:** xcap on Wayland is sketchy (portal-based capture), global shortcuts are unreliable on Wayland, tesseract sidecar packaging across distros needs work, no signing equivalent. Defer until phase 7 dust settles and a Linux-curious contributor shows up. **Underserved niche, but high cost.**

### 8. Plugin / extensibility API for end users (post-capture pipeline hooks + custom uploaders)
   Fit: 4/5  Reach: 3/5  Complexity (higher=cheaper): 3/5  Value: 3/5
   Competitors that ship it: ShareX (SXCU custom uploaders + after-capture workflows), partial Snagit (TechSmith integrations).
   Reasoning: **This is the ShareX killer feature** that turns power users into evangelists. Architecturally we already have plugin boundaries — exposing a small JSON or WASM hook surface for "after capture, run X" is plausible and *consistent with our identity*. Combine with item #3 (custom upload endpoints) and you cover 80% of why ShareX devotees stay on ShareX. Risk: every public hook surface becomes a maintenance contract.

### 9. EV code-signing cert (Windows SmartScreen ramp)
   Fit: 5/5  Reach: 3/5  Complexity (higher=cheaper): 5/5  Value: 4/5
   Competitors that ship it: Snagit, CleanShot, Screenpresso, every paid competitor.
   Reasoning: **Not a feature — a distribution fix.** First-time Win install today triggers SmartScreen ("Windows protected your PC"). Free competitors (ShareX) buried this with reputation over years; we can buy ~$400-600/yr to bypass the ramp. Strict business decision, almost zero engineering, big install-funnel win. Worth flagging here because the question is "next 10 things to do" and this materially affects every Windows download.

### 10. Capture templates / step-tutorial builder (multi-shot + step markers + captions → exportable doc)
   Fit: 4/5  Reach: 3/5  Complexity (higher=cheaper): 3/5  Value: 3/5
   Competitors that ship it: Snagit (templates — recently AI-assisted via Screentelligence), partial CleanShot.
   Reasoning: Builds on already-shipped step markers (decision #6 in the design log). The "I need to document this process" use case is real and Snagit owns it. Doesn't need AI in v1 — just a "compose multiple captures into one page with auto-numbered steps + captions" flow. Pairs with item #4 (multi-image stitching). Differentiates against every clipboard-only and capture-only competitor by leaning into the "produce shareable docs locally" lane.

## Alternatives Proposed

Considered but **didn't make the top 10**:

- **Crash report uploads (self-hosted Sentry)** — Rejected. Pure infra. No user-visible competitive value. Privacy posture in the design doc explicitly stays no-telemetry; reversing that is a brand cost. Implement only if real crash volume forces it.
- **App Store / Microsoft Store distribution** — Rejected for now. MS Store: marginal install gain, big sandboxing friction (global hotkeys, paste injection, accessibility prompts are all worse in store apps). Mac App Store: same plus the notarization vs. sandbox tradeoff hurts hotkey capture. **Audience B explicitly says no store.** Revisit only if SmartScreen friction (item #9) doesn't suffice.
- **Speech bubbles, spotlight/dim, magnifier loupe in annotator** — Cheap, but already in design doc as v1 non-goals and not a competitive blocker.
- **Snagit-style "smart move" / object replacement (Screentelligence)** — Trend-following AI. High complexity; unclear durable value; conflicts with no-telemetry brand if it phones home.
- **QR-code reading in OCR pipeline** — Tiny feature; Shottr ships it; can fold into item #6.
- **Self-destructing share links** — Subset of item #3.
- **Pin to top / screen freeze** — CleanShot loves it; mid-tier value; could fold into a "freeze and annotate" mode without ranking it standalone.
- **Auto-blur of detected text (PII redaction)** — Trend-following, Win11 Snipping Tool added it. Useful but narrow audience; defer.
- **Window-region "smart-snap" auto-detection at capture time** — Already partly addressed in deferred preview-cache work; doesn't move the needle competitively.

Runner-up: **iOS/Android companion app for clipboard sync only** — extends Paste's iCloud advantage but builds a whole new platform. Don't ship in this 10.

## Risks Identified

1. **Competitive parity is a trap.** Chasing Snagit feature-for-feature turns the app into a worse Snagit. The differentiated path is "Snagit-quality core + combined clipboard + privacy + free." Items #1, #4, #10 lean *toward* parity; #3, #5 (BYO), #8 lean *toward* differentiation. Pick the mix deliberately.
2. **Video recording (#2) changes the product identity.** Suddenly we're competing with Loom and Kap, not Snagit and Shottr. Memory footprint goes up. The "lightweight tray app" pitch weakens. May warrant a separate window/process or a "video mode" toggle to keep the surface bounded.
3. **Cross-device sync (#5) is the cliff.** Done wrong it corrupts SQLite, leaks E2EE keys, or pushes us into running a relay. Done right it requires a real protocol + careful test matrix. Easiest to start with explicit **export/import per-machine** before claiming "sync."
4. **Plugin API (#8) is a forever maintenance contract.** Stable APIs are hard. Bad APIs become technical debt. Even ShareX's SXCU shows its age. Scope the v1 to a narrow hook surface (e.g., post-capture + share-target only) and version it explicitly.
5. **EV cert (#9) needs a legal/identity decision.** Side projects often run as personal identity which makes EV certs personally identifiable. Decide cert holder *before* purchasing.
6. **macOS Vision (#6) creates per-OS quality divergence.** Search results will differ between Win and Mac libraries. Document the divergence; surface backend in settings.

## Open Questions

1. **What does "share-friendly side project" cost ceiling look like?** EV certs ($400-600/yr), Apple Developer ($99/yr — already paid?), optional infra for share-link relays — does Eric want any recurring cost, or strictly zero?
2. **Is there real install/telemetry data to anchor on, or are we flying blind on user count?** Without that, prioritization between "broaden the funnel" (video, scrolling) and "deepen loyalty" (plugins, sync) is a guess.
3. **Linux yes/no as a north-star** — does Eric want this to be a Win+Mac product, or eventually all three? Affects every architecture choice (Vision OCR, EV cert vs. Linux signing, custom uploaders portability).
4. **Is the user willing to host any service?** Item #3 (share links) is most powerful if the user can run a tiny relay; otherwise we're stuck with BYO S3. Knowing the answer changes the design.
5. **Do power users in Eric's circle actually ask for scrolling?** If yes, #1 jumps to a near-certain ship. If no, video (#2) may be the more universal pull.
6. **How important is "no servers" as identity vs. as policy?** If it's hard policy, items #3 and #5 must be BYO-only. If it's a current preference, optional managed-cloud sub could unlock a price-tier later (would be a different audience B+).
