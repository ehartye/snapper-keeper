# Round 2 — Maintainer cross-pollination

My Round 1 position is locked. Below is what changes in my mind given the other perspectives, and what doesn't.

---

## Reactions

### On Business/Strategy

**EV cert (#1) — strong agreement, but they slightly undercount the ongoing tax.** Business ranks it 5/5/5/5; I ranked it 5/4/5/5. We're aligned. Their flag in Risk #4 — "if Eric stops paying, the auto-updater chain breaks on next signing" — is the correct framing and the maintainer-relevant detail neither of us bolted on the front of our positions. I'll add: it's not just publisher mismatch, it's that **the dotnet `sign` tool path, Azure profile name, and Tauri `signCommand` are baked into `tauri.conf.json:104`** today. Switching cert vendors mid-flight is a `signCommand` edit + 5 follow-up commits (we already lived that movie in phase 7). Worth doing once; lock the vendor on a 3-yr contract if available.

**Crash report uploads — they kept self-hosted Sentry (#8 at fit 4/5).** I rejected it and kept only the GH-issue-funnel form. **I think my framing is more honest for this audience.** Their "sentry.io free tier under @hartye.com" suggestion is interesting but: (a) it's still a hosted third party that sees stack traces and timestamps, (b) onboarding a third party with a privacy posture page in the wizard adds UI surface, (c) "free tier" is rate-limited and one viral HN post torches the quota. The GH-issue-funnel form has zero infra, zero TOS, and works whether we have 5 users or 500. Either approach unblocks the same problem; I stand by mine being cheaper to maintain.

**Plugin API as JS sandbox (#7) — I rejected; they accepted a scoped form.** They have a real point that I underweighted: scoping to a webview-side JS sandbox with read-only library access dramatically reduces the API surface from what I imagined (Rust ABI / WASM modules / sandboxed processes). It maps to existing Tauri capability scoping. **I still don't love it.** The trap isn't "is the v1 surface small" — the trap is that *every plugin install becomes a malware vector* and we now have to think about plugin signing, plugin discovery, plugin permissions, and "did Eric vet this plugin." Business explicitly acknowledges this in Risk #6. Their #8 (CLI / URL-scheme) hooks captures the upper-80% of the value without the perimeter expansion. I'd cut their #7 in favor of my #8 and add the runner-up Smart Sections.

**Microsoft Store at #9 — partially redundant with EV cert.** Business calls this out themselves in Risk #7. **Maintainer view:** MSIX repackage is genuinely cheap-ish (Tauri has decent support), but it introduces a *parallel* update channel (Store auto-update vs. Tauri updater) and we then have to keep two manifests in sync forever. That's a permanent maintenance tax I'd rather not pay if EV cert already solves SmartScreen. Pick one. **Rank EV cert above MS Store, ship MS Store only if EV cert proves insufficient.**

### On User/Consumer

**Scrolling capture #1 — agreement.** Both User and Competitive put this at #1; I put it at #3. **They're more right than I am about ranking.** I downweighted it on complexity grounds (3-4 weeks honest estimate) and that's the maintainer instinct — but if every other perspective independently picks it as the top need, the *demand signal* trumps my cost concern. I'd swap mine to #2 behind EV cert.

**PII auto-redact (User #2) — I missed this entirely.** This is the best single suggestion in any of the three rounds. **Complexity reality-check from the codebase:** we already have OCR text + bounding boxes in `ocr_text` table from phase 5. The blur tool exists from the 2026-05-22-blur-tool-pixelation.md polish phase. Pattern-matching regex is ~50 lines per category. Wiring it as a "Suggest redactions" button in the annotate window that *populates* blur shapes (user confirms before save) is ~3–4 days of work, not weeks. User's complexity score of 4/5 is honest. **Privacy-first identity feature for ~5 days of work — this should be in the top 5.** I'd ship this before scrolling capture if I had to pick one.

**Eyedropper + pixel-measure (User #3) — agree, cheap, missed by Business and me.** Pure Konva canvas additions, the canvas tooling is in `app/src/windows/annotate/`. <500 LoC each. Designer audience-fit is real. Honest fit: this is 2 days of work for one of them, a week for both. Belongs in any list focused on table-stakes-vs-paid-competitors.

**Paste-as-plain-text (User #4) — easy and 5/5 reach.** I missed it. **Complexity from the codebase:** the popup currently calls `paste_item` in `crates/snk-clipboard/src/paste.rs:113`. Adding a "transform on paste" parameter that strips RTF/HTML before `arboard.set()` is ~50 lines + a Shift+Enter binding in the popup window. Half-day max. This is what I'd actually rank as the highest-leverage-per-hour item on any list.

**LAN ephemeral share via QR (User #10) — interesting variant of my #4 LAN-sync.** User's framing is *file-share, not state-sync*: spin up a tiny localhost server on the LAN interface, render a QR, link expires in 5 min. This is meaningfully cheaper than my LAN-sync proposal because there's no pairing UX, no E2EE handshake, no clipboard-sync protocol — just an ephemeral HTTP listener with a TTL. **It captures journey 8 (multi-device) at maybe 30% the cost of full LAN-sync.** Risk: self-signed cert UX on a phone is genuinely painful; the HTTP-on-trusted-LAN mode they propose is fine but raises eyebrows in any network-security review. I think there's a smart hybrid here: ship LAN-share (cheap, narrow, immediate value) first; LAN-sync (more general but expensive) only if signal demands it.

### On Competitive

**Competitive #1 = scrolling capture, agreement again.** Their ShareX-style diff-stitch approach is the right call: it's **OS-agnostic enough** that we don't need separate Win/Mac scroll-synthesis paths *if we accept programmatic scroll injection via existing OS hotkey + image-similarity stitching*. **Maintainer reality-check:** their complexity score of 2/5 lines up with mine. Probably 3-4 weeks honest with the simple algorithm; "sticky-header detection" and "lazy-load aware" are CleanShot-level polish that we should explicitly *not* ship in v1 (Competitive flags this implicitly when they call out CleanShot is best-in-class).

**Local share-link via BYO-bucket (Competitive #3) — strong, plus this is a viable bridge to no-server cloud sync.** I want to call this one out specifically. The ShareX SXCU approach — "configure your own S3/R2/Backblaze, post-capture toolbar gets `Copy share link`" — is **perfectly on-brand for audience B**. The user runs no service (they consume an existing cloud bucket they already pay for / are eligible for free-tier), we run no service. Engineering cost is contained: an upload abstraction trait, 2-3 backend implementations (S3-compatible covers most), a credential storage shim (use `keyring` crate, OS-native secure store), a config UI in Settings. **My estimate: ~2 weeks.** This is genuinely better than my #9 "iCloud/OneDrive backup-only mode" because it preserves user privacy posture *and* solves the "send a link to Slack" use case my proposal doesn't. **I'd swap Competitive #3 in for my #9.**

**Video/GIF combined at #2 (Competitive) vs split GIF-only (Business #5) vs Video-too-expensive (me #5):** Competitive is honest about the 1/5 complexity score. Business's "split GIF, defer video" is a meaningful narrowing — gifski as a single binary sidecar is way cheaper than full FFmpeg-with-H264. **Maintainer view:** if we ship recording at all, **gifski-only is the only acceptable v8 scope.** Adding MP4 means: H264/H265 patent encumbrance (commercial use is technically licensable but practically tolerated), codec-encoder version-management across runners, audio device enumeration, file-size warnings. GIF via gifski avoids 80% of that pain. I'd raise this in priority if we're shipping any recording: **Business's #5 GIF-only framing is correct, Competitive's #2 combined-MP4-and-GIF is over-scope.**

**Capture templates / step-tutorial builder (Competitive #10) — interesting, but I'd downrank.** Snagit's templates are powerful precisely because they're an editor surface, not a feature. Building "multi-select → composite into one page with captions" is a real product effort, not a 2-week feature. Better as a runner-up than as a top-10. Pairs naturally with their #4 (multi-image gallery) which I'd merge into a single "compose multiple captures" item.

**Cross-device sync at #5 (Competitive) — too high.** They're right that competitors lean into multi-device hard. They're also right that BYO-storage / Tailscale-style P2P is the architecturally honest version. **But** the engineering cost they themselves rate 1/5 (multi-month) for what is genuinely a hard problem (SQLite + conflict resolution + encryption + attachment movement). For a side-project maintainer this is the highest-risk item on any of the three lists. **Competitive's User-style #10 ephemeral share is a strictly better first step.**

---

## Tensions

### Tension 1 — How heavy should "user-facing competitive parity" features weigh vs. "broaden the funnel" infrastructure?

User and Competitive both rank scrolling capture #1 and video/GIF in the top 3. I (and Business) rank **EV cert / SmartScreen** as #1. Both are defensible.

**Maintainer judgment:** EV cert is right *if* the time budget is small or current install friction is high. Scrolling capture is right *if* the goal is to be feature-competitive with Snagit. Without user signal, this is a religious argument. **I'd resolve it by phasing: ship EV cert + paste-as-plain-text + PII-auto-redact in a quick "polish phase 8" (~2 weeks), THEN start scrolling capture as phase 9 (~4-6 weeks).** Both perspectives get satisfied; nothing is starved.

### Tension 2 — Plugin/extensibility API: User says no, Business says scoped-yes, Competitive says ShareX-killer-yes

I (Round 1) rejected the plugin API outright in favor of CLI / URL-scheme hooks (my #8). Business proposed a JS sandbox form. Competitive proposed "post-capture pipeline hooks + custom uploaders." User explicitly defers.

**The tension resolves cleanly if we separate two things:** (a) "let users automate snapper-keeper from outside" (URL-scheme / CLI) is **strictly safer** than (b) "let users install plugins inside snapper-keeper" (sandboxed JS / WASM). (a) is my #8; (b) is what Business and Competitive are pushing for. **For audience B + one maintainer, (a) covers 80% of the real value at 5% of the perimeter expansion.** I'd ship (a) and explicitly defer (b) until there's user demand we can't satisfy with (a).

**Competitive's "custom uploaders" piece (their #3 share-link) is *not* a plugin API** — it's a config-driven feature with N built-in backends (S3-compat, Backblaze, custom-endpoint). That's distinct from "user-installable code" and should be evaluated separately. I think it's a good feature; I think it's NOT a plugin API.

### Tension 3 — Cross-device sync: User wants ephemeral share (LAN, file-only), Business wants LAN clipboard sync, Competitive wants P2P + BYO storage

Three different shapes of the same need. My Round 1 picked LAN clipboard sync (Business's flavor) because the existing clipboard plugin is the natural locus.

**Re-reading: User's framing is cheaper AND more universally useful.** Ephemeral LAN share via QR-code solves the "phone needs the screenshot" use case for *any* capture in seconds, without pairing UX, persistent state, or a sync protocol. Business's clipboard-sync solves a different but narrower need ("paste from Mac to Windows"). Competitive's BYO storage / Tailscale-style is the most powerful and also the most expensive.

**Maintainer ranking:** ship User's #10 (LAN ephemeral share, 1-2 weeks) before either of the others. Re-evaluate after that lands. This is a case where my Round 1 was too ambitious.

### Tension 4 — macOS Vision OCR: all four of us picked it (in different slots), nobody pushed back

User says #6, fit 4/5. Business says #3, fit 5/5. Competitive says #6, fit 5/5. I said #2, fit 5/5. **High consensus, varied placement.** The reason it's high in some lists and lower in others is "is the OCR quality gap user-perceivable today?" The answer is yes if you're a Mac user comparing snapper-keeper to Shottr; the answer is no if you're Win-only.

**No real tension; the placement difference is honest weighting of cross-OS vs. Mac-only impact.** This is a feature I'd ship early because the architecture-fit is excellent (`snk-ocr` already abstracts backend, queue is built) and it *removes* a maintenance burden (Tesseract sidecar bundling on Mac). It belongs in the top 4 of any synthesis.

### Tension 5 — Linux: I said #6 (defer with explanations), Business said #10 (last but ship), Competitive said #7 (mid-list)

All three of us say "expensive, niche-but-influential audience." User explicitly drops it.

**The tension is: how much does "but Linux devs would love us" weigh against "Wayland is fundamentally hostile to two of our flagship flows"?** Business's framing — "underserved niche, but high cost" — is exactly right. Competitive's complexity score of 1/5 matches mine (multi-month).

**I'd resolve it the same way for any reasonable budget: add a `--target=x86_64-unknown-linux-gnu` *compile-only* job to CI (free, catches portability bugs cheaply), defer actual Linux installer + Wayland portal work until there's GH-issue signal from real Linux users.** None of us picked it for top 5; the question is just whether to ship it at all in the next 10 features. **I'd remove it from the top 10 and replace it with a User #2 (PII auto-redact) or User #4 (paste-as-plain-text) which are 10× cheaper and 5× more universally useful.**

---

## New Insights

These I missed in Round 1 and now think are important:

### Insight 1 — "Surface what we already have" features dramatically outscore "build something new" features on ROI

User #2 (PII redact uses existing OCR + existing blur tool), User #3 (eyedropper reads existing image pixels), User #4 (paste-as-plain-text transforms existing clipboard payload), Business #4 (Text Actions exposes existing OCR data), Competitive #4 (multi-image stitch uses existing Konva canvas). **These all reuse phase 1-7 investments and cost 2-7 days each.** None of them were on my Round 1 list. Collectively, ~3 weeks of work could ship 4 of these and materially upgrade the product.

**My Round 1 was anchored to the seed list's deferred-features framing.** The seed list biases toward "big things we explicitly punted on" rather than "small things we never thought to ship because they weren't on the design doc." Business explicitly flags this (their Risk #1). **A synthesis list should heavily weight these low-cost-surfacing items.**

### Insight 2 — The "video recording" question is really three different decisions, not one

After reading all three perspectives, I see the video question decomposes:
- **(a) GIF-only via gifski sidecar** — Business #5. ~2-3 weeks. Single binary per OS. No licensing drama.
- **(b) Trim-existing-video** — my Round 1 #10. ~1.5 weeks. Bundled FFmpeg sidecar but stream-copy only, no transcoding.
- **(c) Full screen recording with MP4+audio** — Competitive #2 as written. 6-10 weeks. Multiple new platform APIs. Bundle size +30MB. Audio device enumeration. New phase-sized commitment.

**These should be evaluated separately.** Competitive bundled them; Business split (a) from (b)+(c); I had only (c) in mind. The honest ranking is **(a) >> (b) > (c)** on value-per-week. Synthesis should consider shipping (a) only, deferring (b) and (c) explicitly.

### Insight 3 — Both Business and Competitive independently want a share-link workflow; they're describing the same architecture differently

Business doesn't put this in their top 10 explicitly, but mentions it implicitly when they discuss "users want to send a Slack link." Competitive makes it their #3 (BYO-bucket "Copy share link"). User #5 (Quick-share targets on toolbar) is the *toolbar UX* of the same idea. **All three want a "send-as-link" exit ramp from the post-capture flow.** None of us put it in the top 3, but in aggregate it's clearly a top-3 demand.

**Maintainer view:** the BYO-bucket form is the right architectural answer — config-driven backends, OS keyring for credentials, ~2 weeks of work. Compared to my Round 1 #9 (iCloud/OneDrive backup-only), this is strictly better. **I'd swap it in.**

### Insight 4 — None of us deeply explored the "phase 8 release rhythm" question, but it's the real upstream question

All three perspectives (and mine) implicitly assume "list 10 features, rank them, build them in order." That's not actually how phase 7 worked or how a part-time side project works. **The honest framing is:**

- **Quick-polish phase 8 (~2-3 weeks):** EV cert + paste-as-plain-text + eyedropper + pixel-measure + PII redact + GH-issue crash funnel. 5 items. Ships in 1 release.
- **Bigger phase 9 (~4-6 weeks):** Scrolling capture OR Vision OCR OR BYO-bucket share. Pick one. Ships in 1 release.
- **Stretch phase 10 (~3-4 weeks):** LAN ephemeral share OR gifski recording. Pick one.

**This frame would let the synthesis output not just a ranked list but a phasing plan.** Worth flagging to the lead before synthesis.

### Insight 5 — Nobody discussed Wayland's auto-paste blocker, which is a Linux dealbreaker for our core feature

I mentioned this in Round 1; Business and Competitive both acknowledged Linux is expensive but neither articulated *why specifically* — they pointed at portals, signing, and tesseract packaging. **The unstated reason Linux is harder than they think: Wayland blocks synthetic-input by design.** Our auto-paste flow (the entire premise of the caret-anchored clipboard popup with "Enter to paste") **does not work on Wayland.** It works on X11. It will not work on Wayland without users granting wlroots-specific or compositor-specific input portal access, which doesn't universally exist.

**This is the single fact that should drop Linux out of any top-10 for this product.** It's not just "Wayland is fiddly"; it's "one of the two killer features doesn't exist on the dominant Linux display server in 2026." If we cared about Linux later, the answer is "ship a degraded mode (clipboard popup with clipboard-only, no auto-paste) on Wayland and live with the asymmetry." That conversation hasn't been had. None of us should be ranking Linux above ANY other top-10 candidate until it has.

---

## Bottom line for synthesis

Locked Round 1 stands. Cross-pollination shifts:

- **+2 ranks for scrolling capture** (three independent perspectives picked it #1 — I was too conservative)
- **+ADD: PII auto-redact, paste-as-plain-text, eyedropper/pixel-measure** as cheap-but-high-value items I missed
- **Replace my #9 (iCloud backup-only)** with Competitive's #3 (BYO-bucket share link)
- **Replace my #4 (LAN clipboard sync)** with User's #10 (LAN ephemeral file share via QR) — strictly cheaper for similar value
- **Hold my position** on EV cert at #1, Vision OCR top-4, deferring full video, downrating plugin API, and removing Linux from any top-10.
