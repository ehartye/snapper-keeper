# Cross-Pollination — Round 2 — Competitive Landscape

Perspective: **Competitive Landscape**
Date: 2026-05-24
Round 1 (locked) lives at: `round-1/competitive.md`

## Reactions

### To Business/Strategy

**Strong agreement: EV cert is the highest-leverage move (their #1, my #9).**
Business put it at #1; I had it at #9. Business is right that "feature ranking is moot until installs actually convert" — every paid Win competitor has reputation; we have a flat install funnel until SmartScreen warns away the first ~3000 users. The disagreement is one of framing, not value: I was ranking by *user-visible feature value*, they were ranking by *funnel value*. They're correct that funnel comes first for a side project with zero telemetry. If I were producing a single ordering instead of a perspective-flavored top-10, EV cert moves up to ~#3-4.

**Strong agreement: their #4 (Snipping Tool–style "Text Actions" overlay on capture).**
This was NOT on my list and it should have been. Business identified that we're already paying the OCR cost in phase 5 but only surfacing it through search. Exposing OCR text as a selectable overlay + "Copy text" toolbar button is sub-1-week effort with massive demo value — it competes head-on with Win11 Snipping Tool's headline 2025/26 feature (Text Actions) and macOS Live Text. This is the single most under-priced item across all three positions. Compounds well with #6 (Vision OCR) and with User's #3 (eyedropper) since both reuse the capture overlay framework.

**Tension: their GIF-only split (#5) vs. my Video/GIF combined (#2).**
Business split video into "GIF only via gifski" and "MP4 later." This is the right call competitively. GIFs auto-play in Slack/Discord/Jira/GitHub; MP4 needs a player and a download. **GIF is the viral artifact, MP4 is the file.** Bonus: gifski is a single-binary sidecar with simple licensing (CC-BY-NC-4.0 — wait, that's a real constraint, see Risks below). Splitting drops the complexity score from 1/5 to ~3/5 and the maintenance load by a lot. I'd happily adopt their framing in any synthesis.

**Tension: their LAN-only sync (#6) reframe of my "cross-device sync."**
Business reframed my cloud-sync candidate to LAN-only with mDNS + libp2p/QUIC. This is better than what I proposed. It dodges the SQLite-corrupts-under-cloud-sync problem entirely, keeps no-servers identity, and matches a beloved category (LocalSend, KDE Connect). I had LAN-only as one bullet inside my #5; Business made it the *whole feature*. Adopting their framing improves it. **However** — Business under-rates the maintenance cost of P2P networking on Windows (firewall rules, mDNS quirks on enterprise networks). Worth scoring complexity at 2/5 not 2/5… actually we agree on the complexity score. Fine.

**Tension: scope of plugin API (their #7 sandboxed JS vs. my #8 broader).**
Business correctly tightened "plugin/extensibility API" to *read-only JS in the existing webview sandbox*. My round-1 framing was looser. Their version is the right shipping shape — it avoids the "every plugin install is a malware vector" risk while keeping the "ShareX killer" pitch. I'd defer to their framing.

**Mild disagreement: Microsoft Store (their #9) ranked above my list.**
I rejected MS Store in my "Alternatives Proposed." Business kept it as the #9 backup-to-EV-cert. They're right that it's a SmartScreen workaround, but I still think the cost (MSIX repackage, separate update pipeline, Partner Center maintenance, potential sandboxing surprises around global hotkey + accessibility) doesn't justify it *when EV cert is already ranked #1.* Pick one. Business explicitly flags this in their Risk #7 ("EV cert and MS Store are partially redundant — should think of as a bundle and pick"). We agree on the conclusion.

### To User/Consumer

**Strong agreement: scrolling capture as #1.**
We both have it at #1 with identical scoring. Three out of four perspectives almost certainly land it there. This is unambiguously the most-aligned candidate across the team.

**Strong agreement: macOS Vision OCR.**
User has it at #6 with 4/5 value; I have it at #6 with 4/5 value. Independent confirmation. Their installer-size-reduction angle (~50 MB smaller Mac bundle) is one I didn't emphasize — that's a real win.

**New insight from User: PII auto-redact (their #2) reuses 90% of shipped work.**
This was NOT on my radar. User correctly identifies that tesseract already produces text + bounding boxes per capture, and the blur tool already exists in the annotator. Pattern-matching for emails/phone/CC/SSN and surfacing them as "suggested redactions" is genuinely a few hundred LoC across `snk-ocr` and the canvas. **This is the highest-fit feature to the privacy-first identity** — it's defensively the move competitors who phone home for AI redaction can't easily match. I'd insert this into my top 10 if Round 1 weren't locked. Probably top 3 in synthesis.

**New insight from User: paste-as-plain-text modifier on the clipboard popup (their #4).**
Genuinely table stakes that I missed entirely. Maccy, Paste, Alfred, Raycast all ship this. Shift+Enter strips formatting. <1 day work. Reach 5/5 because anyone who pastes from a browser into Word has hit this. Belongs in any 10-item list.

**New insight from User: eyedropper + pixel-measure (their #3).**
Designer audience that I had in mind but didn't surface as a specific feature. Both tools are <500 LoC on the existing Konva canvas. CleanShot has a magnifier loupe; nobody combines eyedropper + measure cleanly. Fits the "compound the annotation lead" thesis.

**New insight from User: LAN ephemeral share via QR code (their #10).**
Different from Business's LAN sync and different from my BYO-S3 link workflow. User proposes the app spins up a localhost HTTPS server bound to the LAN interface and renders a QR code; phone scans, gets the image, link dies. This is *operationally* simpler than either P2P sync (Business) or BYO-S3 upload (mine) — no pairing, no credentials, no buckets. Combined with my #3 (BYO share-target endpoints) and Business's #6 (LAN sync) these are three different points on the "cross-device" design space and probably ONE of them should be in the final 10, not all three.

**Tension: User's #7 (Video/GIF combined) keeps the combined framing.**
User retained the combined video+GIF candidate at 1/5 complexity. Business correctly split it. The split is better — User's combined framing concedes too much complexity for the marginal reach of MP4-over-GIF.

**Tension: User explicitly rejects EV cert as "operational, not user-facing."**
User dropped EV cert from their top 10 reasoning that it's not user-visible. Business put it at #1. **Both views are valid for their lens.** From competitive lens, I side with Business: the SmartScreen warning *is* a user-facing competitive disadvantage versus every paid Mac/Win competitor that has zero install friction. User's lens optimizes for "real user features"; Business's lens optimizes for "things that get features to users." Synthesis should keep it.

### To Maintainer

**Strong agreement: their #1 (EV cert) and #2 (Vision OCR) for cheap, high-impact wins.**
Maintainer ranks EV at #1 and Vision OCR at #2 — same as Business. The convergence here is striking: three out of four perspectives (Business, Maintainer, and mine if I were honest about funnel value) want EV cert near the top. User is the only dissent. Vision OCR has a similar convergence.

**New insight from Maintainer: their #7 (manual GH-issue funnel for crashes) is brilliant.**
Maintainer reframed "self-hosted Sentry" as "panic_hook writes a JSON crash dump locally + one-click button opens a pre-filled GitHub issue in the user's browser." Zero servers, zero network code in the app, full user consent every time. This is *better* than the seed candidate. I had crash reporting in my "Alternatives Proposed" as a rejection; Maintainer's framing flips it to a clear yes. Drops complexity from 1/5 (self-hosted infra) to ~5/5 (a few days). Belongs in the synthesis top 10.

**New insight from Maintainer: their #8 (CLI / URL-scheme automation).**
`snapper-keeper://capture/region` deep-link integrations with Raycast, Alfred, Stream Deck, Hammerspoon, AutoHotkey, Shortcuts.app. **This is the elegant version of the ShareX-power-user pitch** without building a full plugin API. Lets us claim "scriptable + automation-friendly" without owning a plugin marketplace. Tauri 2 has `tauri-plugin-deep-link` so cost is genuinely 1 week. I'd take this over a full plugin API any day. Complements Business's #7 (sandboxed JS) — you could ship deep-link first, then JS extensions later only if signal warrants.

**New insight from Maintainer: their #9 (iCloud/OneDrive backup-only mode).**
A "backup-only" mode that mirrors capture PNG files (not the DB) to a user-selected sync folder. Doesn't pretend to be sync — it's one-way file copy. Honest about what it is. Cheap (extends `snk-library/files.rs`, no new deps, ~1 week). I had dismissed cloud-folder mode in my Open Questions as risky; Maintainer correctly notes that *as long as the DB stays out of the synced folder*, it's safe. **This satisfies the most common "I want cloud sync" request (don't lose my screenshots) at near-zero infra cost.** Belongs in synthesis.

**New insight from Maintainer: their #10 (video trimmer-only) as a video alternative.**
Drop-a-video-file-onto-library → trim → export, using FFmpeg stream-copy. Punts on the hard "record the screen" problem and just becomes useful when users already have a video. I find this less compelling than full GIF recording (Business's #5) but it's a legitimately cheaper way to get "video support" in the bullet list. Worth it as a backup if GIF recording is too expensive.

**Tension: Maintainer's complexity calibration vs. mine.**
Maintainer cites phase 7's actual ~5 follow-up commits and used those as a calibration anchor; my complexity scores were less data-driven. Maintainer's complexity scores (especially scrolling at 2/5 with an honest "fragile on canvas/virtualized lists" caveat, and video at 1/5 with "6-10 weeks honest estimate") feel more accurate than mine. **My #2 (Video/GIF) at complexity 1/5 was probably too generous given the splitting wasn't applied** — Maintainer says 6-10 weeks even for trimmed scope, that's brutal. Business's GIF-only at gifski sidecar is more like 3-4 weeks per Maintainer's calibration.

**Tension: Maintainer ranks Linux higher (#6) than I did (#7) but with a similar reluctance.**
We agree on the underlying analysis (Wayland UX rework, +1 CI runner, "fundamentally changes two flagship flows"), differ slightly on relative position. Both of us put it well below scrolling capture / Vision OCR / cross-device. Functionally aligned.

**Mild disagreement: my plugin API (#8) vs. Maintainer's rejection.**
Maintainer explicitly rejected plugin/extensibility API as too expensive. I had it at #8. Business had a tightened version at #7. Reading Maintainer's reasoning ("stable wire-format API constrains internal refactors permanently, every internal change becomes a compat review, plus discovery and forever backwards compat"), I think Maintainer is right and I was wrong. **Replace it with Maintainer's #8 (URL-scheme automation)** which delivers 80% of the same "scriptable" pitch at 20% of the cost.

## Tensions

The four perspectives disagree on these structural questions:

1. **EV cert ranking.** Business #1, Maintainer #1, mine #9, User dropped. The competitive lens (mine) was right to include it but wrong to rank it so low. **Resolution: it should be top 3 in synthesis.** It's a competitive-disadvantage fix even though it's not a feature.

2. **Cross-device feature shape.** Three different framings of the same wish:
   - Mine: BYO S3 upload + "Copy share link"
   - Business: LAN-only mDNS+QUIC clipboard+library sync
   - User: LAN ephemeral QR-code share for a single image
   - Maintainer: iCloud/OneDrive folder mirror (backup-only)
   **All four address different sub-uses of "I want it on another device."** The synthesis should pick one or two, not all four. Ranking by competitive differentiation: Business's LAN sync is most distinctive (nobody combines cross-device clipboard+library without cloud); Maintainer's folder mirror is cheapest insurance; User's QR is the lightest UX win. **My BYO-S3 upload is the most-redundant** with the other three and I'd drop it from synthesis.

3. **Video framing.** Mine: combined video+GIF. Business: split, ship GIF only. Maintainer: warns about full video honestly (6-10 weeks), suggests trimmer-only as cheap alternative. User: combined. **The split is right.** Synthesis should have GIF recording (gifski sidecar, no audio v1) explicitly, not video.

4. **Plugin API vs. URL-scheme.** Mine: full plugin API. Business: sandboxed JS. Maintainer: rejects, replaces with URL-scheme deep links. **Maintainer's URL-scheme wins the cost-value tradeoff.** Synthesis should drop the plugin API and adopt URL-scheme deep links.

5. **Crash reporting framing.** I rejected. Business kept self-hosted Sentry at #8 ("opportunity cost scales linearly with user count"). Maintainer reframed as one-click GH-issue funnel (no server, no network code, user consent every time). **Maintainer's reframing is unambiguously better than either.** It captures Business's "we need visibility" point at near-zero cost while preserving my "no telemetry posture" objection.

6. **Linux.** All four perspectives ranked it in the 6-8 zone or dropped it. **No real tension; it's a unanimous "yes but not now."** Maintainer's suggestion to add a *compile-only* Linux CI job (without shipping installers) is a clever middle-ground I missed.

## New Insights

Things I didn't see in Round 1 that other perspectives triggered:

1. **The four cheapest items would beat my top three.** Business's Text Actions (~1 week), User's paste-as-plain-text (<1 day), User's eyedropper+measure (<1 week each), Maintainer's GH-issue crash funnel (~3 days), and Maintainer's URL-scheme deep links (~1 week) sum to <6 weeks of work but deliver a *user-visible feature density* that probably exceeds scrolling capture alone. My competitive lens over-weighted "big features that close gaps with Snagit" and under-weighted "many small features that compound the existing identity."

2. **"Compound the existing investment" is a recurring theme across all three other perspectives.** Vision OCR makes phase 5's OCR feel smarter. PII redaction reuses OCR + blur tool. Text Actions overlay surfaces OCR data we already index. Eyedropper rides existing Konva canvas. Drag-out source ships with the existing library window. **None of these are visible from a pure competitor-feature-matrix exercise.** Competitive analysis (my lens) systematically misses these.

3. **The Snipping Tool / PowerToys angle is real.** Both Business and User noted that Win11 Snipping Tool's "Text Actions" is the viral feature in 2025/26. I had Win Snipping Tool in my landscape table but didn't connect the dots that *Microsoft just validated the "screenshot → grab text" UX*. We should fast-follow on a workflow Microsoft just spent millions of marketing dollars normalizing.

4. **The "no-cloud but make sharing work" design space has at least 4 distinct shapes.** Before Round 2 I assumed one solution. Business / User / Maintainer each independently invented a different one (LAN P2P sync, LAN ephemeral QR, cloud-folder backup mirror). The right Phase-8+ plan probably picks 2 of those 4 — one for "actually sync across my own machines" (Business's LAN sync OR Maintainer's folder mirror) and one for "send to my phone or a colleague right now" (User's QR share). Mine (BYO S3) is the only one that requires the user to do infra; it's strictly worse than the alternatives for most users.

5. **The "GIF specifically, not video" choice is a brand-coherence decision, not just a scope decision.** Business correctly framed GIF as a viral artifact (plays everywhere) and MP4 as a file (needs a player). Shipping GIF only is a statement that snapper-keeper is "screenshot tools that share well," not "video tools that compete with Loom." Keeps the identity sharp. I had video in my top 10; the synthesis should have GIF and explicitly not MP4.

6. **The biggest blind spot in my Round 1: I had no PII / redaction-aware item.** User's #2 is arguably the single most on-brand feature on the entire combined board — it directly differentiates against cloud-AI competitors that can't make this promise. From a competitive lens this is a *defensive moat* feature, not just a user-friction feature. Synthesis should rank it very high.

7. **gifski licensing wrinkle (a correction to Business's #5).** gifski is dual-licensed AGPLv3 / commercial. Distributing as a bundled sidecar in a free OSS app is fine under AGPLv3 *as long as snapper-keeper itself is AGPL-compatible.* The current repo doesn't specify a license publicly; if Eric intends MIT/Apache for snapper-keeper, gifski as bundled binary may require source disclosure or paying for the commercial license. Alternatives: `image` crate's GIF encoder (slower, larger files), or a custom encoder. **Worth checking before committing to gifski as the GIF path.** This isn't a deal-breaker — there are workarounds — but it's a footnote the synthesis should not skip.

8. **Maintainer's "compile-only Linux CI" idea generalizes.** The same logic applies to scrolling capture's per-OS code paths and to LAN sync's networking code: *write portable code, add CI to verify it compiles cross-platform, ship installers only where we're confident.* This is cheap insurance against the "future feature is impossible to port" trap.
