# Round 2 — Business/Strategy cross-pollination

**Date:** 2026-05-24
**Round 1 stance:** LOCKED. This document only adds reactions, tensions, and new insights triggered by reading User/Consumer, Maintainer, and Competitive Landscape.

## Reactions

### To User/Consumer

- **#2 PII auto-redact (built on existing OCR)** is the single most under-rated finding in the
  whole research package, and I missed it. From a business lens it's a *brand-defining moment*
  feature: it's the kind of capability that ends up in every "snapper-keeper does X that
  Snagit doesn't even attempt" comparison post, and it costs almost nothing because we already
  paid for OCR + bounding boxes + the blur tool. This is precisely the compounding-existing-
  investment move my Round 1 only halfway recognized (I put Text Actions overlay at #4 for the
  same architectural reason but missed the bigger privacy-angle version). Strategic gold.
- **#4 Paste as plain text / paste as image** is the cheapest legitimate "feature" on any of
  our four lists. As a business unit, near-zero cost moves like this should always ship
  *between* the bigger phase efforts to keep velocity-perceived-by-users high — a kind of
  marketing-as-changelog tactic. My Round 1 didn't account for these velocity-fillers.
- **#10 LAN ephemeral QR share** is a more elegant framing of the same "no-cloud sync use
  case" that drove my Round 1 #6. The QR-code/HTTPS-localhost angle solves the *cross-device
  pairing UX* that I waved at — pairing is the hardest part of LAN sync, and a QR + HTTPS-
  with-self-signed-cert flow finesses it for the most common journey (phone receives one
  image). I should have specified this UX path. User-Consumer is right that this is the
  pragmatic ramp before full LAN clipboard sync.
- The personas exercise is more disciplined than my "audience B" abstraction. From a business
  view, persona #5 (teacher/streamer) is the persona most likely to actually evangelize the
  product publicly via classroom demos and stream chat — *streamer reach matters dispropor-
  tionately for word-of-mouth in this category.* That edges video/GIF up toward where
  Competitive Landscape put it.

### To Maintainer

- We **agree on #1 (EV cert)** and **agree on Vision OCR**. Two of four perspectives
  converging gives this very high confidence; suggest both rank in the synthesis top 3.
- Maintainer's reframing of crash reporting as **"one-click GH-issue funnel"** is correct and
  better than my "self-hosted Sentry" framing. Same strategic outcome (Eric learns about
  crashes), but: zero infra cost, zero recurring spend, user-consent baked in by the act of
  filing the issue, and *moves crash reports into the public artifact stream* (issues), which
  helps build the product's GitHub presence — which itself is marketing. I'm taking this
  upgrade; my #8 should be read as Maintainer's "GH-issue funnel" form, not Sentry.
- **CLI / URL-scheme automation hooks (Maintainer #8)** is a candidate I missed and from a
  business lens it's underrated. "Snapper-keeper works with Raycast / Alfred / Stream Deck /
  Hammerspoon / Shortcuts.app" via deep links is the *cheapest way to land in 5 review/
  comparison posts at once* — every one of those tools has a community that writes "best
  X integrations" listicles. Cost is one small plugin; reach is multiplied by 5 other tools'
  audiences for free. This belongs above my #7 plugin-API candidate.
- **#9 iCloud / OneDrive backup-only mode** is a great compromise I underweighted. From a
  business angle it neutralizes "I lost my screenshots when my laptop died" as a *negative
  review point* without us touching servers. Pure defensive play. Belongs in top 10 over
  some of my speculative items.
- Maintainer's honest **6-10 weeks estimate for video** is sobering. My Round 1 split out GIF
  as the cheap half, but Maintainer's framing makes me realize *even GIF-only* would chew
  the same encoder/capture-loop/region-overlay rework. So GIF isn't actually as cheap as I
  scored it. Adjusting my mental confidence on candidate #5 downward without rewriting Round 1.

### To Competitive Landscape

- **#3 Local share-link via BYO S3/R2/Backblaze** is the candidate I most regret missing. From
  a business perspective: this is the "buy a comparable feature to CleanShot Cloud at $0/mo"
  pitch that lights up exactly the niche we should own — privacy-conscious devs/designers who
  have AWS or Cloudflare accounts already. The killer point Competitive Landscape makes:
  **"snapper-keeper still runs no servers, but unlocks the send-Slack-a-link workflow."** That
  preserves brand posture *and* steals the most viral CleanShot workflow. Higher business
  value than my Microsoft Store candidate. I want to formally signal this should bump into
  any synthesis top 10.
- The **Snagit pricing anchor ($63 perpetual + paid upgrades) and CleanShot ($29 one-time +
  $8/user/mo cloud)** are great market intel I should have surfaced. This frames the
  "free, local-first, signed installer" pitch as having a real dollar-value-saved story for
  every download — meaningful for HN/Reddit headlines.
- Competitive Landscape's **#10 capture templates / step-tutorial builder** is the one
  candidate I considered but dropped from my runner-ups. Reading the framing as "Snagit owns
  the 'produce shareable docs' lane," I see the strategic angle better: combining already-
  shipped step markers + already-shipped tags + a multi-select compose flow is *cheap and
  identity-amplifying*. Snagit's templates are AI-assisted now; ours can be plain-and-fast,
  which is its own positioning. Worth promoting from my runner-ups.
- Competitive's **caveat that video changes the product identity** ("competing with Loom and
  Kap, not Snagit") aligns with my Risk #2 ("identity drift") and reinforces it. Two
  perspectives independently flag this — it should be a hard gate in the synthesis discussion.

## Tensions

### Tension 1: Where does scrolling capture rank?

- **My Round 1:** #2 (high value, complexity-expensive but identity-defining).
- **User/Consumer:** #1 (highest value).
- **Maintainer:** #3 (real but bounded engineering cost, 3-4 weeks).
- **Competitive:** #1 ("ship only one thing, ship this").

Three of four perspectives put it at #1. I put it at #2 only because I weighted EV cert as a
*funnel-multiplier* on top of every other feature. From a strict business-value lens, the
question is: does the EV cert genuinely block all the other features from reaching users, or
is SmartScreen friction overstated? If the answer is "users push through SmartScreen for a
genuinely-better tool," then scrolling capture should be #1 and EV cert should be #2 or #3.
I won't revise my locked position but I want to note: **the consensus across the other three
perspectives is so strong on scrolling capture that the synthesis should treat my #2 ranking
as a minority view, not the centroid.**

### Tension 2: Crash reporting — Sentry vs. GH-issue funnel

- **My Round 1:** Self-hosted Sentry (#8), opt-in, framed as defensive infra.
- **Maintainer:** "GH-issue funnel" — one-click button opens a pre-filled GH issue in the
  browser, zero network code in the app, zero infra.
- **User/Consumer:** Reject entirely; privacy posture, no real installs yet.

Maintainer's path is *strictly better* than my Sentry path on every business axis (cost: $0
vs. $20-50/mo; infra: none vs. VPS+TLS; privacy: explicit user click vs. opt-in checkbox;
marketing: public issue stream vs. private dashboard). I concede the framing; the candidate
should survive into synthesis but as Maintainer's form, not mine.

User/Consumer's "reject entirely" is the wrong call here — without *some* crash visibility,
the product flies blind forever. But their concern (privacy) is addressed by the GH-funnel
form, so it's a tension that resolves cleanly if we adopt Maintainer's variant.

### Tension 3: Cross-device sync framing

- **My Round 1:** LAN-only synced clipboard via mDNS + libp2p (#6).
- **User/Consumer:** LAN ephemeral one-shot QR share (#10) — different problem; no
  persistent sync, just ad-hoc "send this image to my phone."
- **Maintainer:** Full LAN clipboard sync with QR-pairing (#4).
- **Competitive:** Cross-device sync at #5 but flags it as the "cliff" with risk of SQLite
  corruption; suggests BYO sync folder as a safer ramp.

There are actually **three distinct features** hiding under "sync":
1. Ad-hoc share (QR / one-shot link) — User/Consumer's framing
2. Persistent LAN clipboard mirror — Maintainer's + my framing
3. Async file backup to user-owned bucket — Competitive's BYO version, Maintainer's iCloud/
   OneDrive backup-only

Business view: **#1 (ad-hoc) is the cheapest and lowest-risk and probably ships first**;
#3 (backup-only) is the second cheapest and addresses the *most common real complaint*
("don't lose my screenshots"); #2 (persistent sync) is the most ambitious and should only
ship after we have user signal that it's actually the missing thing. The synthesis should
*separate these candidates rather than treat them as one*. My Round 1 conflated #1 and #2;
correcting that in cross-pollination.

### Tension 4: Plugin / extensibility surface

- **My Round 1:** Sandboxed JS extension surface (#7).
- **Maintainer:** Hard reject. Use deep-link URL scheme (#8) instead — gets 80% of the
  value without the maintenance contract.
- **User/Consumer:** Defer indefinitely; users would rather have scrolling.
- **Competitive:** Plugin API at #8 as the "ShareX killer feature."

Three of four lean toward "don't build a plugin API; build a URL scheme + BYO uploaders
that lets external automation tools drive snapper-keeper instead." This is strictly cheaper,
preserves identity ("snapper-keeper does one thing well; here's the API for tools that
already exist to drive it"), and **avoids the forever-API-compat tax** Maintainer flagged.
Concede the architecture move. The deep-link approach beats my "JS extension" approach on
maintenance economics by orders of magnitude.

### Tension 5: Microsoft Store distribution

- **My Round 1:** #9 (legitimacy + SmartScreen bypass).
- **Maintainer:** Hard reject (sandboxing + ongoing review SLA + parallel signing identity).
- **User/Consumer:** "Actively destructive to the product" because of sandbox + accessibility
  conflicts.
- **Competitive:** Out of audience B; revisit only if EV cert doesn't suffice.

This is the candidate where I'm most outnumbered. Three perspectives independently say the
sandbox model breaks the product's core flows (global hotkeys, accessibility auto-paste).
**They are correct on the technical merits; I underweighted the sandbox compat problem.**
Business view post-cross-pollination: Microsoft Store should drop out of the top 10 and EV
cert alone carries the "Windows install legitimacy" weight. My Round 1 ranking of #9 is the
weakest entry; it should not survive into the synthesis.

## New Insights

### Insight 1: "Cheap velocity-filler" features deserve their own category

User/Consumer's #4 (paste as plain text), Maintainer's #7 (GH-issue funnel), and my eventual
realization about quick wins point to a class of features that none of us scored highest but
collectively are the most efficient use of maintainer hours. The synthesis should probably
treat these *not as competitors with scrolling capture* but as **filler shipped between
larger features** to keep momentum visible. The right model isn't "rank top 10 in priority"
— it's "two big phases (scrolling, Vision OCR), one medium phase (LAN ad-hoc share or
BYO upload), and four cheap polish items spread between."

### Insight 2: The OCR investment is wildly under-leveraged

Both User/Consumer (PII auto-redact) and Competitive (Win11 Snipping Tool's Text Actions) and
my own #4 (Text Actions overlay) all surface that **the phase-5 OCR work has at least three
distinct user-facing features still untapped**: redact suggestions, post-capture text-selection
overlay, and search-text-on-web from a captured region. From a business standpoint, OCR has
the **highest ROI of any phase already shipped** because every additional surfacing of the
OCR data costs days, not weeks, and each one is independently compelling. A *single phase*
called "OCR surfaces" could ship 3 candidates from this list at once. That changes the
budget math.

### Insight 3: "Audience B" is doing too much work in our scoring

All four perspectives invoked audience B / "share-friendly side project" as a constraint, but
*we never aligned on what audience B's threshold for adoption-marketing-effort is.* Some of
us (mine, Competitive) treat installs/funnel-conversion as a real metric to optimize against.
Others (User/Consumer's risk #1, Maintainer's open question #5) treat it as a "personal
utility + a few friends" project where install count doesn't matter much. **This is the
single biggest uncertainty driving prioritization disagreement.** The synthesis cannot
produce a single confident ranking without resolving it. From a business lens, this is the
*decision that should precede the feature decisions*, not be made implicitly inside them.

### Insight 4: BYO storage is a posture, not just a feature

Competitive's #3 (BYO uploaders) plus Maintainer's #9 (backup-only mode) plus my LAN sync
all share a common pattern: **the user supplies the infrastructure**, snapper-keeper supplies
the integration. This is more than a coincidence — it's a coherent product posture that
could be marketed as a *category*: "Your tools, your storage, your network. We just connect
them." That's a sharp differentiator against Snagit/CleanShot/Paste (their cloud, their
storage) and even against ShareX (theirs is technically BYO but the UX assumes you're a
sysadmin). If the synthesis adopts "BYO" as an explicit strategic theme, it ties together
3-4 candidates into one identity-amplifying phase.

### Insight 5: There is a hidden complexity multiplier I missed: signing every new sidecar

Maintainer's "every new sidecar binary adds another way for the release pipeline to break
silently" point reframes feature cost. Anything that ships a new bundled binary (FFmpeg for
video/GIF, additional Tesseract data, gifski, etc.) **pays the phase-7 signing tax in full
again per platform.** From a business perspective, this means:

- Vision OCR (#2 in my ranking) is even better than I scored it because it *removes* a
  sidecar from the Mac bundle.
- GIF recording (my #5) is even worse than I scored it because gifski needs signing per
  platform and the bundle grows ~30 MB.
- BYO share-link (Competitive #3) is even better than I scored it because it ships no new
  sidecars at all — just network code.

The signing tax is the hidden line item in every feature's cost. It should be an explicit
score modifier in the synthesis: **"how many new binary artifacts does this ship?"**

### Insight 6: We collectively over-rank Linux

All four perspectives ranked Linux somewhere in 6–10, but each of us also flagged Wayland
as a UX-breaking concern, ongoing distro packaging tax, and tiny-audience problem.
The fact that everyone independently said "low-confidence inclusion" suggests **Linux should
probably be cut from the top 10 entirely** and reframed as a "compile-only CI job + accept
contributions" footnote. Business view: keeping it in the top 10 risks consuming a phase
that returns less value than 2-3 cheap polish items would. I want to put this on record as
the candidate the synthesis should be most willing to drop.

### Insight 7: The synthesis should produce a *phased* recommendation, not a flat ranking

After reading all three perspectives, the *right* artifact isn't a single ranked list — it's
a recommended **sequencing plan** that says:

- **Phase 8 (defensive, ~2 weeks):** EV cert + GH-issue funnel + paste-as-plain-text.
  Lowest cost, highest funnel impact, ships fast.
- **Phase 9 (identity-defining, ~4 weeks):** Scrolling capture. The competitive gap-closer.
- **Phase 10 (compounding existing investment, ~3 weeks):** OCR surfaces — PII redaction
  suggestions + Text Actions overlay + Vision OCR backend on Mac.
- **Phase 11 (BYO posture, ~3 weeks):** Local share-link via BYO uploaders (S3/R2) +
  backup-only mode to user-selected folder + LAN ad-hoc QR share.
- **Phase 12 (integrations, ~2 weeks):** Deep-link URL scheme + eyedropper/pixel-measure
  annotation tools + drag-out from library.

This sequencing implicitly drops Microsoft Store, Linux, full LAN clipboard sync, generic
plugin API, video recording, and self-hosted Sentry — all of which were on at least one
perspective's top 10 but lose under cross-perspective scrutiny. Stating that explicitly is
more useful than a single ranked list.
