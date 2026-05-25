# Round 2 — User/Consumer cross-pollination

**Author:** User/Consumer
**Date:** 2026-05-24
**Round 1 position:** LOCKED — reactions below do not retract or soften it.

## Reactions

### To Business/Strategy

- **EV cert at #1 (Business) vs. not in my top 10 at all.** Strong observation that I missed: phase 7 ships *signed* installers but they're not *reputation-trusted* until 3000-ish installs land. Every user-journey I walked assumed "user already has the app open." Business is right that there's a step zero — "user attempts to install and clicks past a SmartScreen warning" — that I treated as solved. I still wouldn't put it #1 (it's a procurement task, not a feature), but I now see it should appear in my list as a blocker on Journey 0 (first install). If I were rewriting Round 1, EV cert displaces my #10 (LAN ephemeral share) and goes ~#5-#6 — because no Windows journey works at all if the user bails at SmartScreen.
- **"Text Actions" / OCR overlay on captures (Business #4).** Strong agreement. This is the polish that turns OCR from "background search infrastructure" to "I can demo this in 5 seconds." We already pay the OCR cost; surfacing the text on the image is a frontend addition. I implicitly assumed users would search through the library; Business is right that users would *grab text from this one capture* a lot more often than that. Reach for this is higher than my own #6 (Vision OCR backend swap) because it's user-visible on day one, not a quality lift months in.
- **"Identity drift toward do-everything productivity suite" risk.** This re-frames my #5 (Quick-share targets), #7 (Video/GIF), and #8 (Drag-out) as identity-blurring even if individually useful. Business is right: pick *one* heavy identity-expanding feature and *one* infra-defensive feature per release. I had three identity bets in my top 10 (#1 scrolling, #2 redact, #7 video) and didn't weight that cumulative effect.
- **"There is no user signal yet" framing.** Business names what was implicit in my Round 1 Open Question #1 — every "user wants X" claim from any of us is speculation. Business's response (ship cheap/wide things first) is the strategic answer; mine (instrument quietly) would conflict with the privacy posture. Business wins this one — defensive cheap features beat speculative expensive ones in the absence of signal.

### To Maintainer

- **Calibration anchors are gold.** Phase 5 = 3 weeks for OCR; phase 7 = "1220 lines planned, 5 follow-up commits to actually work." That's the data my complexity scores lacked. Re-reading my own ranking: my "complexity 2/5" for scrolling capture (#1) is probably right by Maintainer's metric (3-4 weeks); my "complexity 4/5" for PII redaction (#2) is also fair (extends snk-ocr with pattern matching, no new sidecar). But my "complexity 1/5" for video/GIF (#7) is *less harsh than Maintainer's*: he's projecting 6-10 weeks honest. Concur — I undercounted the encoder-bundling tax.
- **"GH-issue funnel for crash reports" instead of Sentry (Maintainer #7).** Brilliant. I hadn't considered the no-network version. Pre-filled URL → browser → user submits = zero network code in the app + zero servers + full user consent. This satisfies the spirit of "the user can report a crash" without the privacy or infra cost. Strictly better than Business's "self-hosted Sentry." Should be in any final list.
- **URL-scheme / deep-link automation (Maintainer #8).** I had drag-out (#8) and quick-share (#5) but didn't think of registering as a URL scheme. This is the *unlocks-third-party-integration* feature I missed — Raycast/Alfred/Stream Deck/Hammerspoon users can wire snapper-keeper into their own workflows. Reach is broader than I'd previously credit because each integration we don't have to build is leverage. Belongs in a final top 10 over my #8 or #9.
- **"iCloud / OneDrive backup-only mode" framing (Maintainer #9).** This is the honest scope of my #10 (LAN share) plus the cloud-sync candidate. Mirror PNGs (not the DB) to a synced folder, no SQLite-corruption risk, labeled as backup-not-sync. I'd say it pairs well with my LAN-share idea: one solves "don't lose my screenshots when my laptop dies"; the other solves "get this one screenshot to another device right now." Both are real, different use-cases.
- **Linux as "compile-only CI job before shipping Linux"** is the right intermediate step I missed. Catches porting bugs cheaply without committing to a third platform's bug reports. Even if Linux ships at v10, the compile-only check is a phase 8 freebie.

### To Competitive Landscape

- **Snagit / CleanShot baseline check.** I scored scrolling capture #1; Competitive scored it #1; Business scored it #2 (only because EV cert leapfrogs). Maintainer scored it #3 (Vision OCR + EV displace it). The convergence is very strong — **everyone wants scrolling capture in the top 3.** This is the strongest cross-perspective signal in this whole round.
- **Local share-link / BYO storage uploader (Competitive #3).** This is what I was reaching for with "Quick-share targets" (my #5) but couldn't articulate. Competitive sharpens it: not just `Open in app X`, but `Copy share link` backed by the user's own S3/R2/Backblaze/server. ShareX's SXCU model proves the pattern. This is more powerful than my Quick-share *and* doesn't violate the privacy posture (the user picks the destination). I'd take Competitive's #3 over my #5 in any synthesis.
- **Multi-image gallery / contact sheet (Competitive #4).** I didn't have this. It pairs with my Journey 2 (PM writing a spec) perfectly — the spec writer doesn't want one screenshot, they want a stitched "here's the flow" image. Cheap (Konva is already in the bundle), reuses step markers, hits the docs/tutorials use case Snagit owns. Worth slotting in.
- **Capture templates / step-tutorial builder (Competitive #10).** Same lineage as the contact sheet. Snagit's killer template feature. I underweighted "produce a shareable doc *from* snapper-keeper" relative to "send a single capture *out of* snapper-keeper." The PM/technical-writer personas I named would benefit most.
- **Competitive's risk #1: "Competitive parity is a trap."** Strong agreement and well-stated. Three of Competitive's top 10 (#1 scrolling, #2 video, #6 Vision OCR) are parity bets. The differentiated ones (#3 BYO share-link, #5 P2P sync, #8 plugin API, #10 templates+library) compound on what we already ship that nobody else has. The right mix isn't "all parity" or "all differentiation" but Competitive's #3 and #4 hit the sweet spot.

## Tensions

### Tension 1 — Cost: spend on EV cert vs. spend on engineering

Business puts EV cert at #1 ($300-600/yr, no engineering). Maintainer agrees ($300+/yr, ~0.5 dev-week). I (User-lens) treat it as "operational, not user-facing" and didn't include it. Competitive lists it at #9 because "EV solves the SmartScreen ramp but isn't a feature."

**My position is wrong in isolation, right in context.** From the user's seat, "the app installs cleanly" is the table stake before anything else matters — but it shows up not as a *feature* the user enjoys, only as the *absence* of a "Run anyway" dialog. Users won't celebrate it, but they'll bail without it. The team-lead should treat EV cert as a *prerequisite* alongside the feature list, not competing with features. Phase 8.0 = EV cert; then phase 8.1 = first user-visible feature.

### Tension 2 — Cross-device sync: how much, what shape

- **My position (#10):** LAN-only ephemeral QR share for one capture at a time. Tiny scope.
- **Business (#6):** LAN-only paired-device clipboard sync via mDNS + Ed25519 pairing. Medium scope.
- **Maintainer (#4):** LAN-only mDNS + QUIC + Ed25519 for clipboard + captures. Same medium scope as Business.
- **Competitive (#5):** Either BYO sync folder (Syncthing/iCloud) OR Tailscale-style P2P. Larger scope but caveats it's a cliff and v2-only.

Tension is on scope and shape. All four agree "no cloud, no servers, no accounts." But:
- My version is "share *this* capture to *that* device, ephemeral, no pairing." Lightest.
- Business/Maintainer's is "persistent device-to-device sync after pairing." Medium.
- Competitive's is "full library sync." Heaviest.

User-lens take: my ephemeral version solves Journey 5 (teacher to student) and Journey 8 (capture on laptop, get to phone *once*). The persistent paired version solves a power-user "two-laptop workflow." The full library sync is the dream but is genuinely months of work + conflict resolution. **Recommend: start with my ephemeral QR share to validate appetite. Persistent pairing is the next step if users actually use the ephemeral version.** Don't jump to full sync until that data exists.

### Tension 3 — Video/GIF: ship or skip

- **My position (#7):** Include, but flag complexity = 1/5 (hardest in my list).
- **Business (#5):** GIF only — ship via gifski sidecar. Skip MP4/H264 because of licensing/distribution complexity.
- **Maintainer (#5):** "Brutally minimal v1" — no audio, MP4 only, 30 fps, region/window only. 6-10 weeks honest. Or skip entirely and ship a *trimmer* (Maintainer #10) if a user drops a video into the app.
- **Competitive (#2):** Yes, ship FFmpeg sidecar for MP4 + GIF, no audio v1.

Real tension between Business (GIF only, gifski) and Competitive/Maintainer (MP4+GIF, FFmpeg). User-lens: GIFs auto-play in chat apps and issue trackers; MP4s don't. For Journey 5 (teacher/streamer) and Journey 3 (support repro), GIF is the format users actually paste. **Business is right that GIF-only is the higher value-per-effort ratio.** Maintainer is right that even GIF-only is 4+ weeks. Recommend: GIF-only v1; explicitly market it as "snapper-keeper makes GIFs, not movies — record once, paste anywhere." That's a sharper identity than "video tool".

### Tension 4 — Plugin / extensibility API: yes (with caveats) vs. no

- **Business (#7):** Yes, JS sandbox, read-only access + post-capture toolbar actions.
- **Competitive (#8):** Yes, ShareX-killer feature — narrow hook surface, versioned explicitly.
- **Maintainer:** *Rejected.* "Forever maintenance contract. The deep-link/URL-scheme approach (#8) satisfies 80% of automation needs without the cost."
- **My position:** I had it in my "Alternatives rejected" — same reasoning as Maintainer.

I align with Maintainer here. Deep-link / URL-scheme (Maintainer #8) gives 80% of the integration value with 5% of the cost and zero compat-contract burden. Plugin APIs are the road to "this snapper-keeper plugin broke after the v3 release" issues forever. **Recommend: ship URL scheme automation; defer plugin API at least to v3 or never.**

### Tension 5 — macOS Vision OCR: identity-divergence risk

- **Business (#3), Maintainer (#2), Competitive (#6):** all rank Vision OCR in their top 6.
- **My position (#6):** Also include it, but I flagged "first time we have per-OS feature implementation behind a shared interface."

Maintainer's risk-list and Competitive's risk-list both quietly accept the per-OS divergence cost; Business doesn't address it. **User-lens concern:** if a user searches their library on Windows and finds different results than on Mac (because Vision found text Tesseract missed), that's a "is this thing broken?" moment. **Mitigation:** when we replace OCR backends, re-OCR the entire library on first run with the new backend on that OS. Slight startup tax but consistent search behavior. Should be a written acceptance criterion if we ship Vision OCR.

### Tension 6 — Linux: defer, compile-only, or commit

- **My position:** Reject (too costly for side project).
- **Maintainer (#6):** Defer pending user signal; add compile-only CI now.
- **Competitive (#7):** "Underserved niche, but high cost." Rank low but not zero.
- **Business (#10):** Last in top 10 — Linux desktop is small but disproportionately influential for HN word-of-mouth.

Three out of four perspectives include Linux in the top 10 but rank it last; I excluded it. Tension is whether even a *deferred* Linux commitment is correct or whether we should write a "no Linux, full stop" decision and free up that future budget. **User-lens position:** if there's even a small commit to Linux, it forces every feature design to be "Linux-compatible" — adds a quiet tax to every plan. **Recommend: write a clear decision either way.** Maintainer's "compile-only CI" is a reasonable middle path because it doesn't promise users anything and costs only CI minutes.

## New Insights

1. **Phase 8 should be split into two tracks: infra + feature.** Three perspectives (Business, Maintainer, mine in retrospect) all point at "infra-defensive" items competing with "user-visible" items in the same ranking. They shouldn't. Phase 8.0 = EV cert + GH-issue crash funnel + compile-only Linux CI (all near-zero engineering, big payoff). Phase 8.1 = first user-visible feature (scrolling capture). This sequencing means installs work cleanly *and* the first real user (whoever they are) experiences the headline feature.

2. **"Text Actions overlay" + "Capture as markdown link" + "Selective area OCR" might be one feature, not three.** Each perspective named a variant:
   - Business: "Text Actions on captures" (display OCR text as selectable overlay)
   - Maintainer: "Selective area OCR" runner-up
   - Competitive: "Local share-link workflow"
   - Mine: "Quick-share targets on toolbar"
   Unifying frame: **after a capture, the post-capture toolbar exposes one row of context-aware actions**: Copy text (if OCR found any), Copy as link (if upload destination configured), Copy as image (default), Save to folder X. Each action is cheap individually; the row is a polish-and-discoverability layer that ties OCR + sharing + the existing toolbar together. Could be a single 1-2 week phase. Maximum surface-area-of-perceived-improvement per dev-week.

3. **The competitive moat is the *combination*, not the individual features.** Competitive explicitly says: "the differentiated path is Snagit-quality core + combined clipboard + privacy + free." None of the *individual* features in my ranking achieves this; the differentiation comes from shipping a coherent set. Implication for the synthesis: prefer features that *compound* what's already shipped (Vision OCR compounds search; LAN share compounds clipboard popup; multi-image gallery compounds the library) over features that build new product areas (video, plugin API). My ranking did this implicitly; Competitive named it explicitly.

4. **"No telemetry" is having a bigger cost than any of us individually scored.** All four of us listed "we have no user signal" as an Open Question. Business said it's the root question. Maintainer said it forces every prioritization to be a guess. Competitive said it makes the share-link decision unanswerable. I said the same in mine. **Insight:** the cost of *strict* zero-telemetry is *prioritization debt*. A single opt-in install-counter (no PII, just count + OS + version, opt-in on first run) would let every future ranking be data-driven instead of speculative. The privacy posture would survive if framed correctly. Worth raising as a posture-revisit decision even if the answer is still no.

5. **"Drag out from library" (my #8) overlaps with "Recent destinations" (my #5) and "Copy as link" (Competitive #3).** These are all variants of "fast path to send a capture somewhere." A single coherent design (a toolbar row + a library-card hover-action + a global hotkey for `paste last capture as link`) might cover all three at lower total cost than building them piecemeal. Worth designing the destination model holistically before shipping any of them.

6. **PII auto-redact (my #2) has a sibling: hide-detected-text (proactive blur) at capture time, not after.** Win11 Snipping Tool added pre-capture redact recently (Competitive noted it). If we ship OCR-driven redaction *on capture* rather than *after annotation*, the user never sees an unredacted version on disk — strictly better privacy posture. Same underlying tech (Tesseract bounding boxes); different UI surface. I had it as a post-hoc feature; the pre-capture form is more powerful.

7. **The team-lead should explicitly decide whether snapper-keeper is "Eric's personal utility" or "a product trying to find users".** Business asked this and it determines half the ranking. If personal utility: ship things Eric will use daily (smart-sections, Vision OCR, scrolling capture, redact). If product seeking users: ship things that demo well and pass the SmartScreen install (EV cert, Text Actions overlay, GIF, share-link). The two lists overlap but aren't identical. A written answer up front would unblock the synthesis.

8. **All four perspectives independently rejected "true cloud sync."** This is the strongest cross-perspective NO in the round. Worth promoting from "deferred" to "explicit non-goal" in the decisions log. The seed-list item should be invalidated outright, not just downranked, so it stops appearing in future planning exercises and the team can spend its imagination on differentiated alternatives instead.
