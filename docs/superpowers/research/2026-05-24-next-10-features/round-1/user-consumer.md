# Round 1 — User/Consumer perspective

**Author:** User/Consumer
**Date:** 2026-05-24
**Lens:** Trace every user journey and API interaction. Real personas: software engineers documenting bugs, PMs writing specs, support engineers in tickets, designers sharing comps, teachers/streamers, technical writers, recruiters. Features must ELIMINATE friction users already feel, not add capability for its own sake.

## User journeys walked

1. **Engineer documents a bug** — Region capture → annotate → paste into Jira/GitHub. Today: works. Friction: long error logs exceed the viewport (scrolling capture gap); image is an attachment, not a URL (sharing-by-link gap); after pasting realizes prod data was visible (redaction gap).
2. **PM writes a spec** — Captures many UI states, assembles into a doc. Friction: no batch-export; can't drop a "set" of recent captures as one drag-and-drop bundle; library "Pinned" smart section helps but you still hand-pick one-by-one.
3. **Support engineer in a ticket** — OCR-searches old captures by quoted error text. Today: works (phase 5). Friction: cannot capture a short video/GIF of repro steps — has to describe in words or use a separate tool.
4. **Designer sharing comps** — Annotates "8px more padding here". Friction: no measurement tool (pixel ruler/distance between two points); no eyedropper to lift hex from a captured image; no rotate/flip on captures from a wrong-oriented source.
5. **Teacher / streamer** — Wants to highlight cursor or draw on the live desktop, record short GIFs of UI flows. Both gaps. Cursor highlight is cheap; live screen drawing is the same Konva canvas pinned to a click-through overlay; GIF recording is a separate beast (FFmpeg sidecar).
6. **Technical writer** — Maintains screenshots in docs. Friction: when UI changes they recapture and re-annotate by hand. Today there is no replay/recapture-this-region or annotation template reuse.
7. **Recruiter / general user** — Pastes a clipboard text item, but the source was rich HTML so Word inherits weird formatting. No "paste as plain text" affordance from the popup. Cheap fix, broadly useful.
8. **Multi-device user** — Wants the capture on phone. Cloud sync is explicitly deferred per privacy posture. But a one-tap "share via QR" (LAN-only, ephemeral) sidesteps the privacy objection — no servers, no accounts, but solves 60% of the pain.
9. **Anyone with sensitive screens** — Captured chrome contains email/account ID. Tesseract already gives us text + bounding boxes. One-tap "redact detected emails / phone numbers / credit-card-shaped strings" is high-value privacy hygiene and reuses existing OCR data.
10. **Anyone, any capture** — After capture they want a quick "share" — currently they paste into a destination. Most users paste into the same 2–3 apps. A "recent destinations" quick-share row on the post-capture toolbar (Slack, Mail, Issue tracker URL) would shave taps for the 80% case.

## Scoring philosophy

- **Product fit (1-5):** matches the existing audience B (share-friendly side project, privacy-first, local-only, no servers, no accounts) and the product identity ("better screenshots + clipboard"). A feature that contradicts the privacy posture takes a hit even if it would be valuable.
- **Reach (1-5):** how many of the 7 personas does this serve, and does it differentiate from native OS tools (macOS screenshot, Windows Snip & Sketch, Snagit, ShareX, Greenshot, Maccy, Paste, Raycast).
- **Complexity (higher = cheaper):** A 5 is "could land in one sprint per the existing plugin architecture." A 1 is "new sidecar + multi-OS native code + new permission model." This is INVERTED from intuitive complexity so higher always means better.
- **Overall value:** weighted by fit × reach with complexity as a tiebreaker. A 5/5/1/4 (high fit, high reach, expensive) usually beats 4/3/5/3 (mediocre but cheap) unless we're staring down a release crunch.

## Position

### 1. Scrolling capture (window / region)
   Fit: 5/5  Reach: 5/5  Complexity (higher=cheaper): 2/5  Value: 5/5
   Reasoning: This is the #1 missing feature in every "vs Snagit / vs Greenshot" comparison and the deferred item I most want back. Real users absolutely hit it: any error log, settings page, or chat thread that exceeds the viewport is currently un-captureable in one shot, forcing 3-4 stitched screenshots that lose continuity. Implementation cost is real (per-OS scroll synthesis + image stitching + handling animated content / sticky headers) but the feature lives entirely inside `snk-capture`'s contract. Once shipped, it reframes the product from "screenshot tool with annotation" to "the screenshot tool I never have to leave."
   User journey it solves: Journey 1 (long error logs), Journey 6 (long docs/spec pages), Journey 3 (full chat ticket history).

### 2. PII auto-redact suggestions (built on existing OCR)
   Fit: 5/5  Reach: 4/5  Complexity (higher=cheaper): 4/5  Value: 5/5
   Reasoning: Tesseract already produces text + bounding boxes per capture (phase 5). Pattern-matching for emails / phone / SSN-shaped / credit-card-shaped / IP addresses is a 200-line Rust module in `snk-ocr` (or a new `snk-redact` plugin). The annotation editor already has a `blur` shape — wiring "Suggest redactions" onto the existing blur tool is a few hundred LoC of frontend. Reuses 90% of what's already shipped. Privacy-first audience B will love this; it's the *kind* of feature that defines the product's identity against cloud-tied competitors that can't safely use AI redaction.
   User journey it solves: Journey 9 directly, Journey 1 (engineer realizes prod data was in the screenshot), Journey 3 (support engineer scrubbing customer PII).

### 3. Eyedropper + pixel-measure annotation tools
   Fit: 5/5  Reach: 3/5  Complexity (higher=cheaper): 5/5  Value: 4/5
   Reasoning: Two small additions to the Konva annotation canvas. Eyedropper reads a pixel on the underlying image and copies hex to clipboard (or appends to a callout). Pixel-measure draws a labelled line/rectangle showing pixel distance. Both are <500 LoC each, both reuse the existing canvas + tool framework, and they're table stakes for designers and UI engineers — the audience most likely to upgrade from native screenshot. High value-per-effort ratio.
   User journey it solves: Journey 4 (designer comps), Journey 6 (technical writer measuring UI), partially Journey 1 (engineer measuring the broken layout).

### 4. Paste as plain text (+ paste-as-image, paste-as-link)
   Fit: 5/5  Reach: 5/5  Complexity (higher=cheaper): 5/5  Value: 4/5
   Reasoning: The clipboard popup currently has one paste mode. Adding a modifier (Shift+Enter = paste as plain text; Cmd+Shift+Enter = paste as image of the text) is universally useful and trivial to implement — strip rich formatting before `arboard.set()`. Anyone who pastes from a browser into a doc has hit this. Cheap, ships next sprint, broadens appeal to literally every clipboard user, and matches what every premium clipboard manager (Paste, Maccy, Alfred) offers.
   User journey it solves: Journey 7 (recruiter), and 100% of the personas at some point.

### 5. Quick-share targets on the post-capture toolbar
   Fit: 4/5  Reach: 4/5  Complexity (higher=cheaper): 4/5  Value: 4/5
   Reasoning: Today the post-capture toolbar is Annotate / Copy / Save / Discard. Add a configurable row of 2-3 "send to" buttons — copy as markdown image link, save to a `~/screenshots-for-x` folder, open in default image editor, drag-out source for drag-and-drop into apps. The data is already on disk; this is just routing. Reduces the "capture → switch app → paste → format" loop to one click for the 80% case. Lower-cost than scrolling capture but higher-touch UX work to get the picker right.
   User journey it solves: Journey 1, Journey 2 (PM batching into doc), Journey 3 (support engineer).

### 6. macOS Vision OCR (replaces Tesseract on Mac)
   Fit: 4/5  Reach: 4/5  Complexity (higher=cheaper): 3/5  Value: 4/5
   Reasoning: Tesseract works but Apple Vision OCR is dramatically better on UI screenshots (the dominant capture type for snapper-keeper). Multi-language, handwriting, way better confidence on small text. The architecture supports this trivially — `snk-ocr` already has a queue interface; you swap the backend per-OS at runtime. On Mac builds, ship Vision; on Windows, keep Tesseract. The Mac install also gets ~50 MB smaller (no Tesseract sidecar binary). High user-visible quality improvement for moderate engineering cost. Direct quality win that compounds the value of OCR search (phase 5 effort).
   User journey it solves: Journey 3 (support engineer's OCR results actually find the thing), all journeys with non-English captures.

### 7. Video / GIF recording (region or window, capped duration)
   Fit: 4/5  Reach: 5/5  Complexity (higher=cheaper): 1/5  Value: 4/5
   Reasoning: The single most-requested adjacent capability. Streamers, teachers, support engineers, and bug reporters all need short-form screen recording. FFmpeg sidecar + per-OS capture → encoder → write to library with the same metadata model. Big-scope: muxing, codec choices, file size warnings, audio (in or out?), pause/resume, captions. The complexity score is 1 because this is bigger than any single phase to date. Bumping into the top half despite the cost because the reach is enormous — it makes the product viable in workflows that today force users to Loom / CleanShot / OBS.
   User journey it solves: Journey 5 (teacher/streamer), Journey 3 (support repro steps), Journey 1 (engineer recording bug repro).

### 8. Recent destinations / drag-out source from library
   Fit: 5/5  Reach: 3/5  Complexity (higher=cheaper): 5/5  Value: 3/5
   Reasoning: Many users want to drag a thumbnail from the library window straight into Slack / Mail / Notion / a file dialog. Tauri supports drag-out via `setDragImage` / `startDrag` patterns. Pure frontend addition, no plugin changes. Low cost, satisfying UX. Reach is medium because mouse-heavy users get the most value; keyboard-heavy users will keep using copy.
   User journey it solves: Journey 2 (PM dragging multiple into doc), Journey 4 (designer dragging into Figma).

### 9. "Recapture this region" / region replay
   Fit: 4/5  Reach: 2/5  Complexity (higher=cheaper): 4/5  Value: 3/5
   Reasoning: For technical writers maintaining docs. Capture stores monitor + bounds metadata already. Add a "recapture" affordance on the library card: "capture the same region on this monitor again" so the doc screenshot can be refreshed without re-measuring. Bonus mode: keep annotations on a separate layer so a new background image inherits the same arrows/callouts (template reuse). Niche but it makes us the go-to tool for one specific job (docs maintenance) that no competitor handles well.
   User journey it solves: Journey 6 directly, Journey 4 partially (designers iterating on the same screen).

### 10. LAN ephemeral share (QR-coded local web link)
   Fit: 3/5  Reach: 4/5  Complexity (higher=cheaper): 3/5  Value: 3/5
   Reasoning: A pragmatic sidestep of the explicitly-deferred cloud sync. The app already has the file; it spins up a localhost HTTPS server bound to the current LAN interface, serves the image at a one-time URL, and renders a QR code on screen. Phone scans, gets the image, link dies in 5 minutes. No accounts, no servers, no telemetry. Stays consistent with the privacy posture. Engineering: rust http server (axum or tiny-http), self-signed cert dance, QR rendering. Risk is the cert UX (devices unfamiliar with the LAN cert); mitigation is a `http://` mode for trusted networks.
   User journey it solves: Journey 8 (multi-device), Journey 5 (teacher sharing a snap to a student in class), Journey 3 (support sending a screenshot to a customer who can't access company chat).

## Alternatives Proposed

Considered and rejected from the seed list / brainstorming:

- **Cloud sync** — Explicitly rejected by the design (audience B, no servers). My #10 (LAN ephemeral share) covers 60% of the use case at 5% of the cost without the privacy hit. Real cloud sync would also require accounts, billing, encryption-at-rest, key management — out of scope for a side project.
- **App Store / Microsoft Store distribution** — Audience B is share-friendly side project. Store sandboxes would block the OS integrations (global hotkeys, accessibility) that the product depends on. Not just out of scope; actively destructive to the product. Drop entirely.
- **Linux support** — Genuinely valuable for the developer audience (Journey 1 is heavily Linux), but it's a multi-month port given xcap/arboard quirks and tesseract packaging on Linux distros. As a side project staffed by one person, it crowds out 3-4 features that benefit ALL existing users.
- **Synced clipboard between devices** — Same family as cloud sync; same rejection. The LAN share idea can extend to clipboard if there's signal.
- **Plugin / extensibility API for end users** — Interesting for power users but the actual API surface is hard to design and creates a long-term support tax. Real users would rather have native scrolling capture than to build their own. Defer indefinitely.
- **Crash report uploads (self-hosted Sentry)** — Privacy posture says no for v1. Logs are local; user can attach them to a bug report manually. Reconsider only if the app gains real installs and crash rates become unknowable.
- **EV code-signing cert** — Operational, not a user-facing feature. Reconsider only when SmartScreen friction becomes a measurable funnel drop. Doesn't compete with the user-visible features above.

Runner-ups worth mentioning but didn't make the top 10:

- **Cursor highlight / spotlight on capture** — Cheap, designer-friendly, but solved by the existing annotation tools after the fact. Lower ROI than the eyedropper/measure pair (#3).
- **Live desktop annotation (draw on the live screen)** — Teachers and streamers want this. Same Konva canvas in a click-through transparent window. Cost is medium (need click-through window flag on both OSes, frame syncing). Borderline #11.
- **Bulk operations in library (multi-select tag, multi-delete, multi-export-as-PDF)** — Quality-of-life. Half-day each. Genuinely missing today.
- **Pinned-text "snippet" expansion** — Type a shortcut, expand to a pinned clipboard item. Adjacent to existing clipboard plugin but distinct enough to need a watcher state machine. TextExpander territory; out of scope for v8.
- **Annotation templates** — Save a set of annotations as a reusable template (e.g. branded watermark + signature arrow style). Mostly for designers and product marketers.

## Risks Identified

- **Scope creep risk** — Video/GIF recording (#7) is huge by snapper-keeper standards. If we take it on, it crowds out the next 3 phases. We should agree explicitly: scrolling capture is one phase, video is a separate phase, both are big. Don't combine.
- **"Just add AI" temptation** — There will be pressure to add cloud-AI features (LLM summarize the screenshot, smart-tag from image content). They would conflict with the no-servers / no-telemetry stance. Hold the line.
- **Cross-platform divergence drift** — macOS Vision OCR (#6) is a great call but it's the first place we have a per-OS feature implementation behind a shared interface. Once we cross that line for OCR, the temptation to do it for capture (scrolling), recording (#7), accessibility, etc. compounds. Make sure the plugin interface stays clean.
- **Personal-data redaction false negatives** (#2) — If we say "we redact emails" and miss a non-standard format, users will trust the badge and leak data. Frame the feature as "suggestions, you confirm" with a non-dismissable confirmation step. Never auto-apply.
- **Solving the wrong problem first** — Quick-share targets (#5) feel polished but most users have one or two destinations. Risk of overbuilding a destination picker when "copy as markdown link" + "save as file in folder X" covers 80%.
- **Cloud-sync gravitational pull** — Once #10 (LAN share) lands, users will request "now do it across the internet". Decide upfront whether LAN share is a permanent feature or a stepping stone, and tell users clearly.

## Open Questions

1. What's the actual install base / retention data telling us about which features are most-used today? Without that, my ranking is informed by personas-I-can-imagine rather than personas-we-have. Should we instrument anonymous usage *counts* (not content) for a month before deciding? (Tension: privacy posture says no telemetry — but bare counts might be acceptable if opt-in.)
2. Is "share-friendly side project" still the audience target? If the product is trending toward "paid prosumer tool" or "open-source community tool" or "polished personal utility," the ranking changes. #7 (video) only pencils out as a side-project investment if the answer is "personal utility I love using" vs "I want users."
3. Is there appetite for an *adjacent* product surface (e.g. a recordings library) or do all features need to fit in the current single-app paradigm? Video/GIF (#7) might be cleaner as a sibling app sharing `snk-library`.
4. What's the budget for cross-platform feature divergence? macOS Vision OCR (#6) crosses that line. If the answer is "minimize", drop #6 and lean harder on scrolling capture and PII redaction.
5. How important is "look-and-feel parity with native OS tools"? If high, the post-capture toolbar (Quick-share, #5) becomes more important. If low, the library window can do all the heavy lifting.
6. Do we want any feature to require an account, ever? My ranking assumes "no, ever," which is why LAN share (#10) replaces cloud sync. If a one-account anchor is acceptable, the calculus changes — but I'd push back hard.
