# Phase 10 — OCR Surfaces (Vision + Win.OCR + PII Redact + Text Actions) — Design

Date: 2026-05-25
Author: Eric Hartye + Claude (Opus 4.7)
Status: Draft (awaiting Eric's review)

Covers issues:

- [#31 — Bundle Tesseract for macOS or surface missing-dependency banner (HIGH)](https://github.com/ehartye/snapper-keeper/issues/31) — resolved by adopting Vision (issue option 3)
- [#41 — Sandbox Tesseract sidecar (job object / sandbox-exec) + per-invocation timeout (MED)](https://github.com/ehartye/snapper-keeper/issues/41) — obsoleted (no sidecar)
- [#40 — Bounded queue + startup sweep for captures missing ocr_text (MED)](https://github.com/ehartye/snapper-keeper/issues/40) — partially obsoleted (queue pressure rationale weakens with in-process OCR); reassess at plan time

Phase 10 trio per research synthesis ([`docs/superpowers/research/2026-05-24-next-10-features/SYNTHESIS.md`](../research/2026-05-24-next-10-features/SYNTHESIS.md), Phase 10):

- macOS Vision OCR backend
- PII auto-redact suggestions
- Post-capture "Text Actions" OCR overlay

This design **expands Phase 10** beyond the synthesis by adopting Windows.Media.Ocr on Windows in addition to Vision on Mac, so Tesseract can be removed from both platforms. The user has set a hard requirement: **100% Tesseract cleanse, no tech debt**.

---

## 1. Goals, non-goals, scope

### Goals

1. Replace the Tesseract sidecar with OS-native OCR on both platforms: Apple Vision on macOS (≥14, `VNRecognizeTextRequest` accurate mode); Windows.Media.Ocr on Windows 10/11.
2. Capture per-word bounding boxes + confidence as a first-class output of OCR, persisted alongside text.
3. Auto-detect a fixed set of PII categories (email, phone, credit card with Luhn validation, US SSN, IP address, common API keys/tokens) and surface confirm-to-redact suggestions everywhere a capture is viewed.
4. Render an Apple-Live-Text-style selectable text overlay over captures, with a "Copy text" action, available in the post-capture toolbar, annotation editor, and library viewer.
5. Eliminate every Tesseract artifact from the repository — sidecar, bundled tessdata, `SNK_TESSERACT_PATH` env override, retry logic, CI bundling, README install instructions, related tests.

### Non-goals

- Re-OCRing existing pre-Phase-10 captures (pre-release, no install base — old data has zero users).
- Named-entity recognition for personal names or street addresses (no NER, no ML).
- A user-toggleable opt-in Tesseract fallback. Hard-cleanse means zero Tesseract code paths.
- Cross-OS OCR-result symmetry. Mac Vision and Windows.Media.Ocr produce different text on the same image; we accept that.
- Multi-language picker. v1 ships with platform defaults (Vision auto-detect; WinOcr uses installed Windows display language; reverts to English if neither yields).
- Library-wide filter UI for "captures with detected PII" — the `pii_spans` table will support this query, but the filter UI ships in a later phase.

### Hard requirements

- 100% Tesseract cleanse. Spec self-review fails if any Tesseract reference (code, docs, CI, bundle) remains. Becomes a verification step in the implementation plan.
- No bundled binaries added by this phase. Net change: −1 sidecar (Tesseract), 0 added.
- All three Phase 10 features ship in one cluster of PRs aimed at a single `v0.2.0` release.

### Deferred to plan-time

- Exact regex patterns and Luhn implementation (categories locked here; specific regexes finalized with test fixtures at plan time).
- WinOcr per-word confidence verification spike (research watch-item #1).
- WinOcr package-identity-from-NSIS spike (research watch-item #2).
- Windows language pack install affordance UX (acknowledged dependency; exact UI is plan-time).

---

## 2. ADR — Engine matrix

### Decision

Adopt OS-native OCR on both platforms: **Apple Vision (`VNRecognizeTextRequest`) on macOS, Windows.Media.Ocr on Windows.** Remove Tesseract entirely from the project.

### Context

Today `snk-ocr` shells out to a Tesseract subprocess on both platforms ([`crates/snk-ocr/src/sidecar.rs:122`](../../../crates/snk-ocr/src/sidecar.rs)). Tesseract is bundled on Windows; on macOS users must install via Homebrew or accept silent OCR failure (issue #31). A research spike (separate transcript) compared Vision / WinOcr / Tesseract on language coverage, screenshot quality, per-word bounds, speed, API stability, Rust integration, distribution, and license.

### Trade-offs

**Loss with Tesseract removal:**

- **Language coverage.** Tesseract ships ~130 trained languages; Vision covers ~16–18 (macOS 14+) or ~26 (macOS 15+ via `RecognizeDocumentsRequest`); Windows.Media.Ocr covers ~25, dependent on installed language packs. **Tesseract's exclusive zone is RTL (Arabic, Hebrew) and Indic (Devanagari, Tamil, Bengali, Burmese, Thai).**
- **Cross-OS search consistency.** Mac and Windows users OCRing the same image will get slightly different text. Only matters if libraries are synced across machines, which snapper-keeper does not do.
- **Test determinism.** Tesseract is a pinned binary. OS-native OCR can shift between point releases. Snapshot-style OCR tests get harder; we mitigate with tolerance-based fixture assertions.

**Gain:**

- **Quality on screenshots.** Tesseract was trained on document scans; Vision was trained on iPhone photos of real-world UI text. Vision is materially better on anti-aliased low-DPI panel text — the exact content snapper-keeper captures.
- **Speed.** Vision ~150–300 ms on Apple Silicon vs Tesseract ~500–2200 ms with subprocess spawn overhead.
- **Native per-word bounds + confidence.** Both Vision and WinOcr return bounding boxes natively. (WinOcr confidence granularity is a watch-item; see §6.)
- **Bundle / signing simplification.** ~35 MB off the macOS installer; ~50 MB off the Windows installer. Two signing steps removed. Issue #31 disappears for free.
- **Identity fit.** Local-first, native-first, no bundled-binary tax. Matches the project posture.
- **App store posture.** Mac App Store sandboxing makes a Tesseract sidecar materially harder to ship; Vision is the supported path. Microsoft Store similarly. Not a goal of this design, but the door stays open.

### Watch items (research-confirmed, not ADR-blocking)

These become discovery spikes that precede implementation (§6):

1. WinOcr per-word `OcrWord.Confidence` granularity.
2. WinOcr `OcrEngine` construction from a non-MSIX (NSIS) Tauri bundle.
3. `objc2-vision` FFI smoke against current Xcode SDK.

### Consequences

- Phase 10 trio ships in ~4 weeks (synthesis estimated ~3 for Mac-only Vision; we accept +1 week for the Windows-symmetric path).
- Future support for RTL / Indic / Hebrew languages requires re-introducing Tesseract as an opt-in user-installed engine. Acceptable: no such user exists today; revisit when one surfaces.

### Alternatives rejected

- **Mac Vision, Windows keeps Tesseract.** Smaller scope, faster ship, but leaves the Tesseract sidecar + signing + bundling on Windows forever. Violates the user's "no tech debt" requirement.
- **Keep Tesseract everywhere, add Vision as a quality boost.** Maximum code-path count. Carries the loss-categories of Tesseract removal without gaining any of the wins.

---

## 3. Engine abstraction

### `OcrBackend` trait

`snk-ocr` exposes a single platform-conditional backend behind a trait:

```rust
pub struct OcrResult {
    pub text: String,
    pub words: Vec<OcrWord>,
    pub language: String,
    pub confidence: f64,
}

pub struct OcrWord {
    pub text: String,
    pub bbox: BBox,
    pub confidence: f64,
    pub line: u32,
}

pub struct BBox { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

pub trait OcrBackend: Send + Sync {
    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError>;
    fn name(&self) -> &'static str;
    fn engine_version(&self) -> String;
}
```

Coordinate convention: normalized 0..1, origin top-left, applied uniformly downstream. Vision's bottom-left coords are converted at the FFI boundary; WinOcr's pixels are normalized against capture dimensions.

### `VisionBackend` (macOS)

- Implementation: `objc2-vision` (0.3.x, actively maintained per research spike). Fallback if FFI regresses: a 50-line Swift sidecar binary.
- Uses `VNRecognizeTextRequest` with `recognitionLevel = .accurate` and `automaticallyDetectsLanguage = true`.
- API choice: `VNRecognizeTextRequest` (16–18 languages, macOS 14+ baseline). Newer `RecognizeDocumentsRequest` (26 languages, macOS 15+) deferred — keeps minimum macOS lower.
- Per-word bounding boxes via `boundingBoxForRange(_:)`.
- `engine_version()` → `"Vision (macOS {major.minor.patch})"`.

### `WinOcrBackend` (Windows)

- Implementation: `windows-rs` crate calling `Windows.Media.Ocr.OcrEngine`.
- Construction order: `OcrEngine::TryCreateFromUserProfileLanguages()` → if empty, `TryCreateFromLanguage("en-US")` → if still empty, return typed `OcrError::NoRecognizerLanguage`.
- Per-word data via `OcrResult → OcrLine[] → OcrWord[]`.
- If the implementation spike confirms `OcrWord.Confidence` is unavailable, broadcast per-line confidence to all words in the line and log the asymmetry.
- `engine_version()` → `"Windows.Media.Ocr ({OS build})"`.

### Backend selection

`snk-ocr::plugin::init()` constructs the backend at plugin setup based on `cfg(target_os = "...")`. No platform-independent fallback — if backend construction fails, OCR is disabled for the session and the failure surfaces as a typed error to the UI.

### `OcrQueue` changes

- Same `mpsc::unbounded_channel<OcrJob>` from today.
- `spawn_blocking` retained for both backends. Vision FFI is sync at ~150–300 ms; WinOcr FFI bridges an async UWP API. Either could block the tokio runtime without `spawn_blocking`. Defensive default matches today's Tesseract worker shape and avoids reasoning about runtime flavors.
- Retry-with-backoff (`run_tesseract` 3-attempt loop) **deleted**. Native OCR rarely transient-fails; typed errors propagate instead.
- On success the worker:
  1. Writes `ocr_text` (with `words_json` and `engine` populated).
  2. Re-indexes capture into FTS (unchanged).
  3. Emits new `ocr:ready` event with `{capture_id}` payload. `snk-pii` and any UI surface listening subscribe to this.

---

## 4. Data model

### Migration

Single forward-only migration. Pre-release, no rollback story.

`0007_phase10_ocr_bounds_and_pii.sql` (next available migration number — verify against `crates/snk-library/migrations/` at plan time):

```sql
-- Per-word bounds + engine version on existing OCR table.
ALTER TABLE ocr_text ADD COLUMN words_json TEXT;
ALTER TABLE ocr_text ADD COLUMN engine     TEXT NOT NULL DEFAULT '';

-- PII detection spans.
CREATE TABLE pii_spans (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    capture_id   TEXT    NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
    category     TEXT    NOT NULL,
    matched_text TEXT    NOT NULL,
    bbox_x       REAL    NOT NULL,
    bbox_y       REAL    NOT NULL,
    bbox_w       REAL    NOT NULL,
    bbox_h       REAL    NOT NULL,
    confidence   REAL    NOT NULL,
    redacted_at  INTEGER,
    dismissed_at INTEGER,
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_pii_spans_capture ON pii_spans(capture_id);
CREATE INDEX idx_pii_spans_pending ON pii_spans(capture_id)
    WHERE redacted_at IS NULL AND dismissed_at IS NULL;
```

`words_json` shape:

```json
[
  {"text": "Hello", "bbox": [0.10, 0.05, 0.08, 0.04], "confidence": 0.97, "line": 0},
  {"text": "world", "bbox": [0.19, 0.05, 0.08, 0.04], "confidence": 0.95, "line": 0}
]
```

Stored as a JSON blob rather than a separate `ocr_words` table because the only access pattern is "fetch all words for one capture" — never cross-capture word queries. SQLite JSON1 functions remain available if that changes.

### `pii_spans` lifecycle

| State | `redacted_at` | `dismissed_at` |
|---|---|---|
| Pending | NULL | NULL |
| Redacted (user confirmed) | timestamp | NULL |
| Dismissed (user rejected) | NULL | timestamp |

Re-scan semantics: if the scanner runs again on a capture with existing spans, it compares `(category, matched_text, bbox)` against existing rows; matches are not re-inserted, novel matches append as new pending rows.

### Crate layout

```
crates/
  snk-library/    (unchanged module structure — adds pii.rs module + words_json + engine fields)
  snk-ocr/        (rewritten — engine trait + Vision/WinOcr backends; sidecar.rs deleted)
  snk-pii/        (NEW — pure plugin, no UI)
  snk-capture/    (unchanged)
  snk-clipboard/  (unchanged)
  snk-hotkeys/    (unchanged)
  snk-popup/      (unchanged)

packages/
  snk-library/    (TS bindings — adds OcrWord, PiiSpan types + redact/dismiss commands)
  snk-ocr/        (TS bindings — adds ocr:ready event)
  snk-pii/        (NEW — TS bindings)

app/src/
  components/
    TextOverlay.tsx
    TextOverlay.css
    PiiBadge.tsx
    PiiReviewSheet.tsx
  windows/
    capture-toolbar/  (extended)
    editor/           (extended)
    library/          (extended)
```

### Plugin commands + events

**`snk-ocr`** adds:

- Event: `ocr:ready` payload `{ capture_id: string }`.
- Command: `get_ocr_words(capture_id) -> OcrWord[]`.

**`snk-pii`** (new):

- Listens: `ocr:ready`.
- Event: `pii:scanned` payload `{ capture_id: string, pending_count: number }`.
- Commands:
  - `list_pii_spans(capture_id) -> PiiSpan[]` — all states.
  - `list_pending_pii(capture_id) -> PiiSpan[]` — pending only.
  - `redact_pii(span_id) -> PiiSpan` — applies pixel blur, marks `redacted_at`.
  - `dismiss_pii(span_id) -> PiiSpan` — marks `dismissed_at` without modifying image.

Pixel-blur reuses the existing Phase 3 annotation editor blur primitive. Plan-time decides whether that primitive lives in `snk-library` or `snk-capture`.

### Architectural invariants preserved

All four `CLAUDE.md` rules:

- ✓ One plugin per feature (`snk-ocr`, `snk-pii` are separate features).
- ✓ All persistence via `snk-library`.
- ✓ No cross-plugin internal imports (`snk-pii` listens to `ocr:ready`, never imports `snk-ocr`).
- ✓ Capture stays fire-and-forget.

---

## 5. PII module

### Categories

Six categories. Pattern + post-filter discipline.

| Category | Pattern (sketch) | Post-filter | Pattern conf |
|---|---|---|---|
| `email` | `[\w.+-]+@[\w-]+\.[\w.-]+` | TLD must be in IANA list | 0.90 |
| `phone` | E.164 + common US formats | 10–15 digits after stripping separators | 0.70 |
| `credit_card` | `\b(?:\d[ -]*?){13,19}\b` | **Luhn checksum** | 0.95 |
| `ssn` | `\b\d{3}-\d{2}-\d{4}\b` | Reject `000`, `666`, `9xx` (SSA invalid ranges) | 0.80 |
| `ip` | IPv4 dotted-quad + IPv6 | Octet range / v6 syntax check | 0.60 |
| `api_key` | Union of: Stripe (`sk_(test\|live)_[a-zA-Z0-9]{24,}`), AWS (`AKIA[0-9A-Z]{16}`), GitHub PAT (`ghp_[a-zA-Z0-9]{36}`), Slack (`xox[bpars]-[a-zA-Z0-9-]+`), JWT (`eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+`) | Per-pattern length/prefix sanity | 0.95 |

Concrete regexes finalize at plan time with per-category test fixtures (including negative controls).

### Scanner flow

1. `snk_pii::scanner::scan(ocr_text, ocr_words) -> Vec<PiiCandidate>` runs all matchers against `ocr_text.text`.
2. For each match, walk `ocr_words` to find which words overlap the match's character range; compute a union bbox.
3. `confidence = min(avg(word.confidence over overlapping words), pattern_confidence)`.
4. Drop candidates below `confidence < 0.5`.
5. De-dupe against existing `pii_spans` for the same capture.
6. Persist surviving candidates as pending rows.
7. Emit `pii:scanned` with `{ capture_id, pending_count }`.

### Worker

`snk-pii` runs an `mpsc` worker mirroring `snk-ocr`'s queue pattern. Subscribes to `ocr:ready`, enqueues scan jobs, worker processes them. Fire-and-forget. Scan latency is sub-millisecond; the worker is for consistency, not performance.

### Engine-version tracking

`ocr_text.engine` records which backend produced the text (e.g., `"Vision (macOS 15.2)"`). A future "rescan with newer engine" tool can use this to invalidate stale data.

### Pixel redaction

`redact_pii(span_id)` reads the capture file, applies the Phase 3 blur primitive to the bbox region, writes back. **Destructive by default** — no original-backup sidecar file. Matches "no tech debt" framing.

### Test fixtures

`crates/snk-pii/tests/fixtures/` ships synthetic text + word arrays per category, plus negative-control fixtures (version strings that look like IPs, 16-digit non-CC numbers that fail Luhn, etc.). Pure data, no real PII committed.

---

## 6. UI surfaces

### Shared primitives

**`<TextOverlay image words selectionMode />`**

- Absolutely positioned overlay matching the image's render box.
- One transparent `<div role="text">` per word, positioned via normalized `bbox` × render dimensions.
- `selectionMode="hover"` — cursor turns to text I-beam over word regions; click-drag selects; native browser text selection (so Ctrl+C / Cmd+C just works).
- `selectionMode="off"` — overlay still renders but inert.
- Resize-aware via `ResizeObserver`.

**`<PiiBadge count onClick />`**

- Small pill: `⚠ N items detected`. Uses existing semantic warning token (no new color).
- Hidden if `count === 0`.

**`<PiiReviewSheet spans onRedact onDismiss />`**

- Per-span row: category icon, masked preview (`vis••@****.com`), bbox highlight on a thumbnail.
- Per-row `[Redact]` / `[Dismiss]` buttons. **No bulk "redact all" in v1** — explicit per-item confirmation is the safety property the research synthesis emphasized.
- Rows animate out after action; sheet closes when zero pending remain.

### Surface 1 — Post-capture toolbar (Phase 2)

- "Copy text" action, disabled with spinner until `ocr:ready`; then enabled. Click copies `ocr_text.text` to clipboard (skips `snk-clipboard` watcher per existing convention).
- `<PiiBadge>` appears in right cluster after `pii:scanned` fires with `pending_count > 0`. Click opens `<PiiReviewSheet>` as a modal anchored to the toolbar.
- No `<TextOverlay>` — floating UI is too small for selectable text.

Latency: OCR + scan typically completes before the user reads the toolbar. Worst case Copy text is briefly disabled and the badge appears a beat later.

### Surface 2 — Annotation editor (Phase 3)

- `<TextOverlay selectionMode="hover">` layered on top of image, below the annotation overlay. New toolbar toggle (text-cursor icon) enables/disables overlay so drawing doesn't fight text selection. Default: ON for fresh editor open, OFF after user starts drawing.
- `<PiiBadge>` in editor top bar. Opens `<PiiReviewSheet>` as a right-docked panel (not a modal — user wants to compare with the canvas).
- Confirming redaction in the editor adds a non-destructive blur annotation to the edit stack (editor blur is layered, baked on export). PII redaction from toolbar / library is the destructive path defined in §5.

### Surface 3 — Library viewer (Phase 6)

- Capture tile in grid: subtle PII badge corner dot (no count) if pending spans exist. Hover reveals count.
- Detail view:
  - `<TextOverlay selectionMode="hover">` over preview image.
  - Persistent "Copy text" button in detail toolbar.
  - `<PiiBadge>` opens `<PiiReviewSheet>` as a modal over the detail view.
- Library list/filter UI for "captures with pending PII" is non-goal for v1.

### Settings surface

Settings → About panel. (Open issue #36 targets the About panel; if it hasn't shipped by Phase 10, building it becomes a precursor task here.)

- **OCR engine** row — shows `name() + engine_version()`.
- **PII categories** row — read-only list of the six active detectors. No per-category disable toggle in v1.
- **Last OCR engine error** row — surfaces typed error if backend is unavailable this session, with `[Open Windows Settings]` shortcut where applicable.

### Empty / error states

- No detectable text → Copy text disabled with tooltip "No text detected"; no overlay; no badge.
- OCR backend unavailable → Copy text shows "OCR unavailable"; About panel shows typed error; no badge; no banner spam.
- PII scan failed → log + skip; no badge; treated as "no PII found."

---

## 7. Error model, testing, build/release

### Error enums

Two new types, following `LibraryError` / `CaptureError` convention (serde-tagged `kind`, variant fields never named `kind`):

```rust
pub enum OcrError {
    BackendUnavailable { reason: String },
    NoRecognizerLanguage { detail: String },
    Recognize { detail: String },
    ImageLoad { path: String, detail: String },
}

pub enum PiiError {
    OcrMissing { capture_id: String },
    Persist { detail: String },
}
```

Both cross IPC as typed errors; UI branches on `kind`, never string-matches.

### Testing strategy

**Unit (Rust):**

- `snk-ocr`: backend trait conformance test runs against a small fixture image with both backends conditional-compiled. Mac CI exercises Vision; Windows CI exercises WinOcr; both verify per-word bounds are returned and non-empty.
- `snk-pii`: per-category fixture suite (text + word arrays → expected spans). Negative-control fixtures for false-positive suppression.
- OCR pipeline snapshot test: a committed `hello_world.png`, asserted text + word count + approximate bbox positions (tolerance-based to survive OS-engine drift).

**Integration:**

- `crates/snk-ocr/tests/integration_test.rs` — replaces today's Tesseract test.
- `crates/snk-pii/tests/integration_test.rs` — end-to-end: synthetic OCR result → scanner → DB write → query back.

**Manual end-to-end (documented in implementation plan):**

- Capture a screenshot containing each PII category; verify badge appears and review sheet shows correct spans.
- Test Mac (Vision) and Windows (WinOcr) separately given backend asymmetry.

**Smoke test impact:** Existing Windows interactive-desktop constraint (`CLAUDE.md`) unchanged. Smoke validates end-to-end on an interactive desktop only; CI's `build-app` job continues validating compilation across all three OSes.

**In-house OCR quality spike (one-shot):** Implementation-plan discovery task. 30–50 representative SK captures (UI screenshots, mixed fonts, low-res panel text in en/ja/zh) through Vision and WinOcr; eyeball quality vs prior Tesseract baseline. Goal: confirmation, not formal benchmark. Material regression → revisit before merging.

### Discovery spikes (precede implementation)

1. **WinOcr per-word confidence** — 50-line throwaway binary calling `OcrEngine.RecognizeAsync`; dump `OcrWord.Confidence`. Document if line-only.
2. **WinOcr from NSIS bundle** — same binary, packaged via NSIS, run on clean Windows VM with no package identity. Confirm `OcrEngine::TryCreateFromUserProfileLanguages` succeeds.
3. **`objc2-vision` smoke** — confirm FFI builds clean against current Xcode SDK, returns observations, exposes per-word bounds via `boundingBoxForRange`.

All three produce throwaway code that informs implementation but doesn't merge. Any blocker → design returns to drawing board before production code lands.

### Build + release impact

**Removed:**

- `release.yml`: Windows Tesseract bundle step (lines ~66–76 per issue #31).
- `crates/snk-ocr/build.rs`: deleted unless it holds non-Tesseract logic (verify at plan time).
- `app/src-tauri/tauri.conf.json`: `bundle.resources` entries for `tesseract/**`.
- `app/src-tauri/tesseract/` directory.

**Added:**

- `objc2-vision` dep (Mac target only via `[target.'cfg(target_os = "macos")'.dependencies]`).
- `windows` crate features for `Media_Ocr`, `Globalization`, `Graphics_Imaging`, `Storage_Streams` (Windows target only).
- `snk-pii` crate added to workspace members in root `Cargo.toml`.
- `packages/snk-pii/` TS bindings package.

**Bundle size delta:** macOS −35 MB, Windows −50 MB. Vision and WinOcr add zero.

**objc2 workspace dedupe check** (per saved memory `reference_objc2_workspace_dedupe.md`): before pinning `objc2-vision`, run `cargo tree | grep objc2` and verify alignment with arboard/tao/muda transitive versions (objc2 0.6, app-kit 0.3 at last check). If `objc2-vision` requires a newer major, upgrade the whole `objc2` family in one commit.

**Code signing:** no new bundled binaries → no new identities, no new notarization steps. Release pipeline gets one signing step *simpler* on each platform.

**SBOM:** `cdxgen` (per `reference_cdxgen_for_pnpm_sboms.md`) picks up `objc2-vision` + `windows` crate additions automatically; Tesseract drops out.

### Migration discipline

Single forward-only migration. Pre-release means no migration-rollback story ships. Migration runs on app startup via existing `snk-library` machinery (Phase 1).

---

## 8. Tesseract cleanse — verification checklist

Verification step in the implementation plan: every item below must be removed or updated. Plan-level commit cannot merge until this checklist passes.

- [ ] `crates/snk-ocr/src/sidecar.rs` — deleted entirely
- [ ] `crates/snk-ocr/src/lib.rs` — `pub mod sidecar;` removed
- [ ] `crates/snk-ocr/build.rs` — deleted unless non-Tesseract logic justifies retention
- [ ] `crates/snk-ocr/Cargo.toml` — `which`, `serial_test`, `tempfile` removed if only used by sidecar tests
- [ ] `app/src-tauri/tauri.conf.json` — `bundle.resources.tesseract/**` entries removed
- [ ] `app/src-tauri/tesseract/` — directory deleted from disk
- [ ] `.github/workflows/release.yml` — Windows Tesseract bundling step removed (lines ~66–76)
- [ ] `README.md` — Tesseract install instructions removed (lines ~24–27, ~56)
- [ ] `CLAUDE.md` — Tesseract setup hint removed if present; phase status table updated
- [ ] `crates/snk-ocr/tests/integration_test.rs` — Tesseract-specific tests replaced with Vision/WinOcr equivalents
- [ ] `SNK_TESSERACT_PATH` env var — `git grep` for all references and remove
- [ ] Issues #31, #41, #40 — closed (or reassessed for #40) with closing comment linking spec and PRs

**Post-implementation check:** `pnpm tauri build && git grep -i tesseract` must return zero matches outside `docs/superpowers/specs/`, `docs/superpowers/plans/`, and `docs/superpowers/research/` (historical design / research docs are allowed; live code, CI, installer, and runtime docs must be clean).

---

## 9. Implementation sequencing

Cluster of PRs, single `v0.2.0` release target. Final sequencing decided in the writing-plans skill, but expected shape:

| PR | Scope |
|---|---|
| **PR-1 (precursor)** | About panel (issue #36) if not already shipped. Discovery spikes 1–3 (throwaway code). |
| **PR-2 (foundation)** | `OcrBackend` trait + Vision + WinOcr backends. Migration `0007`. `ocr:ready` event. **Includes Tesseract cleanse.** |
| **PR-3 (PII plugin)** | `snk-pii` crate. PII scanner + worker + commands. `pii:scanned` event. Per-category fixtures. |
| **PR-4 (UI surfaces)** | `<TextOverlay>`, `<PiiBadge>`, `<PiiReviewSheet>`. Wired into post-capture toolbar, editor, library viewer. About panel rows. |

PR-2 is the largest; it can split if the Vision and WinOcr backends grow independent enough to warrant separate review.

---

## 10. Open questions / known unknowns

- **WinOcr per-word confidence granularity** — spike 1 answers; design accepts either outcome (line-broadcast on regression).
- **WinOcr package-identity from NSIS** — spike 2 answers; if it requires MSIX we revisit (would force Tauri bundler change or a small WinRT host shim).
- **Whether the existing blur primitive is exposable from `snk-pii`** — plan-time question; primitive currently lives in the annotation editor. May need a small refactor to share it.
- **About panel — exists or needs building?** — plan-time check on issue #36 status.
- **Migration number `0007`** — verify highest existing migration at plan time.
- **Minimum macOS version** — the project does not currently pin a minimum (no `minimumSystemVersion` in `tauri.conf.json`). Adopting Vision via `objc2-vision` forces a de facto minimum of macOS 14. Plan-time decision: pin the minimum explicitly in `tauri.conf.json` so installers gate appropriately, and document in README. If the `objc2-vision` version we pull requires macOS 15+, raise the minimum accordingly.
- **Editor right-docked panel** — the annotation editor today does not (as far as the design knows) ship any right-docked panel component. `<PiiReviewSheet>` in the editor would be the first. Plan-time check: confirm whether a panel primitive exists or needs introduction; if introduction, that's a small precursor task.
- **Pre-existing `ocr_text` rows in dev databases** — the migration adds `engine TEXT NOT NULL DEFAULT ''`. Any dev-database row that pre-dates the migration will have `engine = ''`. Pre-release means no user databases exist, so no production impact. Dev databases can be wiped or left with empty engine values without consequence.

---

## 11. Acceptance criteria

This design is implemented correctly when:

1. `git grep -i tesseract` returns zero matches in `crates/`, `app/`, `packages/`, `.github/`, and `README.md`.
2. Bundled installer size for macOS drops by ≥30 MB; Windows drops by ≥45 MB.
3. A capture taken on Mac populates `ocr_text.text` + `ocr_text.words_json` + `ocr_text.engine = "Vision (...)"` within ~500 ms of `capture:saved`.
4. Same for Windows with `engine = "Windows.Media.Ocr (...)"`.
5. A capture containing each of the six PII categories produces a pending `pii_spans` row per match (with negative-control false positives suppressed).
6. Post-capture toolbar, annotation editor, and library viewer all surface "Copy text" + `<PiiBadge>` + `<TextOverlay>` per §6.
7. Confirming a redaction in the post-capture toolbar or library viewer mutates the underlying image file and records `redacted_at`.
8. Confirming a redaction in the annotation editor adds a non-destructive blur annotation to the edit stack.
9. Issues #31, #41 closed; #40 closed or reassessed with rationale.
