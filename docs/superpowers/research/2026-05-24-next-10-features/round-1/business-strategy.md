# Round 1 — Business / Strategy perspective

**Date:** 2026-05-24
**Lens:** Resource cost, opportunity cost, strategic fit, product identity coherence

## Framing assumptions (Business lens)

Before scoring, I'm anchoring on five facts that drive every decision below:

1. **One-person, side-project economics.** Every feature creates a perpetual maintenance tax — bugs, OS-version breakage, dependency churn. The marginal cost of feature #11 is not zero; it's the cumulative drag on shipping #12. Eric is the entire org chart.
2. **Audience B is "share-friendly side project," not "monetize."** No paid tier, no servers, no telemetry. That means every feature has to pay back in *adoption / personal-utility / portfolio-credibility*, never revenue. Features that only matter for a paying enterprise tier are negative-value here.
3. **Product identity is already coherent and rare.** Capture + clipboard + OCR + annotate + caret-anchored auto-paste, local-first, no account — this is a positioning *moat*. Snagit costs $63/yr and is closed; ShareX is Windows-only; Greenshot is dated; CleanShot is Mac-only and paid; macOS Shottr is paid. The next features must *compound* this identity, not blur it into "yet another productivity suite."
4. **There is no user signal yet.** Phase 7 just shipped signed installers. There are no real users, no GitHub issues from strangers, no analytics (by design). Spending heavy on speculative features is high-risk; cheap features that broaden the funnel are strictly better until signal arrives.
5. **The architecture rewards small additive plugins; it punishes anything that breaks the "snk-library owns persistence" rule or introduces servers.** Cloud sync and synced clipboard, in particular, are *category-breaking* from the architecture's point of view.

These assumptions are why my ranking diverges sharply from the seed list's implicit order.

## Position

Top 10, highest overall value first:

```
1. EV code-signing cert (Windows SmartScreen)
   Fit: 5/5   Reach: 5/5   Complexity (higher=cheaper): 5/5   Value: 5/5
   Reasoning: Phase 7 ships signed installers, but standard OV certs require ~3000 installs before
   SmartScreen stops warning. Every "Unknown publisher / Run anyway" prompt kills funnel — for a
   side project with zero brand recognition, this is THE adoption bottleneck on Windows. EV cert
   gets you reputation from install #1. Cost: $300-$600/yr + a USB HSM dance. Zero code complexity
   (Phase 7 already plumbs the sign step). Highest ROI on the list by a wide margin: it makes
   every OTHER feature actually reach users.

2. Scrolling capture
   Fit: 5/5   Reach: 4/5   Complexity (higher=cheaper): 2/5   Value: 5/5
   Reasoning: The #1 feature competitor users name when they justify paying $63 for Snagit. It's
   the most-requested capture feature anywhere in this category. Expensive (per-OS stitching,
   UIA / Accessibility scrolling, image alignment heuristics) but pays back directly in product
   identity: snapper-keeper becomes "Snagit but free + local." Without it the product is "yet
   another screenshot tool." Single biggest competitive gap. Worth eating the complexity.

3. macOS Vision OCR (replace/augment Tesseract)
   Fit: 5/5   Reach: 4/5   Complexity (higher=cheaper): 4/5   Value: 4/5
   Reasoning: Apple's Vision framework is dramatically better than Tesseract on screenshots
   (mixed fonts, low-res text, handwriting) AND ships with the OS — no sidecar to bundle, no
   tessdata. Wins twice: (a) materially better OCR accuracy = better search = the whole product
   feels smarter; (b) reduces app footprint and installer size on Mac. snk-ocr already has a
   queue-and-backend abstraction; a `MacOcrBackend` is a contained add. Tesseract stays as the
   cross-platform fallback. Compounds the "local-first feels magic" identity.

4. Windows Snipping Tool-style "Text Actions" / OCR overlay
   Fit: 5/5   Reach: 5/5   Complexity (higher=cheaper): 4/5   Value: 4/5
   Reasoning: NEW candidate, not on the seed list. After a capture, expose OCR text as a
   selectable overlay on the image, plus a "Copy text" button on the post-capture toolbar.
   "Capture a screenshot, get the text out" is the single most viral feature in PowerToys Text
   Extractor / macOS Live Text. We're already paying for OCR in phase 5 — we already have the
   text. Surfacing it costs <1 week, mostly frontend. This turns OCR from "search-only utility"
   into "headline feature people demo to friends." Massive marketing value, near-zero cost.

5. GIF recording (defer video, ship GIF only)
   Fit: 4/5   Reach: 5/5   Complexity (higher=cheaper): 2/5   Value: 4/5
   Reasoning: Splitting the seed list's "Video/GIF recording" into halves: GIF alone is the
   high-value piece. GIFs are how people share bug repros and "look what I did" — they
   auto-play in every Slack/Discord/issue tracker. Tauri + xcap can frame-grab; encoding can use
   gifski via sidecar (mature, ships single binary per OS, no FFmpeg legal/distribution mess).
   Skip MP4/H264 for now — that's where licensing, audio mixing, and encoder complexity explode.
   Reach-multiplier: every shared GIF is silent marketing for snapper-keeper.

6. Synced clipboard between devices (LAN-only, no cloud)
   Fit: 4/5   Reach: 4/5   Complexity (higher=cheaper): 2/5   Value: 4/5
   Reasoning: Reinterpreting the seed candidate. The seed framed it as cloud-synced — that's
   off-strategy (servers, accounts, telemetry). LAN-only via mDNS + libp2p/QUIC with a paired-
   device handshake keeps zero-server posture intact and matches LocalSend / KDE Connect, which
   are genuinely beloved. It's the rare feature that simultaneously: (a) hits a real cross-device
   pain, (b) reinforces "no cloud, no account" identity rather than diluting it, (c) acts as a
   compounder on the clipboard plugin we already shipped. Hard to get right (NAT/firewall on
   Windows, pairing UX), but uniquely defensible.

7. Plugin / extensibility API (read-only, JS sandbox)
   Fit: 4/5   Reach: 3/5   Complexity (higher=cheaper): 3/5   Value: 4/5
   Reasoning: Reframed as scoped: NOT full Rust plugin loading (that's a security and review
   nightmare for a side project). Instead a *JS extension* surface inside the existing webview,
   sandboxed, read-only access to captures/clipboard + ability to register post-capture toolbar
   actions ("Send to Confluence", "Upload to S3", "Run through ChatGPT vision"). This turns
   power users into unpaid contributors and lets snapper-keeper plug into the LLM ecosystem
   without us writing any of those integrations. Reach is 3 because most users won't install
   extensions, but the *narrative* reach for HN/Reddit is high.

8. Self-hosted crash report uploads (opt-in Sentry)
   Fit: 4/5   Reach: 2/5   Complexity (higher=cheaper): 4/5   Value: 3/5
   Reasoning: Strategic, not user-visible. Today, when a user hits a crash, Eric never finds out
   unless they file an issue. With zero telemetry and zero users-known, the project is flying
   blind. Self-hosted Sentry (or sentry.io free tier under @hartye.com) with explicit opt-in
   during first-run wizard keeps the privacy posture intact while unblocking real debugging.
   Opportunity cost of NOT having this scales linearly with user count; it's cheap insurance
   to add now BEFORE the user count grows. Defensive infra, not a feature.

9. Microsoft Store distribution
   Fit: 3/5   Reach: 4/5   Complexity (higher=cheaper): 4/5   Value: 3/5
   Reasoning: Tauri + MSIX is well-trodden. Microsoft Store on Windows 11 gives: (a) bypass of
   SmartScreen entirely, (b) auto-update via Store (parallel to our Tauri updater), (c) a
   trusted-source listing that legitimizes the product. Free to publish. Mac App Store is
   meaningfully harder (notarization is already done but Store requires sandboxing that breaks
   capture entitlements + auto-paste — fundamentally incompatible with how the app works). So
   list MS Store only, skip Mac App Store. Listed at #9 because EV cert (#1) solves most of
   the same SmartScreen problem at lower complexity.

10. Linux support (Wayland-first)
    Fit: 3/5   Reach: 3/5   Complexity (higher=cheaper): 2/5   Value: 3/5
    Reasoning: Linux desktop is small but disproportionately influential for word-of-mouth in
    the kind of HN/r/programming audience that would notice and share a local-first Rust
    productivity tool. xcap supports Linux; arboard supports Linux; the biggest cost is the
    Wayland fragmentation tax (portal protocols for screen capture, no global hotkeys on
    Wayland-pure compositors) plus tesseract packaging across distros. Listed last because the
    audience is small AND it's an open-ended quality commitment — once you ship Linux you're
    on the hook for distro-specific bug reports forever.
```

## Alternatives Proposed

### Considered and rejected from the seed list

- **Cloud sync (as originally framed, with servers).** Rejected. Violates audience B posture
  (no servers, no accounts). Would force a backend, a billing decision, GDPR/data-residency
  questions, and on-call. The reframed LAN-only version (#6) captures the use case without
  the infra. The seed candidate as written is a *product identity fragmenter* and should be
  explicitly killed in the decisions log, not just deferred.

- **App Store distribution on macOS.** Rejected separately from MS Store. App Store sandbox
  requirements are fundamentally hostile to clipboard auto-paste and global capture hotkeys —
  shipping there would mean shipping a crippled product under the same name, which is worse
  for brand than not shipping there at all.

- **Original "Video / GIF recording" combined candidate.** Split — GIF in (#5), video out.
  Video adds FFmpeg licensing/distribution, audio capture entitlements, encoder tuning, and a
  whole class of "why is my file 800MB" issues. Snagit charges money for it because it's
  expensive to support. Side project should not absorb that maintenance load yet.

- **Synced clipboard between devices (as cloud-relay).** Rejected in original framing; kept
  as LAN-only (#6).

### Runner-up candidates (11–15)

These didn't make the top 10 but deserve mention:

11. **Smart-section/saved-search rules** — "auto-tag everything from Figma," "screenshots
    older than 30 days move to archive." Compounds the library investment from phase 6 at
    near-zero cost. Replaces #10 if the team decides Linux is too speculative.

12. **Cloud-folder "Bring Your Own Storage" mode (Dropbox / iCloud)** — explicitly *unsupported*
    today per design doc, but documenting a "single-machine, point library at synced folder"
    config would be cheap and lets users self-serve the cross-device problem the seed
    "cloud sync" candidate is really about.

13. **Selective area OCR (Cmd+Shift+T → OCR a region, no save)** — a one-keystroke "extract
    text from anywhere on screen" mode that doesn't even create a capture row. Tightly scoped,
    plays into the Text Actions angle (#4). Could absorb #4 if budget tight.

14. **In-app keyboard-shortcut editor / cheat sheet** — currently the hotkey UX is bare per
    phase 6. Reduces support burden, improves first-run conversion. Cheap.

15. **Per-monitor color-picker / measurement tool on capture overlay** — Snagit/CleanShot have
    these as freebies. Modest reach but cheap.

### Explicitly dropped

- **Speech bubbles / spotlight-dim / loupe / rotate** — design doc already calls these out as
  non-goals. No business case to revisit.
- **License gating, paid tiers, accounts** — explicitly off-strategy.
- **Mobile companion app** — completely off-strategy; not on the seed list, mentioning only
  to pre-empt scope creep.

## Risks Identified

1. **The seed list is an architecture-shaped wish list, not a user-signal list.** It was
   compiled from "things we deferred while building v1," which biases toward "things engineers
   thought about" rather than "things users will care about." We have zero ground truth from
   actual users yet. Any ranking is fundamentally speculation — the highest-confidence move is
   actually #1 (EV cert) and #8 (Sentry) because they're the things that let us *get* user
   signal cheaply.

2. **Identity drift toward "do-everything productivity suite."** Cloud sync, plugin API,
   extensibility, multi-device — each individually plausible, in aggregate the product loses
   its sharp positioning ("local-first capture + clipboard, no account"). Pick *one* heavy
   identity-expanding feature and one infra-defensive feature per release, not three identity
   bets at once.

3. **Per-OS maintenance cost compounds non-linearly.** Adding Linux today means every future
   feature has to consider Linux. Adding scrolling capture means OS-specific scroll APIs that
   must be maintained as Windows/Mac APIs evolve. The "free maintenance budget" of a one-person
   side project is finite and currently spent on the v1 surface. Each new identity-expanding
   feature implicitly eats budget for *all* future bugfixes.

4. **EV cert recurring cost and HSM-binding** is a real ongoing tax — $300-$600/year,
   physical hardware, the cert is bound to that hardware. If Eric stops paying, every existing
   user's auto-updater chain breaks on next signing (mismatched publisher). This is the only
   "feature" on the list with a contract attached. Worth doing, but go in eyes-open.

5. **GIF recording is a slippery slope to video.** Once it ships, the #1 issue request will be
   "add MP4 export" and "add system audio capture." Need explicit messaging that GIF is
   intentional scope, not a stepping stone — otherwise the maintenance load expands.

6. **Plugin API is a security perimeter expansion** even sandboxed. Every plugin install is a
   potential malware vector or data-exfiltration path. For a privacy-first product, this has
   to be done very carefully or it undermines the entire pitch. If we're not confident in the
   sandboxing review, drop #7 in favor of #11 (smart-sections) and ship something safer.

7. **Microsoft Store + EV cert are partially redundant.** Either solves SmartScreen. Doing
   both is fine but the second one's marginal value is lower than its scoring suggests —
   should think of #1 and #9 as a bundle and pick.

## Open Questions

1. **What is Eric's actual time budget per week for phase 8+?** "Top 10" sized for 2 hrs/week
   looks completely different from 20 hrs/week. The EV cert + Sentry combo is the right answer
   if the time budget is small. Scrolling capture + Vision OCR + LAN sync is the right answer
   if it's large.

2. **Is there appetite to add ANY recurring cost?** EV cert is $300-$600/yr and the only
   item on my list with a vendor relationship. Without it, the whole adoption-funnel argument
   for SmartScreen evaporates and Microsoft Store (#9) becomes much higher priority.

3. **Should we instrument *anything* for adoption signal?** Even opt-in "phone home on first
   launch with version + OS" would tell us: (a) installs, (b) Windows/Mac split, (c) update
   reach. Currently phase 7's "GitHub release download count" is our only proxy, and it
   doesn't tell us if installs convert to long-term usage. This is upstream of every prioritization
   question.

4. **What's the open-source / portfolio outcome we want?** If this is a portfolio piece for
   future hiring, "elegant local-first architecture with scrolling capture and LAN sync" wins.
   If it's a daily-driver tool for Eric and a few friends, smart-sections + Sentry + EV cert
   win. These two paths diverge in feature ordering after item #3.

5. **Is the audience really only Windows + Mac, or is Linux a deferred-but-real target?**
   This determines whether Linux (#10) is a v2 thing or a never-thing. Should be a written
   decision, not left ambient.

6. **For LAN sync (#6), what's the threat model on a coffee-shop Wi-Fi?** mDNS broadcasts
   device names and presence. If the answer is "users should only use sync on trusted
   networks," that's a UX boundary the design has to handle from day one, not patched later.

7. **For #4 (Text Actions overlay), do we want it on captures only, or also on the live
   screen (e.g., Cmd+Shift+T to OCR-grab any text without creating a capture)?** The second
   is more useful but doubles the implementation. Could ship the first and let the second
   land as #13 if signal warrants.
