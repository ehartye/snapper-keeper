# Phase 10 — OCR Surfaces (Vision + WinOcr + PII + Text Actions) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Replace the Tesseract sidecar with OS-native OCR on both platforms (Apple Vision on macOS, Windows.Media.Ocr on Windows), persist per-word bounding boxes, add PII auto-detection with explicit per-item user confirmation, and ship a selectable Live-Text-style overlay in the post-capture toolbar, annotation editor, and library viewer. Hard requirement: **100% Tesseract cleanse, no fallback, no tech debt**.

**Architecture:** Four sequenced PRs. PR-1 is two throwaway discovery spikes (no commits to feature branch — findings inform the rest of the plan). PR-2 swaps the OCR backend: new `OcrBackend` trait + `VisionBackend` + `WinOcrBackend` + migration V006 (adds `ocr_text.words_json` + `ocr_text.engine` + `pii_spans` table) + complete Tesseract cleanse. PR-3 adds the new `snk-pii` Tauri plugin: regex-based scanner + worker + commands. PR-4 builds the React surface: shared `<TextOverlay>` / `<PiiBadge>` / `<PiiReviewSheet>` components wired into post-capture toolbar, annotation editor, library viewer, and About panel.

**Tech Stack:** Rust (workspace edition 2021, rustc 1.78+; new deps: `objc2-vision` Mac-only, `windows` crate Win-only with `Media_Ocr` + `Globalization` + `Graphics_Imaging` features; `regex`, `serde_json` workspace), Tauri 2 (plugins + events + commands), React 18 + TypeScript strict + Vitest + RTL, SQLite via `rusqlite` + `rusqlite_migration`.

**Source spec:** [`docs/superpowers/specs/2026-05-25-phase10-ocr-surfaces-design.md`](../specs/2026-05-25-phase10-ocr-surfaces-design.md) (merged via [PR #131](https://github.com/ehartye/snapper-keeper/pull/131)).

**Worktree:** `C:/Users/ehart/repos/snapper-keeper-worktrees/feat-phase10-ocr-surfaces/`
**Branch:** `feat/phase10-ocr-surfaces` (off `origin/main`, which already contains the spec)

---

## PR Overview

| PR | Branch (off `feat/phase10-ocr-surfaces`) | Scope | Approx. tasks |
|---|---|---|---|
| **PR-1 (spikes)** | n/a — throwaway code, not committed | Discovery: WinOcr per-word confidence + NSIS bundle; `objc2-vision` FFI smoke | 2 spikes |
| **PR-2 (foundation)** | `feat/phase10-pr2-foundation` | `OcrBackend` trait + Vision + WinOcr backends; migration V006; `snk-pii` placeholder stub; **Tesseract cleanse** | T1–T19 |
| **PR-3 (PII plugin)** | `feat/phase10-pr3-pii-plugin` | `snk-pii` full implementation: scanner + worker + commands + TS bindings + blur primitive share | T20–T29 |
| **PR-4 (UI surfaces)** | `feat/phase10-pr4-ui-surfaces` | `<TextOverlay>`, `<PiiBadge>`, `<PiiReviewSheet>`; toolbar/editor/library/About wiring; close issues | T30–T39 |

PRs land sequentially on `feat/phase10-ocr-surfaces`. After PR-4 merges into the parent branch, the parent itself opens a final integration PR against `main` for the `v0.2.0` release tag.

---

## Conventions

- **Conventional Commits:** `feat(ocr):`, `feat(pii):`, `feat(ui):`, `chore(deps):`, `chore(cleanse):`, `refactor(library):`, `test(ocr):`, `docs:`, `ci:`.
- **Staging discipline:** `git add <explicit-paths>` only. **Never `git add .` or `-A`.** (Per memory `reference_team_driven_shared_worktree.md`.)
- **`git diff --cached` before every commit** in a shared worktree to catch other-agent staged hunks. (Per memory `feedback_git_diff_cached_before_commit.md`.)
- **One task = one commit** unless explicitly noted as "bundle with previous."
- **Plan-as-source-of-truth:** if you find a bug in this plan, raise it to the team lead BEFORE applying a fix in your commit. Lead edits the plan in place. (Per memory `feedback_plan_as_source_of_truth.md`.)
- **Broad verification scope:** when a task changes a schema or shared type, run the broadest test scope that proves correctness — full crate suites, not module-filtered ones. (Per memory `feedback_broad_verification_scope.md`.)
- **TDD where tests are meaningful.** Schema and pure-logic tasks (regex matchers, Luhn, bbox computation) MUST be test-first. Plumbing tasks (deps add, file deletions, conf edits) verify via "the suite still passes" + targeted smoke.
- **No comments unless WHY is non-obvious.** Defer to user CLAUDE.md.
- **Tesseract cleanse is a verification gate.** PR-2 cannot merge until `git grep -i tesseract -- ':!docs/superpowers/{specs,plans,research,reviews}/**'` returns zero matches.

---

## Pre-flight

You are about to fork a sibling worktree from `main` (which already contains the spec at `docs/superpowers/specs/2026-05-25-phase10-ocr-surfaces-design.md` after PR #131 merged). The execution skill creates the worktree; this section documents what to verify and what's authoritative.

### Required tools

- **Rust:** stable 1.78+ (workspace pins via `rust-toolchain.toml`).
- **pnpm:** 9+.
- **Mac dev:** Xcode 16+ (provides Vision framework SDK).
- **Windows dev:** Windows 10 19041+ / Windows 11. No extra SDK install — `windows` crate vendors what it needs.
- **No Tesseract.** Do not install it during PR-2 execution. The cleanse step verifies it's also not required to build.

### Key files you'll touch

- `Cargo.toml` (workspace root — `snk-pii` added to members)
- `crates/snk-library/migrations/V006__phase10_ocr_bounds_and_pii.sql` (new)
- `crates/snk-library/src/migrate.rs` (wire V006 + tests)
- `crates/snk-library/src/ocr.rs` (extend `OcrText` with `words_json` + `engine`)
- `crates/snk-library/src/pii.rs` (new module)
- `crates/snk-library/src/lib.rs` (`pub mod pii;`)
- `crates/snk-ocr/Cargo.toml` (drop Tesseract-only deps; add `objc2-vision`, `windows`)
- `crates/snk-ocr/src/` (substantially rewritten; `sidecar.rs` deleted)
- `crates/snk-ocr/build.rs` (likely deleted)
- `crates/snk-ocr/tests/integration_test.rs` (replace)
- `crates/snk-pii/` (new crate)
- `packages/snk-ocr/` (TS bindings extended)
- `packages/snk-pii/` (new TS bindings package)
- `app/src-tauri/Cargo.toml` (add `snk-pii` dep)
- `app/src-tauri/src/main.rs` or `lib.rs` (load `snk-pii` plugin)
- `app/src-tauri/capabilities/default.json` (add `snk-pii` permissions)
- `app/src-tauri/tauri.conf.json` (delete `bundle.resources.tesseract/**`)
- `app/src-tauri/resources/tesseract/` (delete directory)
- `app/src/components/TextOverlay.tsx` + `.css` + `.test.tsx` (new)
- `app/src/components/PiiBadge.tsx` + `.test.tsx` (new)
- `app/src/components/PiiReviewSheet.tsx` + `.test.tsx` (new)
- `app/src/windows/capture-toolbar/` (extend)
- `app/src/windows/editor/` (extend)
- `app/src/windows/library/` (extend)
- `app/src/windows/settings/SettingsWindow.tsx` (update OCR description; remove "Tesseract" mention)
- `app/src/windows/settings/AboutSection.tsx` (add OCR engine + PII categories + last error rows)
- `.github/workflows/release.yml` (delete Windows Tesseract bundling step)
- `.gitignore` (delete tesseract paths)
- `README.md` (delete Tesseract install instructions; update plugin description)
- `CLAUDE.md` (delete Tesseract setup pointer; update phase status table)

### objc2 workspace dedupe (Mac-only pre-check)

Per memory `reference_objc2_workspace_dedupe.md`: before pinning `objc2-vision`, run on a Mac (or in Mac CI):

```bash
cargo tree -i objc2 2>&1 | head -50
cargo tree -i objc2-app-kit 2>&1 | head -50
```

Verify the transitive `objc2` version (likely 0.6 via `arboard`, `tao`, `muda`) and the `objc2-app-kit` version (likely 0.3). `objc2-vision` must align with the same major. If `objc2-vision` requires a newer major, **stop and upgrade the whole `objc2` family in one commit before proceeding** — do not ship parallel runtime trains. Document the actual resolved versions in PR-2's PR description.

---

## PR-1: Discovery spikes (no commits to feature branch)

Goal: answer the three watch-items from the spec's research phase BEFORE writing production code. Spike code is throwaway — does not merge to the feature branch.

### Spike A: WinOcr per-word confidence + NSIS bundle

**Why:** spec §6 watch-items #1 + #2. If `OcrWord.Confidence` is line-only, we broadcast line confidence to all words in that line (documented asymmetry). If `OcrEngine::TryCreateFromUserProfileLanguages` fails from a non-MSIX bundle, we replan PR-2.

**Where:** any throwaway directory outside the worktree, e.g. `C:/Users/ehart/scratch/winocr-spike/`.

**Step 1: Create scratch Rust project**

```powershell
mkdir C:\Users\ehart\scratch\winocr-spike
cd C:\Users\ehart\scratch\winocr-spike
cargo init --bin --name winocr_spike
```

**Step 2: Add `windows` dep**

Edit `Cargo.toml`:

```toml
[dependencies]
# Plan amendment (Spike A finding): workspace transitively pulls windows 0.61/0.62
# via tao/wry/tauri. Pin 0.62 to align — see Findings entry "windows crate version".
windows = { version = "0.62", features = [
  "Media_Ocr",
  "Globalization",
  "Graphics_Imaging",
  "Storage",
  "Storage_Streams",
  "Foundation",
] }
```

**Step 3: Write a 60-line probe**

Edit `src/main.rs`:

```rust
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::{FileAccessMode, StorageFile};

fn main() -> anyhow::Result<()> {
    let image_path = std::env::args().nth(1).expect("usage: winocr_spike <png-path>");
    let path = HSTRING::from(std::path::Path::new(&image_path).canonicalize()?.as_os_str());

    // Try user-profile-languages first; fall back to en-US.
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .or_else(|_| OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&HSTRING::from("en-US"))?))?;

    let file = StorageFile::GetFileFromPathAsync(&path)?.get()?;
    let stream = file.OpenAsync(FileAccessMode::Read)?.get()?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
    let bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;

    let result = engine.RecognizeAsync(&bitmap)?.get()?;

    let lines = result.Lines()?;
    let line_count = lines.Size()?;
    println!("=== {} lines ===", line_count);

    for i in 0..line_count {
        let line = lines.GetAt(i)?;
        let text = line.Text()?.to_string_lossy();
        println!("Line {}: '{}'", i, text);

        let words = line.Words()?;
        for j in 0..words.Size()? {
            let w = words.GetAt(j)?;
            let wt = w.Text()?.to_string_lossy();
            let br = w.BoundingRect()?;
            // Try to access .Confidence() — if it doesn't exist, this won't compile,
            // which is itself the answer to the spike question.
            #[allow(unused_variables)]
            let confidence: Option<f64> = None;  // Replace with actual call if API exists
            println!(
                "  word: '{}' bbox=({:.1},{:.1},{:.1},{:.1}) conf=line-only",
                wt, br.X, br.Y, br.Width, br.Height
            );
        }
    }
    Ok(())
}
```

Add `anyhow = "1"` to deps if needed.

**Step 4: Build + run from cmd.exe (NOT MSIX) against a PNG with text**

Use any handy PNG containing visible text — a screenshot of this plan in a browser works.

```powershell
cargo build --release
.\target\release\winocr_spike.exe C:\path\to\some_text_screenshot.png
```

**Expected outcomes:**
- ✅ **Outcome A (best case):** Output prints word-level data successfully. → WinOcr works from NSIS-equivalent (plain `.exe`) bundles. Document the per-word data shape. If `OcrWord.Confidence` accessor exists in the API surface, note it. If not, plan T10 (`WinOcrBackend`) accepts line-broadcast confidence.
- ⚠️ **Outcome B (engine construction fails):** `TryCreateFromUserProfileLanguages` and `TryCreateFromLanguage("en-US")` both error. → MSIX is required. **Stop and replan PR-2** — options are (a) require MSIX packaging change in Tauri bundler config, (b) introduce a tiny WinRT host shim that registers package identity at runtime, or (c) revisit Tesseract-on-Windows decision (would violate hard cleanse requirement; needs user discussion).

**Step 5: Document findings**

Write a 1-paragraph summary in the PR-2 description and in the plan's "Findings" section (added below before T1 lands). Include:
- WinOcr engine construction outcome.
- Whether per-word confidence is exposed.
- Bounding-rect coordinate space (pixels, top-left origin per Microsoft docs — confirm).

Delete the `winocr-spike` directory afterward. Do NOT commit it to the worktree.

---

### Spike B: `objc2-vision` FFI smoke

**Why:** spec §6 watch-item #3. Confirm `objc2-vision` 0.3.x builds clean against current Xcode SDK, returns observations, exposes per-word bounds via `boundingBoxForRange`. If it doesn't, fall back is a small Swift sidecar — would replan T9.

**Where:** any throwaway directory on a Mac, e.g. `~/scratch/vision-spike/`.

**Step 1: Create scratch project**

```bash
mkdir -p ~/scratch/vision-spike && cd ~/scratch/vision-spike
cargo init --bin --name vision_spike
```

**Step 2: Add `objc2-vision` + dependencies**

Edit `Cargo.toml`:

```toml
[dependencies]
objc2 = "0.6"
objc2-foundation = { version = "0.3", features = ["NSString", "NSURL", "NSData"] }
objc2-vision = { version = "0.3", features = [
  # Plan amendment (Spike B finding): "VNImageRequestHandler" and "VNRecognizedTextObservation"
  # are CLASS names, not feature names. Actual features are the parent module names.
  "VNRequest",
  "VNRequestHandler",           # provides VNImageRequestHandler
  "VNRecognizeTextRequest",
  "VNObservation",              # provides VNRecognizedTextObservation
  "VNGeometry",                 # provides VNRectangleObservation (boundingBoxForRange return)
  "objc2-core-foundation",      # gates CGRect on VNDetectedObjectObservation::boundingBox()
] }
objc2-core-image = { version = "0.3", features = ["CIImage"] }
```

(Feature names per crate docs — verify against `cargo doc -p objc2-vision --open` if the feature names have drifted.)

**Step 3: Write a 80-line probe**

Edit `src/main.rs`:

```rust
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSString, NSURL, NSArray};
use objc2_vision::{VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation, VNRequest};

fn main() {
    let path = std::env::args().nth(1).expect("usage: vision_spike <png-path>");
    let abs = std::fs::canonicalize(&path).expect("canonicalize input path");

    unsafe {
        let url_str = NSString::from_str(&abs.to_string_lossy());
        let url = NSURL::fileURLWithPath(&url_str);

        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(objc2_vision::VNRequestTextRecognitionLevel::Accurate);
        request.setAutomaticallyDetectsLanguage(true);

        let handler = VNImageRequestHandler::initWithURL_options(
            VNImageRequestHandler::alloc(),
            &url,
            &objc2_foundation::NSDictionary::new(),
        );

        let requests: Retained<NSArray<VNRequest>> = NSArray::from_slice(&[
            ProtocolObject::from_ref(&*request).cast()
        ]);

        let perform_result = handler.performRequests_error(&requests);
        if let Err(e) = perform_result {
            eprintln!("performRequests failed: {:?}", e);
            std::process::exit(1);
        }

        let observations = request.results()
            .expect("results() returned nil");
        println!("=== {} observations ===", observations.count());

        for i in 0..observations.count() {
            let obs: Retained<VNRecognizedTextObservation> = observations.objectAtIndex(i).cast();
            let candidates = obs.topCandidates(1);
            if candidates.count() == 0 {
                continue;
            }
            let candidate = candidates.objectAtIndex(0);
            let text = candidate.string().to_string();
            let conf = candidate.confidence();
            let bbox = obs.boundingBox();

            println!(
                "[{}] '{}' conf={:.2} bbox=({:.3},{:.3},{:.3},{:.3})",
                i, text, conf, bbox.origin.x, bbox.origin.y, bbox.size.width, bbox.size.height
            );

            // Per-word bounds via boundingBoxForRange.
            let nsstr = candidate.string();
            let total_len = nsstr.len();
            let mut byte_pos = 0;
            for (word_idx, word) in text.split_whitespace().enumerate() {
                let range = objc2_foundation::NSRange { location: byte_pos, length: word.len() };
                byte_pos += word.len() + 1; // crude — fine for spike
                if byte_pos > total_len { break; }
                match candidate.boundingBoxForRange_error(range) {
                    Ok(rect_obs) => {
                        let r = rect_obs.boundingBox();
                        println!(
                            "    word[{}] '{}' bbox=({:.3},{:.3},{:.3},{:.3})",
                            word_idx, word, r.origin.x, r.origin.y, r.size.width, r.size.height
                        );
                    }
                    Err(e) => eprintln!("    word[{}] '{}' boundingBoxForRange err: {:?}", word_idx, word, e),
                }
            }
        }
    }
}
```

API names follow `objc2` 0.6 generated bindings. If they've drifted (selector method names change between objc2 minor versions), use `cargo doc -p objc2-vision --open` and adjust selectors — that's what the spike is for.

**Step 4: Build and run against a Mac-captured PNG with text**

```bash
cargo build --release
./target/release/vision_spike /path/to/some_screenshot.png
```

**Expected outcomes:**
- ✅ **Outcome A:** Builds clean, prints observations with per-word bboxes. → T9 proceeds with FFI implementation. Note any selector-name divergences from this spike's code.
- ⚠️ **Outcome B (build fails):** `objc2-vision` 0.3 feature names don't match, or selectors absent. → Try newer `objc2-vision` version. If newer requires `objc2` major bump, run the workspace dedupe check (pre-flight section) — may require updating the whole `objc2` family. Document the actual versions that work.
- ⚠️ **Outcome C (runtime crash or empty observations):** Vision call works but returns nothing. → Could be a sandbox/permissions issue with the throwaway binary (Vision needs no entitlement, but read-access to the image file matters). Try moving the image to `~/Desktop` and re-running.
- ⛔ **Outcome D (fundamentally broken):** No path forward via objc2. Fall back is a Swift sidecar (~50 lines). This adds a bundled binary, violates the spec's "no new bundled binaries" non-goal. **Stop and discuss with the user before continuing.**

**Step 5: Document findings**

Same as Spike A — write a 1-paragraph summary into the PR-2 description and the plan's "Findings" section.

Delete `vision-spike` afterward. Do NOT commit it.

---

### Findings — fill in after spikes complete

> **For the implementer:** edit this section in place with the spike results before starting T1. This becomes the authoritative record of what's known about the underlying APIs.

- **WinOcr engine construction from plain `.exe` bundle:** ✅ **Outcome A.** `OcrEngine::TryCreateFromUserProfileLanguages()` succeeds from a plain `.exe` (no MSIX package identity, no manifest). Verified on Windows 11 with only `en-US` language pack installed; returned recognizer language tag `en-US`. `AvailableRecognizerLanguages()` enumerates installed packs; `MaxImageDimension()` returns 10000 px. No MSIX work required — NSIS bundle is fine.
- **WinOcr per-word `Confidence` API:** ❌ **NOT exposed at any level.** The `OcrWord` class surface in `windows = "0.62"` exposes ONLY `Text()` and `BoundingRect()`. The `OcrLine` class surface exposes ONLY `Text()` and `Words()`. There is no `Confidence()` accessor anywhere in `Windows.Media.Ocr` (confirmed by reading `windows-0.62.2/src/Windows/Media/Ocr/mod.rs:230-244` and lines 159-174). T8 must use a heuristic line confidence (per-plan: 0.85) broadcast to every word — this is the documented asymmetry vs Vision.
- **WinOcr bounding-rect coordinate space:** ✅ **Confirmed pixels, top-left origin.** Probe on 600×200 PNG with text drawn at Y=30 and Y=90 returned word bboxes at Y≈42 and Y≈102 in raw pixel units; bottom-right of words remained within the image bounds. T8 will divide by `decoder.PixelWidth()/PixelHeight()` to produce the normalized `BBox` the schema expects.
- **`windows` crate version:** The workspace already pulls `windows = "0.61.3"` and `windows = "0.62.2"` transitively (via `tao`/`wry`/`tauri`-stack). The plan's suggested `0.58` is stale. Spike used `0.62` and built clean. T8 should pin `0.62` (not `0.58`) to align with the workspace and minimize duplicate-major bloat.
- **windows-rs 0.62 async API change:** `IAsyncOperation::get()` was REMOVED in this version. Use the inherent method `.join()` instead (e.g. `StorageFile::GetFileFromPathAsync(&path)?.join()?`). It is a public method on each of the four async types (`IAsyncAction`, `IAsyncOperation<T>`, `IAsyncActionWithProgress<P>`, `IAsyncOperationWithProgress<T,P>`); no `windows-future` dep or trait import needed. The plan T8 code block uses `.and_then(|op| op.get())` — must be rewritten to `.and_then(|op| op.join())`.
- **Windows UNC path gotcha for StorageFile:** `Path::canonicalize()` on Windows returns paths prefixed with `\\?\` (extended-length namespace). `StorageFile::GetFileFromPathAsync` REJECTS that prefix with HRESULT 0x800700A1 ("path is too long"). The implementation must strip the `\\?\` prefix before constructing the `HSTRING`. Trivial 5-line fix; required.
- **`objc2-vision` resolved version:** ✅ `0.3.2`. `cargo check --target aarch64-apple-darwin` builds clean against a throwaway probe with the corrected feature set. Runtime FFI smoke (`performRequests` against a real Vision framework + verifying observations + per-word bbox extraction) is **DEFERRED TO MAC CI** — Spike B was executed from a Windows host where the framework is unreachable. PR-2's Mac CI build job validates runtime on PR-2 open.
- **`objc2` family versions in workspace:** ✅ All aligned at the same major. `objc2 0.6.4`, `objc2-foundation 0.3.2`, `objc2-app-kit 0.3.2` (all transitive via arboard/global-hotkey/muda); `objc2-vision 0.3.2` (new). No parallel runtime trains; objc2 workspace-dedupe check passes (per memory `reference_objc2_workspace_dedupe.md`).
- **Vision per-word bounds API verified:** ✅ Source-confirmed (runtime deferred). `VNRecognizedText::boundingBoxForRange_error(NSRange) -> Result<Retained<VNRectangleObservation>, Retained<NSError>>` exists in `objc2-vision 0.3.2`'s hand-written `src/observation.rs` (gated on `VNObservation` feature). `VNRectangleObservation::boundingBox() -> CGRect` exists on superclass `VNDetectedObjectObservation` (gated on `objc2-core-foundation` feature). T7's `recognize()` implementation matches the API surface; selectors verified by reading the crate source at `~/.cargo/registry/src/.../objc2-vision-0.3.2/`.
- **Plan amendments triggered by Spike B findings (approved 2026-05-26 — already applied in place):**
    1. **T6 Mac dep block** — replaced non-existent feature names `VNImageRequestHandler` / `VNRecognizedTextObservation` (class names, not features) with `VNRequestHandler` / `VNObservation`; added `VNGeometry`, `objc2-core-foundation`, and `NSProcessInfo` (on `objc2-foundation`). See inline plan amendment comment in §T6 Step 1.
    2. **§PR-1 Spike B Step 2** — same dep-block correction so the spike code in the plan compiles if re-run on a Mac.
- **⚠️ Spike B caveat — Windows-host verification does NOT compile Mac code** (added 2026-05-26 post-Mac-CI catch):
    Spike B's "resolution check on Windows host" is fundamentally insufficient for verifying Mac-only objc2 code compiles. The build.rs chain (`objc2-exception-helper` → `try_catch.m`) requires a `cc` toolchain for `aarch64-apple-darwin` that doesn't exist on Windows. Cargo silently fails at the build-script step before rustc proper runs.

    **What Windows-host verification DOES prove:** Cargo.toml feature names resolve, dep versions are compatible, crate metadata is valid.

    **What it does NOT prove:** any line of `vision.rs` actually compiles.

    Implementers writing Mac-only objc2/Vision code from a Windows host must treat Mac CI on PR open as the FIRST real compile. Expect at least one iteration of "push, watch Mac CI fail, fix trait imports, push again." Budget for this in time estimates.

    Mitigation in this codebase: spec §6's "Vision per-word bounds API verified via `boundingBoxForRange_error`" remains valid (verified by reading crate source), but the broader "objc2-vision FFI smoke" claim in Spike B must include "compile-verified on Mac" before it can be called done.

    Historical record — PR-2's first Mac CI run (PR #135) caught 3 trait-import errors not surfaced by any Windows-host check: `VNImageRequestHandler::alloc()` needed `use objc2::AnyThread;`; `ProtocolObject::from_ref(...).cast()` was the wrong abstraction (VNRequest is a class, not a protocol) — replaced with `request.as_super().as_super()` via `use objc2::ClassType;`; `Retained::cast()` is deprecated in objc2 0.6 — replaced with `Retained::cast_unchecked`. Fixed in commit `85f034e`.
- **Plan amendments triggered by Spike A findings (proposed; awaiting team-lead approval per plan-as-source-of-truth protocol):**
    1. **T6 + T8** — pin `windows = "0.62"` (not `0.58`). Plus drop `Foundation_Collections` feature only if unused; spike confirms `Foundation_Collections` is NOT required for the core OCR call path (we never instantiate a generic vector; the API returns `IVectorView<OcrLine>` whose interface is exported by `Media_Ocr`).
    2. **T8 `WinOcrBackend::recognize`** — three diffs vs the plan's pasted code block:
        - Replace every `.and_then(|op| op.get())` with `.and_then(|op| op.join())` (4 call sites).
        - After `image_path.canonicalize()`, strip a leading `\\?\` prefix before building the `HSTRING`.
        - Delete the `line_confidence` helper and the `total_conf`/`avg` computation; hard-code line-broadcast confidence at `0.85` and store `confidence = 0.85` on the `OcrResult` too. Document in a single comment why (no API).
    3. **T6 `Cargo.toml`** — leave `windows` features as planned (`Media_Ocr, Globalization, Graphics_Imaging, Storage, Storage_Streams, Foundation`). No need to add `windows-future` because `.join()` is an inherent method on the async types.

---

## PR-2: Foundation (OCR backend swap + migration + Tesseract cleanse)

**Branch:** `feat/phase10-pr2-foundation` off `feat/phase10-ocr-surfaces`.

Goal: replace Tesseract with native OCR backends, persist per-word bounds + engine on `ocr_text`, create the `pii_spans` table for PR-3 to populate, and eliminate every Tesseract artifact from the repo. Ends with a working app on both platforms where capture → OCR pipeline writes `text` + `words_json` + `engine` and emits the new `ocr:ready` event.

### Task 1: Workspace stub for `snk-pii` crate

**Why first:** the spec adds `snk-pii` to workspace members. Per CLAUDE.md: "Cargo workspace requires every declared member to exist on disk. If you add members to the root `Cargo.toml` for tasks that haven't shipped yet, also commit a placeholder manifest." Stub now; flesh out in PR-3.

**Files:**
- Create: `crates/snk-pii/Cargo.toml`
- Create: `crates/snk-pii/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Create stub `Cargo.toml`**

```toml
[package]
name = "snk-pii"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
```

**Step 2: Create empty `src/lib.rs`**

```rust
//! snk-pii — PII detection plugin. Stub; implementation lands in PR-3.
```

**Step 3: Add to workspace members**

Read the workspace `[workspace]` table in `Cargo.toml` (root). Add `"crates/snk-pii"` to the `members` array, alphabetically next to `crates/snk-ocr`.

**Step 4: Verify workspace builds**

```bash
cargo build -p snk-pii
```

Expected: builds clean (no source, no deps).

**Step 5: Commit**

```bash
git add Cargo.toml crates/snk-pii/Cargo.toml crates/snk-pii/src/lib.rs
git diff --cached
git commit -m "chore(workspace): add snk-pii crate stub for Phase 10"
```

---

### Task 2: Migration V006 — `words_json`, `engine`, `pii_spans`

**Files:**
- Create: `crates/snk-library/migrations/V006__phase10_ocr_bounds_and_pii.sql`
- Modify: `crates/snk-library/src/migrate.rs`

**Step 1: Write the failing migration test first**

In `crates/snk-library/src/migrate.rs`, **append to `mod tests`** (after `v005_drops_sensitive_column_from_clipboard_items`):

```rust
#[test]
fn v006_adds_words_json_and_engine_to_ocr_text() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).expect("apply migrations");

    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(ocr_text)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert!(cols.contains(&"words_json".into()), "words_json column missing; got {cols:?}");
    assert!(cols.contains(&"engine".into()), "engine column missing; got {cols:?}");
}

#[test]
fn v006_creates_pii_spans_table_with_indexes() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).expect("apply migrations");

    let cnt: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pii_spans'",
        [], |row| row.get(0)
    ).unwrap();
    assert_eq!(cnt, 1);

    let idx_full: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_pii_spans_capture'",
        [], |row| row.get(0)
    ).unwrap();
    assert_eq!(idx_full, 1);

    let idx_pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_pii_spans_pending'",
        [], |row| row.get(0)
    ).unwrap();
    assert_eq!(idx_pending, 1);
}
```

**Step 2: Run tests — confirm they fail**

```bash
cargo test -p snk-library migrate::tests::v006
```

Expected: both tests FAIL — `words_json column missing` and `pii_spans count != 1`.

**Step 3: Create the migration SQL**

Create `crates/snk-library/migrations/V006__phase10_ocr_bounds_and_pii.sql`:

```sql
-- Phase 10 — Per-word bounds + engine version on ocr_text; PII spans table.

ALTER TABLE ocr_text ADD COLUMN words_json TEXT;
ALTER TABLE ocr_text ADD COLUMN engine     TEXT NOT NULL DEFAULT '';

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

**Step 4: Wire V006 into `migrate.rs`**

In `crates/snk-library/src/migrate.rs`:

```rust
const V006: &str = include_str!("../migrations/V006__phase10_ocr_bounds_and_pii.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(V001),
        M::up(V002),
        M::up(V003),
        M::up(V004),
        M::up(V005),
        M::up(V006),
    ])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 6,   // bumped from 5
            recoverable: e.to_string().contains("Backup"),
        })?;
    Ok(())
}
```

**Step 5: Run tests — confirm they pass**

```bash
cargo test -p snk-library migrate
```

Expected: ALL migrate tests PASS, including `migration_count_matches_latest_applied_version` (which checks file count vs applied version — proves V006 is wired correctly).

**Step 6: Commit**

```bash
git add crates/snk-library/migrations/V006__phase10_ocr_bounds_and_pii.sql crates/snk-library/src/migrate.rs
git diff --cached
git commit -m "feat(library): V006 migration — words_json, engine, pii_spans"
```

---

### Task 3: Extend `OcrText` + add `OcrWord` / `BBox` types in `snk-library`

**Files:**
- Modify: `crates/snk-library/src/ocr.rs`

**Step 1: Add failing tests**

Append to the existing `mod tests` in `crates/snk-library/src/ocr.rs`:

```rust
#[test]
fn upsert_persists_words_json_and_engine() {
    let (_tmp, db) = fresh_db();
    let cap_id = insert_capture(&db);
    let words = vec![
        OcrWord { text: "hello".into(), bbox: BBox { x: 0.1, y: 0.05, w: 0.08, h: 0.04 }, confidence: 0.97, line: 0 },
        OcrWord { text: "world".into(), bbox: BBox { x: 0.19, y: 0.05, w: 0.08, h: 0.04 }, confidence: 0.95, line: 0 },
    ];
    upsert_full(&db, &cap_id, "hello world", "eng", 0.95, &words, "Vision (test)").unwrap();

    let row = get(&db, &cap_id).unwrap().unwrap();
    assert_eq!(row.text, "hello world");
    assert_eq!(row.engine, "Vision (test)");
    let parsed = row.words.expect("words populated");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].text, "hello");
    assert!((parsed[0].confidence - 0.97).abs() < 1e-6);
}

#[test]
fn legacy_upsert_leaves_words_null() {
    let (_tmp, db) = fresh_db();
    let cap_id = insert_capture(&db);
    upsert(&db, &cap_id, "legacy text", "eng", 0.8).unwrap();
    let row = get(&db, &cap_id).unwrap().unwrap();
    assert!(row.words.is_none(), "legacy upsert must leave words_json NULL");
    assert_eq!(row.engine, "");
}
```

**Step 2: Run tests — confirm they fail**

```bash
cargo test -p snk-library ocr::tests
```

Expected: FAIL — `OcrWord`/`BBox`/`upsert_full` not defined.

**Step 3: Add types + update `OcrText` + add `upsert_full`**

Rewrite the top of `crates/snk-library/src/ocr.rs` to:

```rust
use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub bbox: BBox,
    pub confidence: f64,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrText {
    pub capture_id: String,
    pub text: String,
    pub language: String,
    pub confidence: f64,
    pub created_at: i64,
    pub engine: String,
    pub words: Option<Vec<OcrWord>>,
}

/// Legacy upsert — does not populate words_json or engine. Kept so existing
/// call-sites (and tests) don't break during the transition. New OCR pipeline
/// code MUST call `upsert_full` instead.
pub fn upsert(
    db: &Db,
    capture_id: &str,
    text: &str,
    language: &str,
    confidence: f64,
) -> Result<()> {
    upsert_full(db, capture_id, text, language, confidence, &[], "")
}

pub fn upsert_full(
    db: &Db,
    capture_id: &str,
    text: &str,
    language: &str,
    confidence: f64,
    words: &[OcrWord],
    engine: &str,
) -> Result<()> {
    let created_at = chrono::Utc::now().timestamp_millis();
    let words_json = if words.is_empty() {
        None
    } else {
        Some(serde_json::to_string(words).map_err(|e| crate::LibraryError::Persist {
            detail: format!("serialize words_json: {e}"),
        })?)
    };
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO ocr_text (capture_id, text, language, confidence, created_at, words_json, engine)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(capture_id) DO UPDATE SET
                text = excluded.text,
                language = excluded.language,
                confidence = excluded.confidence,
                created_at = excluded.created_at,
                words_json = excluded.words_json,
                engine = excluded.engine",
            rusqlite::params![capture_id, text, language, confidence, created_at, words_json, engine],
        )?;
        Ok(())
    })
}

pub fn get(db: &Db, capture_id: &str) -> Result<Option<OcrText>> {
    db.with_conn(|conn| {
        let result = conn.query_row(
            "SELECT capture_id, text, language, confidence, created_at, words_json, engine
             FROM ocr_text WHERE capture_id = ?1",
            [capture_id],
            |row| {
                let words_json: Option<String> = row.get(5)?;
                let words = words_json
                    .map(|s| serde_json::from_str::<Vec<OcrWord>>(&s))
                    .transpose()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        5, rusqlite::types::Type::Text, Box::new(e),
                    ))?;
                Ok(OcrText {
                    capture_id: row.get(0)?,
                    text: row.get(1)?,
                    language: row.get(2)?,
                    confidence: row.get(3)?,
                    created_at: row.get(4)?,
                    engine: row.get(6)?,
                    words,
                })
            },
        );
        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}
```

If `LibraryError::Persist` doesn't exist yet, add `Persist { detail: String }` to the enum in `crates/snk-library/src/error.rs` (the file where `LibraryError` actually lives — `lib.rs` only re-exports it), marked with the same serde tags as the others.

Plan amendment 2026-05-26: also append a snapshot entry for the new `Persist` variant to `crates/snk-library/tests/library_error_wire_shape.rs`, matching the existing per-variant pattern. The test file's header explicitly states "failing here is intentional; update snapshot when contract changes" — adding a new variant IS a contract change.

**Step 4: Run tests — confirm they pass**

```bash
cargo test -p snk-library ocr
```

Expected: ALL `ocr::tests` pass (new tests + the existing `upsert_inserts_new_ocr_text`, `upsert_replaces_existing_ocr_text`, `get_returns_none_for_missing`).

**Step 5: Run full crate suite (broad verification per memory)**

```bash
cargo test -p snk-library
```

Expected: full suite passes. If anything else (search, fts) breaks, it's likely a downstream consumer of `OcrText` that didn't account for the two new fields. Fix before commit.

**Step 6: Commit**

```bash
git add crates/snk-library/src/ocr.rs crates/snk-library/src/error.rs crates/snk-library/tests/library_error_wire_shape.rs
git diff --cached
git commit -m "feat(library): OcrText words_json + engine; OcrWord/BBox types"
```

---

### Task 4: New `pii` module in `snk-library`

**Files:**
- Create: `crates/snk-library/src/pii.rs`
- Modify: `crates/snk-library/src/lib.rs` (`pub mod pii;`)

**Step 1: Add failing test scaffold**

Create `crates/snk-library/src/pii.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PiiCategory {
    Email,
    Phone,
    CreditCard,
    Ssn,
    Ip,
    ApiKey,
}

impl PiiCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            PiiCategory::Email => "email",
            PiiCategory::Phone => "phone",
            PiiCategory::CreditCard => "credit_card",
            PiiCategory::Ssn => "ssn",
            PiiCategory::Ip => "ip",
            PiiCategory::ApiKey => "api_key",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PiiSpan {
    pub id: i64,
    pub capture_id: String,
    pub category: PiiCategory,
    pub matched_text: String,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_w: f32,
    pub bbox_h: f32,
    pub confidence: f64,
    pub redacted_at: Option<i64>,
    pub dismissed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewPiiSpan<'a> {
    pub capture_id: &'a str,
    pub category: PiiCategory,
    pub matched_text: &'a str,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_w: f32,
    pub bbox_h: f32,
    pub confidence: f64,
}

pub fn insert(db: &Db, span: NewPiiSpan<'_>) -> Result<i64> {
    let created_at = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO pii_spans
                (capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h, confidence, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                span.capture_id, span.category.as_str(), span.matched_text,
                span.bbox_x, span.bbox_y, span.bbox_w, span.bbox_h,
                span.confidence, created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    })
}

pub fn list_for_capture(db: &Db, capture_id: &str) -> Result<Vec<PiiSpan>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h,
                    confidence, redacted_at, dismissed_at, created_at
             FROM pii_spans WHERE capture_id = ?1 ORDER BY id"
        )?;
        let rows = stmt.query_map([capture_id], row_to_span)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    })
}

pub fn list_pending_for_capture(db: &Db, capture_id: &str) -> Result<Vec<PiiSpan>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h,
                    confidence, redacted_at, dismissed_at, created_at
             FROM pii_spans WHERE capture_id = ?1
                AND redacted_at IS NULL AND dismissed_at IS NULL ORDER BY id"
        )?;
        let rows = stmt.query_map([capture_id], row_to_span)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    })
}

pub fn mark_redacted(db: &Db, span_id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE pii_spans SET redacted_at = ?1 WHERE id = ?2",
            rusqlite::params![now, span_id],
        )?;
        Ok(())
    })
}

pub fn mark_dismissed(db: &Db, span_id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE pii_spans SET dismissed_at = ?1 WHERE id = ?2",
            rusqlite::params![now, span_id],
        )?;
        Ok(())
    })
}

pub fn get(db: &Db, span_id: i64) -> Result<Option<PiiSpan>> {
    db.with_conn(|conn| {
        let res = conn.query_row(
            "SELECT id, capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h,
                    confidence, redacted_at, dismissed_at, created_at
             FROM pii_spans WHERE id = ?1",
            [span_id], row_to_span);
        match res {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}

fn row_to_span(row: &rusqlite::Row<'_>) -> rusqlite::Result<PiiSpan> {
    let category_str: String = row.get(2)?;
    let category = match category_str.as_str() {
        "email" => PiiCategory::Email,
        "phone" => PiiCategory::Phone,
        "credit_card" => PiiCategory::CreditCard,
        "ssn" => PiiCategory::Ssn,
        "ip" => PiiCategory::Ip,
        "api_key" => PiiCategory::ApiKey,
        other => return Err(rusqlite::Error::FromSqlConversionFailure(
            2, rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("unknown PII category {other}"))),
        )),
    };
    Ok(PiiSpan {
        id: row.get(0)?,
        capture_id: row.get(1)?,
        category,
        matched_text: row.get(3)?,
        bbox_x: row.get(4)?,
        bbox_y: row.get(5)?,
        bbox_w: row.get(6)?,
        bbox_h: row.get(7)?,
        confidence: row.get(8)?,
        redacted_at: row.get(9)?,
        dismissed_at: row.get(10)?,
        created_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::test_support::fresh_db;

    fn cap(db: &Db) -> String {
        crate::captures::insert(db, crate::NewCapture {
            file_path: PathBuf::from("test.png"),
            width: 100, height: 100,
            source_app: None, source_window_title: None, monitor: None,
        }).unwrap().id
    }

    fn span_for<'a>(c: &'a str, t: &'a str) -> NewPiiSpan<'a> {
        NewPiiSpan {
            capture_id: c, category: PiiCategory::Email, matched_text: t,
            bbox_x: 0.1, bbox_y: 0.1, bbox_w: 0.1, bbox_h: 0.05, confidence: 0.9,
        }
    }

    #[test]
    fn insert_then_get_round_trips() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let id = insert(&db, span_for(&c, "alice@example.com")).unwrap();
        let row = get(&db, id).unwrap().unwrap();
        assert_eq!(row.category, PiiCategory::Email);
        assert_eq!(row.matched_text, "alice@example.com");
        assert!(row.redacted_at.is_none());
    }

    #[test]
    fn list_pending_excludes_resolved() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let id1 = insert(&db, span_for(&c, "a@x.com")).unwrap();
        let id2 = insert(&db, span_for(&c, "b@x.com")).unwrap();
        let id3 = insert(&db, span_for(&c, "c@x.com")).unwrap();
        mark_redacted(&db, id1).unwrap();
        mark_dismissed(&db, id3).unwrap();
        let pending = list_pending_for_capture(&db, &c).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id2);
    }

    #[test]
    fn list_for_capture_returns_all_states() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let id1 = insert(&db, span_for(&c, "a@x.com")).unwrap();
        mark_redacted(&db, id1).unwrap();
        let all = list_for_capture(&db, &c).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].redacted_at.is_some());
    }

    #[test]
    fn category_round_trip_through_db() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        for cat in [PiiCategory::Email, PiiCategory::Phone, PiiCategory::CreditCard,
                    PiiCategory::Ssn, PiiCategory::Ip, PiiCategory::ApiKey] {
            let mut s = span_for(&c, "x");
            s.category = cat;
            let id = insert(&db, s).unwrap();
            let r = get(&db, id).unwrap().unwrap();
            assert_eq!(r.category, cat);
        }
    }

    #[test]
    fn cascading_delete_via_captures_fk() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let _ = insert(&db, span_for(&c, "a@x.com")).unwrap();
        let _ = insert(&db, span_for(&c, "b@x.com")).unwrap();
        crate::captures::delete(&db, &c).unwrap();
        let remaining = list_for_capture(&db, &c).unwrap();
        assert!(remaining.is_empty());
    }
}
```

**Step 2: Wire module in `lib.rs`**

In `crates/snk-library/src/lib.rs`, add `pub mod pii;` next to `pub mod ocr;`.

**Step 3: Run tests**

```bash
cargo test -p snk-library pii
```

Expected: all 5 `pii::tests` pass. If `captures::delete` doesn't exist, swap the cascading-delete test to delete via raw SQL (`conn.execute("DELETE FROM captures WHERE id = ?1", [c])`).

**Step 4: Run full crate suite**

```bash
cargo test -p snk-library
```

Expected: full pass.

**Step 5: Commit**

```bash
git add crates/snk-library/src/pii.rs crates/snk-library/src/lib.rs
git diff --cached
git commit -m "feat(library): pii module — PiiSpan, PiiCategory, CRUD"
```

---

### Task 5: `snk-ocr` types module + `OcrBackend` trait + `OcrError`

**Files:**
- Create: `crates/snk-ocr/src/backend.rs`
- Create: `crates/snk-ocr/src/error.rs`
- Modify: `crates/snk-ocr/src/lib.rs`
- Modify: `crates/snk-ocr/tests/integration_test.rs` — gate the body behind `#![cfg(any())]` until T12 rewrites (plan amendment 2026-05-26 — downgrading sidecar to private breaks the existing test's `snk_ocr::sidecar::run_tesseract` calls from outside the crate; gating keeps `cargo test --workspace` green through the T5→T12 window)

**Step 1: Write trait + types + error**

Create `crates/snk-ocr/src/backend.rs`:

```rust
use std::path::Path;

pub use snk_library::ocr::{BBox, OcrWord};

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<OcrWord>,
    pub language: String,
    pub confidence: f64,
}

pub trait OcrBackend: Send + Sync {
    fn recognize(&self, image_path: &Path) -> Result<OcrResult, crate::OcrError>;
    fn name(&self) -> &'static str;
    fn engine_version(&self) -> String;
}
```

Create `crates/snk-ocr/src/error.rs`:

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
#[serde(tag = "kind")]
pub enum OcrError {
    #[error("OCR backend unavailable: {reason}")]
    BackendUnavailable { reason: String },

    #[error("no recognizer language available: {detail}")]
    NoRecognizerLanguage { detail: String },

    #[error("recognize failed: {detail}")]
    Recognize { detail: String },

    #[error("image load failed for {path}: {detail}")]
    ImageLoad { path: String, detail: String },
}
```

**Step 2: Rewrite `lib.rs`**

`crates/snk-ocr/src/lib.rs`:

```rust
//! snk-ocr — native OCR backend selection (Vision / Windows.Media.Ocr) + async queue.

pub mod backend;
pub mod error;
pub mod plugin;
pub mod queue;

// Sidecar module kept (private) until T9 rewrites queue.rs and T10 rewrites plugin.rs.
// Both still reference crate::sidecar::* internally. Full deletion happens in T13.
mod sidecar;

// `vision` and `winocr` modules are declared by T7 and T8 respectively
// (each adds its own `#[cfg(target_os = "...")] pub mod ...;` line here
// alongside creating the source file). T5 does NOT declare them — would
// fail `cargo build` because the source files don't exist yet.

pub use backend::{OcrBackend, OcrResult};
pub use error::OcrError;
pub use plugin::init;
```

Plan amendment 2026-05-26: `sidecar.rs` is downgraded from `pub mod sidecar;` to a private `mod sidecar;` here (rather than removed entirely) because `queue.rs` and `plugin.rs` still reference `crate::sidecar::*` until T9 rewrites the queue and T10 rewrites the plugin. T13 deletes both the file and this `mod` declaration together. Without this private `mod` line, `cargo build -p snk-ocr` fails on Step 3. The two-line comment is a worthwhile exception to the "no comments" rule because the dead-walking transition is non-obvious.

**Step 3: Gate the old integration test**

Replace the body of `crates/snk-ocr/tests/integration_test.rs` with:

```rust
// Integration tests gated until T12 rewrites them against the native backend.
// Sidecar is downgraded to a private module in T5 (lib.rs), so the previous
// `snk_ocr::sidecar::run_tesseract` test calls would no longer compile from
// outside the crate. T12 replaces this file entirely with backend-trait tests.
#![cfg(any())]
```

The `#![cfg(any())]` attribute (always-false predicate) excludes the entire test file from compilation. Single line; zero behavioral risk; preserves bisect-cleanliness through the T5→T12 window.

**Step 4: Run build + tests**

```bash
cargo build -p snk-ocr
cargo test -p snk-ocr
```

Expected: both pass clean. Plan amendment 2026-05-26: the previous "tests will fail" caveat was wrong — see Step 3.

**Step 5: Commit**

```bash
git add crates/snk-ocr/src/backend.rs crates/snk-ocr/src/error.rs crates/snk-ocr/src/lib.rs crates/snk-ocr/tests/integration_test.rs
git diff --cached
git commit -m "feat(ocr): OcrBackend trait, OcrResult, OcrError types"
```

---

### Task 6: `snk-ocr/Cargo.toml` — add platform deps, prepare for cleanse

**Files:**
- Modify: `crates/snk-ocr/Cargo.toml`

**Step 1: Rewrite `[dependencies]` + add platform targets**

```toml
[package]
name = "snk-ocr"
version = "0.0.1"
links = "snk-ocr"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[build-dependencies]
tauri-plugin = { workspace = true }

[dependencies]
snk-library = { path = "../snk-library" }
tauri.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true

[target.'cfg(target_os = "macos")'.dependencies]
# Plan amendments (Spike B findings approved 2026-05-26):
#   - "VNImageRequestHandler" / "VNRecognizedTextObservation" are CLASS names, not features.
#     The actual features are "VNRequestHandler" and "VNObservation".
#   - "VNGeometry" needed for VNRectangleObservation (returned by boundingBoxForRange_error).
#   - "objc2-core-foundation" gates CGRect on VNDetectedObjectObservation::boundingBox().
#   - "NSProcessInfo" needed by T7's engine_version() OS version read.
#   - "NSRange" pulled in transitively by "VNObservation" — kept explicit for clarity.
objc2 = "0.6"
objc2-foundation = { version = "0.3", features = [
    "NSString", "NSURL", "NSArray", "NSDictionary", "NSError", "NSRange",
    "NSProcessInfo",
] }
objc2-vision = { version = "0.3", features = [
    "VNRequest",
    "VNRequestHandler",
    "VNRecognizeTextRequest",
    "VNObservation",
    "VNGeometry",
    "objc2-core-foundation",
] }

[target.'cfg(target_os = "windows")'.dependencies]
# Plan amendments (Spike A + T8 follow-up, approved 2026-05-26):
#   - Pin to 0.62 to align with workspace transitive (via tao/wry/tauri).
#     Avoids duplicate-major bloat.
#   - "Wdk_System_SystemServices" + "Win32_System_SystemInformation" + "Win32_Foundation"
#     added for RtlGetVersion call in win_build_number() (T8 step 1 helper).
windows = { version = "0.62", features = [
    "Media_Ocr",
    "Globalization",
    "Graphics_Imaging",
    "Storage",
    "Storage_Streams",
    "Foundation",
    "Wdk_System_SystemServices",
    "Win32_System_SystemInformation",
    "Win32_Foundation",
] }

[dev-dependencies]
image = { workspace = true }
serial_test = "3"
tempfile = "3"
```

(Adjust `objc2-vision` / `objc2-foundation` feature names to whatever the Spike B findings recorded as actually working. Adjust `windows` version to the latest stable major if the workspace pins a different version — check `cargo tree | grep windows-rs` first.)

Per memory `reference_objc2_workspace_dedupe.md`: after this edit, run `cargo tree -i objc2` to confirm only one major version. If two majors, escalate to user before continuing — workspace-wide objc2 upgrade is its own task.

**Step 2: Verify build**

```bash
# On Mac:
cargo build -p snk-ocr --target aarch64-apple-darwin

# On Windows:
cargo build -p snk-ocr --target x86_64-pc-windows-msvc
```

Expected: builds clean on the host platform. Cross-platform build is not required at this step — just confirm the host platform compiles with the new deps.

If the build fails because `which` or `serial_test` is removed from `[dependencies]` and still referenced by `sidecar.rs`, that's expected and fixed in T13. You may need to temporarily comment out `pub mod sidecar;` in `lib.rs` to unblock the build — already done in T5.

**Step 3: Commit**

```bash
git add crates/snk-ocr/Cargo.toml Cargo.lock
git diff --cached
git commit -m "chore(ocr): swap deps — drop Tesseract sidecar crates, add objc2-vision + windows"
```

Plan amendment 2026-05-26 (vision-mac during T6): `Cargo.lock` MUST be staged alongside dep changes in this repo. Repo precedent (`git log --grep='^chore(deps)'`) shows every prior dep-touching commit bundles the lockfile. Omitting it desyncs every other agent's checkout. The plan's original single-file `git add` was local-only thinking missing multi-agent impact.

---

### Task 7: `VisionBackend` (macOS)

**Files:**
- Create: `crates/snk-ocr/src/vision.rs`
- Modify: `crates/snk-ocr/src/lib.rs` — add `#[cfg(target_os = "macos")] pub mod vision;` (plan amendment 2026-05-26: T5 left this declaration to T7 so the file-vs-declaration order is consistent)

**Step 1: Implement the backend**

Use the API surface confirmed by Spike B. Below is the conservative shape — selectors and constants may need to match what the spike found:

```rust
#![cfg(target_os = "macos")]

use std::path::Path;

use objc2::rc::Retained;
use objc2_foundation::{NSDictionary, NSRange, NSString, NSURL};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation,
    VNRequestTextRecognitionLevel,
};
use tracing::{debug, warn};

use crate::backend::{BBox, OcrBackend, OcrResult, OcrWord};
use crate::OcrError;

pub struct VisionBackend;

impl VisionBackend {
    pub fn new() -> Result<Self, OcrError> {
        // No construction-time failure modes — Vision is part of the OS.
        Ok(Self)
    }
}

impl OcrBackend for VisionBackend {
    fn name(&self) -> &'static str { "Vision" }

    fn engine_version(&self) -> String {
        // Read macOS version via NSProcessInfo for the audit field.
        // Fallback to "unknown" rather than failing — engine_version is informational.
        let v = unsafe {
            let pi = objc2_foundation::NSProcessInfo::processInfo();
            let os = pi.operatingSystemVersion();
            format!("{}.{}.{}", os.majorVersion, os.minorVersion, os.patchVersion)
        };
        format!("Vision (macOS {v})")
    }

    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let abs = image_path.canonicalize().map_err(|e| OcrError::ImageLoad {
            path: image_path.display().to_string(),
            detail: e.to_string(),
        })?;

        unsafe {
            let path_str = NSString::from_str(&abs.to_string_lossy());
            let url = NSURL::fileURLWithPath(&path_str);

            let request = VNRecognizeTextRequest::new();
            request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
            request.setAutomaticallyDetectsLanguage(true);

            let handler = VNImageRequestHandler::initWithURL_options(
                VNImageRequestHandler::alloc(),
                &url,
                &NSDictionary::new(),
            );

            let perform_result = handler.performRequests_error(
                &objc2_foundation::NSArray::from_slice(&[
                    objc2::runtime::ProtocolObject::from_ref(&*request).cast()
                ])
            );
            perform_result.map_err(|e| OcrError::Recognize {
                detail: format!("performRequests: {e:?}"),
            })?;

            let observations = request.results().ok_or_else(|| OcrError::Recognize {
                detail: "results() returned nil".into(),
            })?;

            let mut text_lines: Vec<String> = Vec::new();
            let mut words: Vec<OcrWord> = Vec::new();
            let mut total_conf: f64 = 0.0;
            let mut conf_count: usize = 0;

            for line_idx in 0..observations.count() {
                let obs: Retained<VNRecognizedTextObservation> =
                    observations.objectAtIndex(line_idx).cast();
                let candidates = obs.topCandidates(1);
                if candidates.count() == 0 { continue; }
                let candidate = candidates.objectAtIndex(0);
                let line_text = candidate.string().to_string();
                let line_conf = candidate.confidence() as f64;

                total_conf += line_conf;
                conf_count += 1;
                text_lines.push(line_text.clone());

                let line_u32 = line_idx as u32;
                let candidate_str = candidate.string();
                let total_len = candidate_str.len();
                let mut byte_pos: usize = 0;
                for word in line_text.split_whitespace() {
                    let word_len = word.len();
                    let range = NSRange { location: byte_pos, length: word_len };
                    if byte_pos + word_len > total_len {
                        warn!("vision word range out of bounds; skipping");
                        break;
                    }
                    match candidate.boundingBoxForRange_error(range) {
                        Ok(rect_obs) => {
                            let r = rect_obs.boundingBox();
                            // Vision returns normalized 0..1 with origin BOTTOM-LEFT.
                            // Convert to TOP-LEFT for our schema convention.
                            let bbox = BBox {
                                x: r.origin.x as f32,
                                y: (1.0 - (r.origin.y + r.size.height)) as f32,
                                w: r.size.width as f32,
                                h: r.size.height as f32,
                            };
                            words.push(OcrWord {
                                text: word.to_string(),
                                bbox,
                                confidence: line_conf,
                                line: line_u32,
                            });
                        }
                        Err(e) => {
                            debug!("boundingBoxForRange failed for '{word}': {e:?}");
                        }
                    }
                    byte_pos += word_len + 1; // +1 for the whitespace; ASCII assumption
                }
            }

            let text = text_lines.join("\n");
            let avg_conf = if conf_count > 0 { total_conf / conf_count as f64 } else { 0.0 };

            Ok(OcrResult {
                text,
                words,
                language: "auto".to_string(),
                confidence: avg_conf,
            })
        }
    }
}
```

**Step 2: Smoke test (Mac only)**

Add a `#[cfg(test)]` smoke test at the bottom of `vision.rs`:

```rust
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn vision_backend_constructs_and_reports_version() {
        let b = VisionBackend::new().expect("construct");
        assert_eq!(b.name(), "Vision");
        let v = b.engine_version();
        assert!(v.starts_with("Vision (macOS "));
    }

    // Real recognize test requires a fixture image — see tests/integration_test.rs
}
```

**Step 3: Build on Mac**

```bash
cargo build -p snk-ocr --target aarch64-apple-darwin
cargo test -p snk-ocr --target aarch64-apple-darwin vision_backend
```

Expected: builds, smoke test passes. The real recognition test ships in T14 (integration test rewrite).

If selector names differ from the spike's findings, update accordingly. Common drift points: `setRecognitionLevel:` vs `setRecognitionLevel_`, `boundingBoxForRange:error:` vs `boundingBoxForRange_error`. Always match what Spike B documented in the Findings section.

**Step 4: Commit**

```bash
git add crates/snk-ocr/src/vision.rs
git diff --cached
git commit -m "feat(ocr): VisionBackend — VNRecognizeTextRequest via objc2-vision"
```

---

### Task 8: `WinOcrBackend` (Windows)

**Files:**
- Create: `crates/snk-ocr/src/winocr.rs`
- Modify: `crates/snk-ocr/src/lib.rs` — add `#[cfg(target_os = "windows")] pub mod winocr;` (plan amendment 2026-05-26: T5 left this declaration to T8 so the file-vs-declaration order is consistent)

**Step 1: Implement the backend**

```rust
#![cfg(target_os = "windows")]

use std::path::Path;

use tracing::{debug, warn};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::{FileAccessMode, StorageFile};

use crate::backend::{BBox, OcrBackend, OcrResult, OcrWord};
use crate::OcrError;

pub struct WinOcrBackend {
    engine: OcrEngine,
}

impl WinOcrBackend {
    pub fn new() -> Result<Self, OcrError> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .or_else(|_| {
                let en = Language::CreateLanguage(&HSTRING::from("en-US"))
                    .map_err(|e| OcrError::NoRecognizerLanguage {
                        detail: format!("CreateLanguage(en-US): {e}"),
                    })?;
                OcrEngine::TryCreateFromLanguage(&en)
                    .map_err(|e| OcrError::NoRecognizerLanguage {
                        detail: format!("TryCreateFromLanguage(en-US): {e}"),
                    })
            })?;
        Ok(Self { engine })
    }
}

impl OcrBackend for WinOcrBackend {
    fn name(&self) -> &'static str { "Windows.Media.Ocr" }

    fn engine_version(&self) -> String {
        // Windows OS build via env or registry — keep simple, default to "Windows.Media.Ocr".
        // For audit purposes we read CurrentBuildNumber from the registry; if it fails,
        // fall back to the static name.
        let build = win_build_number().unwrap_or_else(|| "unknown".to_string());
        format!("Windows.Media.Ocr (10.0.{build})")
    }

    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let abs = image_path.canonicalize().map_err(|e| OcrError::ImageLoad {
            path: image_path.display().to_string(),
            detail: e.to_string(),
        })?;
        // Plan amendment (Spike A finding approved 2026-05-26): Windows
        // canonicalize() returns extended-length namespace paths prefixed with
        // \\?\, which StorageFile::GetFileFromPathAsync rejects with HRESULT
        // 0x800700A1. Strip the prefix before constructing the HSTRING.
        let abs_str = abs.to_string_lossy();
        let stripped = abs_str.strip_prefix(r"\\?\").unwrap_or(&abs_str);
        let path = HSTRING::from(stripped);

        // Plan amendment (Spike A finding approved 2026-05-26): windows-rs 0.62
        // removed `IAsyncOperation::get()`. Use the inherent `.join()` method
        // on each of the four async types — no `windows-future` dep, no trait import.
        let file = StorageFile::GetFileFromPathAsync(&path)
            .map_err(|e| OcrError::ImageLoad { path: image_path.display().to_string(), detail: format!("GetFileFromPathAsync: {e}") })?
            .join()
            .map_err(|e| OcrError::ImageLoad { path: image_path.display().to_string(), detail: format!("await GetFileFromPathAsync: {e}") })?;

        let stream = file.OpenAsync(FileAccessMode::Read)
            .and_then(|op| op.join())
            .map_err(|e| OcrError::ImageLoad { path: image_path.display().to_string(), detail: format!("OpenAsync: {e}") })?;

        let decoder = BitmapDecoder::CreateAsync(&stream)
            .and_then(|op| op.join())
            .map_err(|e| OcrError::ImageLoad { path: image_path.display().to_string(), detail: format!("BitmapDecoder: {e}") })?;

        let pixel_width = decoder.PixelWidth().unwrap_or(1) as f32;
        let pixel_height = decoder.PixelHeight().unwrap_or(1) as f32;

        let bitmap = decoder.GetSoftwareBitmapAsync()
            .and_then(|op| op.join())
            .map_err(|e| OcrError::ImageLoad { path: image_path.display().to_string(), detail: format!("GetSoftwareBitmap: {e}") })?;

        let result = self.engine.RecognizeAsync(&bitmap)
            .and_then(|op| op.join())
            .map_err(|e| OcrError::Recognize { detail: format!("RecognizeAsync: {e}") })?;

        let lines = result.Lines().map_err(|e| OcrError::Recognize { detail: format!("Lines: {e}") })?;
        let line_count = lines.Size().unwrap_or(0);

        let mut text_lines: Vec<String> = Vec::new();
        let mut words: Vec<OcrWord> = Vec::new();

        // Plan amendment (Spike A finding approved 2026-05-26): Windows.Media.Ocr
        // does NOT expose any Confidence accessor on OcrWord OR OcrLine (verified
        // by reading windows-0.62.2 source). Hard-code line-broadcast confidence
        // at 0.85 — this is the documented asymmetry vs Vision.
        const WIN_OCR_HEURISTIC_CONF: f64 = 0.85;

        for li in 0..line_count {
            let line = match lines.GetAt(li) {
                Ok(l) => l,
                Err(e) => { debug!("line {li} GetAt err: {e}"); continue; }
            };
            let line_text = line.Text().map(|h| h.to_string_lossy()).unwrap_or_default();
            text_lines.push(line_text.clone());

            let line_words = line.Words().map_err(|e| OcrError::Recognize { detail: format!("Words: {e}") })?;
            for wi in 0..line_words.Size().unwrap_or(0) {
                let w = match line_words.GetAt(wi) {
                    Ok(w) => w,
                    Err(_) => continue,
                };
                let wt = w.Text().map(|h| h.to_string_lossy()).unwrap_or_default();
                let r = match w.BoundingRect() {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let bbox = BBox {
                    x: r.X / pixel_width,
                    y: r.Y / pixel_height,
                    w: r.Width / pixel_width,
                    h: r.Height / pixel_height,
                };
                words.push(OcrWord { text: wt, bbox, confidence: WIN_OCR_HEURISTIC_CONF, line: li });
            }
        }

        let text = text_lines.join("\n");
        let language = self.engine.RecognizerLanguage()
            .ok()
            .and_then(|l| l.LanguageTag().ok().map(|h| h.to_string_lossy()))
            .unwrap_or_else(|| "auto".to_string());

        Ok(OcrResult { text, words, language, confidence: WIN_OCR_HEURISTIC_CONF })
    }
}

fn win_build_number() -> Option<String> {
    // Plan amendment 2026-05-26 (winocr-pc, T8 follow-up): the original code
    // read `std::env::var("OS_BUILD")` — Windows does NOT set that variable,
    // so engine_version() reliably returned "10.0.unknown". RtlGetVersion is
    // the authoritative source (ntdll, not subject to the GetVersionEx
    // app-compat lying Microsoft bolted on in 8.1+). Returns NTSTATUS == 0
    // (STATUS_SUCCESS) on success. Requires `windows` features:
    // `Wdk_System_SystemServices`, `Win32_System_SystemInformation`, `Win32_Foundation`.
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    let status = unsafe { RtlGetVersion(&mut info) };
    if status.0 == 0 {
        Some(info.dwBuildNumber.to_string())
    } else {
        None
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn winocr_backend_constructs_and_reports_version() {
        // If the host machine has no language packs, this can legit fail with NoRecognizerLanguage.
        // We accept either Ok or NoRecognizerLanguage in CI; flaky path is logged not failed.
        match WinOcrBackend::new() {
            Ok(b) => {
                assert_eq!(b.name(), "Windows.Media.Ocr");
                assert!(b.engine_version().starts_with("Windows.Media.Ocr ("));
            }
            Err(OcrError::NoRecognizerLanguage { detail }) => {
                eprintln!("test machine has no recognizer language available: {detail}");
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}
```

**Step 2: Build on Windows**

```bash
cargo build -p snk-ocr --target x86_64-pc-windows-msvc
cargo test -p snk-ocr --target x86_64-pc-windows-msvc winocr_backend
```

Expected: builds; smoke passes (or NoRecognizerLanguage is logged if the runner machine has no language packs installed).

**Step 3: Commit**

```bash
git add crates/snk-ocr/src/winocr.rs
git diff --cached
git commit -m "feat(ocr): WinOcrBackend — Windows.Media.Ocr via windows-rs"
```

---

### Task 9: Refactor `OcrQueue` to use `Box<dyn OcrBackend>`, drop sidecar references

**Files:**
- Modify: `crates/snk-ocr/src/queue.rs`
- Modify: `crates/snk-ocr/src/plugin.rs` — minimum-touch update to keep workspace building (plan amendment 2026-05-26 — pre-flight catch). The existing `let queue = OcrQueue::start(Arc::clone(&db), root);` call uses the OLD 2-arg signature. T9 changes the signature to 4 args (backend, db, root, emit_ready) which plugin.rs can't supply yet (backend selection is T10, emit_ready callback is T10). Swap the line to `let queue = OcrQueue::disabled();` so the workspace builds through the T9→T10 window. T10 then rewrites plugin.rs end-to-end with the real backend selection. **Do NOT touch the `crate::sidecar::set_bundled_resource_dir(dir)` call in plugin.rs in T9** — that stays until T10's rewrite (sidecar.rs is still on disk until T13).

**Step 1: Rewrite queue**

```rust
use std::sync::Arc;

use snk_library::Db;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::backend::{OcrBackend, OcrResult};

pub struct OcrQueue {
    tx: mpsc::UnboundedSender<OcrJob>,
}

struct OcrJob {
    capture_id: String,
    image_path: std::path::PathBuf,
}

impl OcrQueue {
    pub fn start(
        backend: Arc<dyn OcrBackend>,
        db: Arc<Db>,
        library_root: std::path::PathBuf,
        emit_ready: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(worker(rx, backend, db, library_root, emit_ready));
        Self { tx }
    }

    pub fn enqueue(&self, capture_id: String, image_path: std::path::PathBuf) {
        if self.tx.send(OcrJob { capture_id, image_path }).is_err() {
            error!("ocr queue closed");
        }
    }
}

async fn worker(
    mut rx: mpsc::UnboundedReceiver<OcrJob>,
    backend: Arc<dyn OcrBackend>,
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    emit_ready: Arc<dyn Fn(&str) + Send + Sync>,
) {
    info!(backend = backend.name(), "ocr worker started");
    while let Some(job) = rx.recv().await {
        let full_path = library_root.join(&job.image_path);
        let backend_clone = Arc::clone(&backend);
        let db_clone = Arc::clone(&db);
        let cap_id = job.capture_id.clone();
        let emit = Arc::clone(&emit_ready);

        // FFI calls are kept off the tokio runtime — Vision is fast but synchronous;
        // WinOcr bridges an async UWP API but still benefits from isolation.
        let result = tokio::task::spawn_blocking(move || backend_clone.recognize(&full_path)).await;

        match result {
            Ok(Ok(out)) => {
                if let Err(e) = persist_and_index(&db_clone, &cap_id, &out, backend.name(), &backend.engine_version()) {
                    error!(capture_id = %cap_id, error = %e, "persist ocr failed");
                    continue;
                }
                emit(&cap_id);
                info!(capture_id = %cap_id, chars = out.text.len(), words = out.words.len(), "ocr indexed");
            }
            Ok(Err(e)) => error!(capture_id = %cap_id, error = ?e, "backend recognize failed"),
            Err(e) => error!(capture_id = %cap_id, error = %e, "ocr task panicked"),
        }
    }
    info!("ocr worker stopped");
}

fn persist_and_index(
    db: &Db,
    capture_id: &str,
    out: &OcrResult,
    backend_name: &str,
    engine_version: &str,
) -> Result<(), String> {
    // Use the qualified engine string the caller passed in; backend_name is for logging only.
    let _ = backend_name;
    snk_library::ocr::upsert_full(
        db, capture_id, &out.text, &out.language, out.confidence, &out.words, engine_version,
    ).map_err(|e| e.to_string())?;
    // Re-index for FTS (unchanged from Phase 5).
    let cap = snk_library::captures::get(db, capture_id).map_err(|e| e.to_string())?;
    snk_library::search::index_capture(
        db, capture_id,
        cap.source_app.as_deref(), cap.source_window_title.as_deref(),
        Some(&out.text), None,
    ).map_err(|e| e.to_string())?;
    Ok(())
}
```

**Step 2: Update plugin.rs to keep workspace building**

In `crates/snk-ocr/src/plugin.rs`, find the line:

```rust
let queue = OcrQueue::start(Arc::clone(&db), root);
```

Replace it with:

```rust
// T9 transition: queue is disabled until T10 wires real backend selection.
let queue = OcrQueue::disabled();
let _ = (Arc::clone(&db), root); // suppress unused warnings until T10
```

OR if T10 is being claimed by you alongside T9, skip the disabled-stub and go straight to T10's full rewrite — bundle the two task commits together. Either way works; the disabled-stub option is simpler if T9 and T10 are claimed by different implementers.

Effect: OCR pipeline is non-functional in the T9→T10 window (captures don't trigger OCR), but the workspace builds and tests pass. Acceptable interim state on a feature branch.

**Step 3: Build**

```bash
cargo build -p snk-ocr
cargo test -p snk-ocr
```

Expected: both pass clean. If `snk_library::ocr::upsert_full` or `snk_library::search::index_capture` signatures don't match, check T3 and Phase 5's `search.rs` and adjust.

**Step 4: Commit**

```bash
git add crates/snk-ocr/src/queue.rs crates/snk-ocr/src/plugin.rs
git diff --cached
git commit -m "refactor(ocr): OcrQueue takes Box<dyn OcrBackend>; emit-ready callback"
```

---

### Task 10: Plugin init wires backend selection + `ocr:ready` event + `get_ocr_words` command

**Files:**
- Modify: `crates/snk-ocr/src/plugin.rs`
- Modify: `crates/snk-ocr/build.rs` — append `"get_ocr_words"` to the `COMMANDS` array
- Modify: `crates/snk-ocr/permissions/default.toml` — append `"allow-get-ocr-words"` to the `permissions` array

Plan amendment 2026-05-26 (winocr-pc post-T10 catch — landed as `fix(ocr): expose get_ocr_words via build.rs COMMANDS + default capability`):

1. **build.rs `COMMANDS`** — Tauri's `tauri_plugin::Builder::new(COMMANDS).build()` autogenerates the per-command permission files under `crates/snk-ocr/permissions/autogenerated/commands/`. Without the array update, the `allow-get-ocr-words` permission file never gets generated.
2. **`default.toml` permissions array** — this is the SOURCE-OF-TRUTH bundle that selects which command permissions roll into the `snk-ocr:default` capability. It is NOT autogenerated; it's hand-maintained per-plugin. Without adding `"allow-get-ocr-words"` here, the autogenerated permission file exists but isn't included in `snk-ocr:default`, and the frontend invoke fails with a capability denial.

Cross-plugin convention check: `snk-annotate/permissions/default.toml` lists `["allow-save-annotation", "allow-derive-capture"]`; `snk-clipboard/permissions/default.toml` lists all three of its commands. Every plugin enumerates its full command set in both `build.rs COMMANDS` AND `permissions/default.toml`.

**Step 1: Rewrite plugin**

```rust
use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};

use snk_library::ocr::OcrWord;
use snk_library::LibraryState;

use crate::backend::OcrBackend;
use crate::queue::OcrQueue;
use crate::OcrError;

pub struct OcrState {
    pub queue: OcrQueue,
    pub backend_name: &'static str,
    pub backend_version: String,
    pub last_error: std::sync::Mutex<Option<OcrError>>,
}

#[tauri::command]
pub fn ocr_status<R: Runtime>(app: tauri::AppHandle<R>) -> Result<serde_json::Value, String> {
    let state = app.try_state::<OcrState>().ok_or("ocr state missing")?;
    let last_err = state.last_error.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "backend": state.backend_name,
        "version": state.backend_version,
        "last_error": last_err.as_ref().map(|e| serde_json::to_value(e).unwrap_or_default()),
    }))
}

#[tauri::command]
pub fn get_ocr_words<R: Runtime>(
    app: tauri::AppHandle<R>,
    capture_id: String,
) -> Result<Vec<OcrWord>, String> {
    let lib = app.state::<LibraryState>();
    let row = snk_library::ocr::get(&lib.db, &capture_id).map_err(|e| e.to_string())?;
    Ok(row.and_then(|r| r.words).unwrap_or_default())
}

fn build_backend() -> Result<Arc<dyn OcrBackend>, OcrError> {
    // Plan amendment 2026-05-26: original code used `return Ok(...)` in each
    // cfg-block, which trips clippy's needless_return lint. Each expression is
    // already the final expression of its block AND the function — implicit
    // return is the idiomatic form. Quality-sentinel caught the clippy failure
    // post-T13 (sidecar.rs deletion unmasked the warning by clearing 9 other
    // dead-code warnings that dominated clippy's output).
    #[cfg(target_os = "macos")]
    {
        let b = crate::vision::VisionBackend::new()?;
        Ok(Arc::new(b) as Arc<dyn OcrBackend>)
    }
    #[cfg(target_os = "windows")]
    {
        let b = crate::winocr::WinOcrBackend::new()?;
        Ok(Arc::new(b) as Arc<dyn OcrBackend>)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(OcrError::BackendUnavailable {
            reason: "no OCR backend available on this platform".into(),
        })
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-ocr")
        .invoke_handler(tauri::generate_handler![ocr_status, get_ocr_words])
        .setup(|app, _api| {
            let lib_state = app.state::<LibraryState>();
            let db = lib_state.db.clone();
            let root = lib_state.root.clone();

            let app_handle_for_emit = app.app_handle().clone();
            let emit_ready: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |capture_id: &str| {
                if let Err(e) = app_handle_for_emit.emit("ocr:ready", capture_id) {
                    tracing::warn!(error = %e, "failed to emit ocr:ready");
                }
            });

            match build_backend() {
                Ok(backend) => {
                    let backend_name = backend.name();
                    let backend_version = backend.engine_version();
                    tracing::info!(backend = backend_name, version = %backend_version, "ocr backend ready");

                    let queue = OcrQueue::start(Arc::clone(&backend), Arc::clone(&db), root, emit_ready);
                    app.manage(OcrState {
                        queue,
                        backend_name,
                        backend_version,
                        last_error: std::sync::Mutex::new(None),
                    });

                    // Subscribe to capture:saved (Phase 1 contract — unchanged).
                    let db_for_listener = Arc::clone(&db);
                    let app_handle = app.app_handle().clone();
                    app_handle.clone().listen("capture:saved", move |event| {
                        let capture_id = event.payload().trim_matches('"').to_string();
                        if capture_id.is_empty() { return; }
                        match snk_library::captures::get(&db_for_listener, &capture_id) {
                            Ok(capture) => {
                                let image_path = std::path::PathBuf::from(&capture.file_path);
                                if let Some(ocr) = app_handle.try_state::<OcrState>() {
                                    ocr.queue.enqueue(capture_id, image_path);
                                }
                            }
                            Err(e) => tracing::warn!(capture_id, error = %e, "could not look up capture for ocr"),
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = ?e, "ocr backend unavailable; OCR disabled this session");
                    // Manage an OcrState with no queue so the get_ocr_words/ocr_status commands
                    // still respond gracefully instead of "state missing".
                    let (dummy_tx, dummy_rx) = tokio::sync::mpsc::unbounded_channel();
                    drop(dummy_rx); // close immediately
                    let queue = OcrQueue { tx: dummy_tx };
                    let err_clone = e.clone();
                    app.manage(OcrState {
                        queue,
                        backend_name: "none",
                        backend_version: "unavailable".into(),
                        last_error: std::sync::Mutex::new(Some(err_clone)),
                    });
                }
            }

            Ok(())
        })
        .build()
}
```

Note: the `dummy` queue construction reaches into private `OcrQueue` fields. Add `pub(crate)` to the `tx` field on `OcrQueue` in `queue.rs`, OR add a `OcrQueue::disabled()` constructor — the latter is cleaner:

In `queue.rs`:

```rust
impl OcrQueue {
    pub fn disabled() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);
        Self { tx }
    }
}
```

Then `plugin.rs` uses `OcrQueue::disabled()`.

**Step 2: Build full app**

```bash
cargo build -p snk-ocr
cargo check -p app  # adjust crate name if app uses a different name
```

Expected: builds clean. The app crate doesn't need code changes for this task — it already loads `snk-ocr::init()`.

**Step 3: Commit**

```bash
git add crates/snk-ocr/src/plugin.rs crates/snk-ocr/src/queue.rs
git diff --cached
git commit -m "feat(ocr): plugin wires backend selection, ocr:ready event, get_ocr_words"
```

---

### Task 11: TS bindings — `packages/snk-ocr` (CREATE) with `ocr:ready` + `getOcrWords`

**Files:**
- Create: `packages/snk-ocr/package.json` (mirror `packages/snk-capture/package.json` shape — name `@snk/ocr`)
- Create: `packages/snk-ocr/tsconfig.json` (mirror `packages/snk-capture/tsconfig.json`)
- Create: `packages/snk-ocr/vitest.config.ts` (mirror `packages/snk-capture/vitest.config.ts`)
- Create: `packages/snk-ocr/src/index.ts` (the bindings — see Step 2)
- Create: `packages/snk-ocr/src/index.test.ts` (mirror `packages/snk-capture/src/index.test.ts` pattern for basic mock-invoke shape verification)
- Maybe: `pnpm-lock.yaml` if pnpm regenerates when the new workspace package registers

Plan amendment 2026-05-26 (vision-mac during T11): `packages/snk-ocr/` was deleted by PR #129 (About panel cluster, closing issue #62 — "the unused `@snk/ocr` TS package, sole export was a stub binding to a stub Rust command"). T11 must CREATE the package, not modify it. The 4-file minimum + tests for consistency mirror `packages/snk-capture/`.

**Step 1: Verify package doesn't exist**

```bash
ls packages/snk-ocr/   # expected: directory not present
```

Confirms the plan-amendment context (deleted by PR #129). Proceed to create from scratch using `packages/snk-capture/` as the structural template.

**Step 2: Create `packages/snk-ocr/src/index.ts`**

New file contents:

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface BBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface OcrWord {
  text: string;
  bbox: BBox;
  confidence: number;
  line: number;
}

export interface OcrStatus {
  backend: string;
  version: string;
  last_error: { kind: string; [key: string]: unknown } | null;
}

export async function getOcrWords(captureId: string): Promise<OcrWord[]> {
  return invoke<OcrWord[]>('plugin:snk-ocr|get_ocr_words', { captureId });
}

export async function ocrStatus(): Promise<OcrStatus> {
  return invoke<OcrStatus>('plugin:snk-ocr|ocr_status');
}

export function onOcrReady(handler: (captureId: string) => void): Promise<UnlistenFn> {
  return listen<string>('ocr:ready', (e) => handler(e.payload));
}
```

(Adjust import path of `@tauri-apps/api/core` vs `@tauri-apps/api/tauri` per the version pinned in `package.json`.)

**Step 3: Verify TS build + tests**

```bash
pnpm -F @snk/ocr build
pnpm -F @snk/ocr test  # if it has tests
```

Expected: type-check clean.

**Step 4: Permissions — already handled by T10 follow-up**

Plan amendment 2026-05-26: the original plan said to edit `app/src-tauri/capabilities/default.json` — wrong layer for this repo. `capabilities/default.json` references `snk-ocr:default` as a plugin-level alias; the actual permission enumeration lives in `crates/snk-ocr/permissions/default.toml`. Adding `allow-get-ocr-words` to that toml was bundled into winocr-pc's T10 follow-up commit `fix(ocr): expose get_ocr_words via build.rs COMMANDS + default capability`.

Verification step before staging T11:

```bash
grep -F 'allow-get-ocr-words' crates/snk-ocr/permissions/default.toml
```

Should return one match. If it doesn't, the T10 follow-up hasn't landed yet — coordinate with the team-lead before proceeding.

**T11 stages NO permission files** — those are owned by the T10 follow-up commit.

**Step 5: Commit**

```bash
git add packages/snk-ocr/package.json packages/snk-ocr/tsconfig.json packages/snk-ocr/vitest.config.ts packages/snk-ocr/src/index.ts packages/snk-ocr/src/index.test.ts
# Also pnpm-lock.yaml if pnpm regenerated it cleanly for the new workspace package:
git add pnpm-lock.yaml   # only if `git diff --cached` shows only @snk/ocr-related additions; otherwise leave to whoever's deps work is in flight
git diff --cached
git commit -m "feat(ocr): TS bindings — getOcrWords, onOcrReady, OcrStatus"
```

---

### Task 12: Replace `crates/snk-ocr/tests/integration_test.rs` with native-backend test

**Files:**
- Modify: `crates/snk-ocr/tests/integration_test.rs`
- Create (if missing): `crates/snk-ocr/tests/fixtures/hello-world.png` (small PNG of the text "hello world")

**Step 1: Create the fixture image**

A trivial way: open any image editor (Paint, Preview, GIMP), white background, type "hello world" in a clear sans-serif font at ~48pt, save as `hello-world.png`. Keep it under 5 KB.

Alternatively, generate programmatically with `image` crate (one-shot script outside the project, then copy the PNG into the fixtures dir).

```bash
ls crates/snk-ocr/tests/fixtures/  # may not exist yet
mkdir -p crates/snk-ocr/tests/fixtures
# Place hello-world.png in there.
```

**Step 2: Rewrite `integration_test.rs`**

```rust
//! Integration tests for snk-ocr — exercises the active platform backend.
//! Vision on macOS, WinOcr on Windows. No subprocess.

use std::path::PathBuf;

use snk_ocr::backend::OcrBackend;

#[cfg(target_os = "macos")]
fn make_backend() -> Box<dyn OcrBackend> {
    Box::new(snk_ocr::vision::VisionBackend::new().expect("Vision backend should construct"))
}

#[cfg(target_os = "windows")]
fn make_backend() -> Box<dyn OcrBackend> {
    match snk_ocr::winocr::WinOcrBackend::new() {
        Ok(b) => Box::new(b),
        Err(e) => {
            eprintln!("WinOcrBackend unavailable on this machine; skipping: {e:?}");
            std::process::exit(0); // soft-skip — clean exit, no panic
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn make_backend() -> Box<dyn OcrBackend> {
    panic!("snk-ocr integration tests require macOS or Windows");
}

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests"); p.push("fixtures"); p.push(name);
    p
}

#[test]
fn recognize_hello_world_returns_text_and_words() {
    let b = make_backend();
    let r = b.recognize(&fixture("hello-world.png")).expect("recognize");
    let text_lower = r.text.to_lowercase();
    assert!(text_lower.contains("hello"), "text should contain 'hello'; got {:?}", r.text);
    assert!(text_lower.contains("world"), "text should contain 'world'; got {:?}", r.text);
    assert!(!r.words.is_empty(), "should return at least one word");
    let first = &r.words[0];
    assert!(first.bbox.w > 0.0 && first.bbox.w <= 1.0, "bbox w should be normalized 0..1; got {}", first.bbox.w);
    assert!(first.bbox.h > 0.0 && first.bbox.h <= 1.0);
}

#[test]
fn engine_version_is_populated() {
    let b = make_backend();
    let v = b.engine_version();
    assert!(!v.is_empty());
}
```

**Step 3: Run the integration test on host**

```bash
cargo test -p snk-ocr --test integration_test
```

Expected: passes on Mac (Vision) and on Windows (WinOcr, unless the runner has no recognizer language). On unsupported platforms the suite soft-skips with exit 0.

**Step 4: Commit**

```bash
git add crates/snk-ocr/tests/integration_test.rs crates/snk-ocr/tests/fixtures/hello-world.png
git diff --cached
git commit -m "test(ocr): integration test against native backend with hello_world fixture"
```

---

### Task 13: Tesseract cleanse — delete `sidecar.rs`, `build.rs`, update lib.rs

**Files:**
- Delete: `crates/snk-ocr/src/sidecar.rs`
- Delete: `crates/snk-ocr/build.rs` (if Tesseract-only)
- Modify: `crates/snk-ocr/src/lib.rs` — remove the `mod sidecar;` private declaration that T5 left in place (plan amendment 2026-05-26 pre-flight catch — original plan said this was "already done in T5" but T5's amended scope KEEPS the private mod declaration; T13 is the actual deletion point)

**Step 1: Pre-flight — confirm no consumers remain**

```bash
git grep -nE 'crate::sidecar|mod sidecar|use crate::sidecar|snk_ocr::sidecar' -- 'crates/snk-ocr/'
```

Expected: ONLY two matches — the `mod sidecar;` line in `crates/snk-ocr/src/lib.rs` and the file itself. If `crate::sidecar::*` callsites appear in queue.rs or plugin.rs, T9 or T10 didn't complete their refactor cleanly — stop and ping team-lead. **Do not delete sidecar.rs until queue.rs and plugin.rs have stopped referencing it.**

**Step 2: Inspect `build.rs`**

```bash
cat crates/snk-ocr/build.rs
```

If it only generates Tauri plugin metadata (via `tauri_plugin::Builder` or `tauri_build`), keep it. If it has Tesseract-specific logic (env-var passing, downloading, etc.), delete it. Most Tauri-plugin `build.rs` files are 3 lines — keep.

**Step 3: Delete the sidecar file + lib.rs declaration**

```bash
git rm crates/snk-ocr/src/sidecar.rs
```

Edit `crates/snk-ocr/src/lib.rs` — remove the three lines:

```rust
// Sidecar module kept (private) until T9 rewrites queue.rs and T10 rewrites plugin.rs.
// Both still reference crate::sidecar::* internally. Full deletion happens in T13.
mod sidecar;
```

**Step 4: Verify lib.rs clean**

```bash
git grep -nE 'sidecar|mod sidecar' -- crates/snk-ocr/src/lib.rs
```

Expected: no output.

**Step 4: Build + test**

```bash
cargo build -p snk-ocr
cargo test -p snk-ocr
```

Expected: builds + tests pass on the host platform.

**Step 5: Commit**

```bash
git add crates/snk-ocr/src/lib.rs
git diff --cached
git commit -m "chore(cleanse): delete snk-ocr/src/sidecar.rs"
```

---

### Task 14: Tesseract cleanse — `tauri.conf.json`, resources dir, `.gitignore`

**Files:**
- Modify: `app/src-tauri/tauri.conf.json` (line ~120 — `bundle.resources` entry)
- Delete: `app/src-tauri/resources/tesseract/` (entire directory)
- Modify: `.gitignore` (lines 20–24 — tesseract paths)

**Step 1: Edit `tauri.conf.json`**

Find and remove the line `"resources/tesseract/**/*": "tesseract/"` from the `bundle.resources` block. If the block becomes empty, remove the empty `bundle.resources` key entirely.

**Step 2: Delete the resources dir**

```bash
git rm -r app/src-tauri/resources/tesseract
```

If `app/src-tauri/resources/` becomes empty afterward, also delete it (keep if other plugins use it).

**Step 3: Edit `.gitignore`**

Remove lines 20–24 (the Tesseract block):

```
# Tesseract distribution copied in by CI before bundling (downloaded fresh per build).
... (the entire block)
app/src-tauri/resources/tesseract/*
!app/src-tauri/resources/tesseract/.placeholder
```

**Step 4: Build to confirm no broken references**

```bash
pnpm tauri build --debug --no-bundle
```

Plan amendment 2026-05-26: `--no-bundle` skips the bundler + signing step (which is what changes across platforms and would prompt for Azure code signing on Windows) while still exercising the cargo build + tauri.conf.json parse + resource glob — that's all this task needs to verify. The full installer build is reserved for T18 (single end-of-PR-2 inspection).

Expected: build succeeds in ~1 min. If `tauri.conf.json` validation fails because `bundle.resources` becomes an empty object, remove the key entirely.

Optional fast schema check:

```bash
pnpm tauri info | head -50
```

Confirms tauri.conf.json parses cleanly against the schema.

**Step 5: Commit**

```bash
git add app/src-tauri/tauri.conf.json .gitignore
git diff --cached
git commit -m "chore(cleanse): remove Tesseract from tauri.conf bundle + gitignore"
```

---

### Task 15: Tesseract cleanse — `.github/workflows/release.yml`

**Files:**
- Modify: `.github/workflows/release.yml` (delete lines ~76–98 — the entire Windows Tesseract bundling step)

**Step 1: Inspect the step**

```bash
sed -n '70,105p' .github/workflows/release.yml
```

Identify the full step boundaries:
- Start: `- name: Bundle Tesseract (Windows)`
- End: just before the next `- name:` step or `- uses:` action

**Step 2: Delete the step**

Open `.github/workflows/release.yml` in an editor and remove the entire step including its `if:`, `env:`, and `run:` blocks. Also remove the preceding comment block (lines ~74–77 about why Tesseract is bundled).

**Step 3: Validate workflow syntax**

```bash
# If actionlint is installed:
actionlint .github/workflows/release.yml
# Or just trust the CI run — push will validate.
```

**Step 4: Commit**

```bash
git add .github/workflows/release.yml
git diff --cached
git commit -m "chore(cleanse): remove Tesseract bundling step from release workflow"
```

---

### Task 16: Tesseract cleanse — `README.md`, `CLAUDE.md`, `SettingsWindow.tsx`

**Files:**
- Modify: `README.md` (lines 25, 54–58, 129)
- Modify: `CLAUDE.md` (line 5)
- Modify: `app/src/windows/settings/SettingsWindow.tsx` (line 286 — OCR description)

**Step 1: Edit `README.md`**

- Line 25 (`Tesseract sidecar runs asynchronously on every capture`): replace with `Native OS OCR (Apple Vision on macOS, Windows.Media.Ocr on Windows) runs asynchronously on every capture`.
- Lines 54–58 (Tesseract install instructions and `SNK_TESSERACT_PATH` env): **delete the entire block.**
- Line 129 (plugin description `snk-ocr/    Tesseract sidecar + async OCR queue + retry`): replace with `snk-ocr/    Native OCR backends (Vision / Windows.Media.Ocr) + async queue`.

**Step 2: Edit `CLAUDE.md`**

Line 5 (`**Dev environment setup (toolchain versions, Tesseract install, per-OS prerequisites...`): replace `Tesseract install, ` with empty string — just `**Dev environment setup (toolchain versions, per-OS prerequisites...`.

Also: update the "Phase status" table at the bottom to add a Phase 10 row, e.g.:

| 10 | OCR surfaces: Vision + WinOcr + PII redact + Text Actions; full bundled-engine removal | In progress |

(Change to "Done" when the parent branch merges.)

**Step 3: Edit `SettingsWindow.tsx`**

Line 286 description: replace `"Automatically extract text from captures using Tesseract"` with `"Automatically extract text from captures using native OS OCR"`.

**Step 4: Commit**

```bash
git add README.md CLAUDE.md app/src/windows/settings/SettingsWindow.tsx
git diff --cached
git commit -m "chore(cleanse): remove Tesseract references from README, CLAUDE.md, settings UI"
```

---

### Task 17: Tesseract cleanse — `SNK_TESSERACT_PATH` sweep + `docs/release-signing.md`

**Files:**
- Any file still referencing `SNK_TESSERACT_PATH` (sweep below).
- `docs/release-signing.md` — delete the entire `## Tesseract bundling` section AND the "macOS bundles are not yet self-contained for OCR" paragraph that follows from it. Plan amendment 2026-05-26: winocr-pc's pre-work `git grep` found this section is not covered by T13/T14/T15/T16 and would fail the T18 verification predicate.

**Step 1: Sweep**

```bash
git grep -n SNK_TESSERACT_PATH
git grep -n -i tesseract -- docs/release-signing.md
```

Expected after this task: zero results from both. If any other `SNK_TESSERACT_PATH` references remain (likely in inline doc comments missed elsewhere), delete those too.

**Step 2: Delete the release-signing Tesseract section**

In `docs/release-signing.md`, remove the entire `## Tesseract bundling` H2 section (multi-paragraph: choco install, sidecar resolve order, macOS dylib `install_name_tool` caveat) AND the immediately-following "macOS bundles are not yet self-contained for OCR" paragraph. The exact line range varies; use `git grep -n` to find boundaries.

**Step 3: Commit**

```bash
git add docs/release-signing.md <any-other-files-touched>
git diff --cached
git commit -m "chore(cleanse): purge Tesseract from release-signing docs + final env-var sweep"
```

If nothing changed (no SNK_TESSERACT_PATH refs AND no release-signing Tesseract section), skip the commit — but verify the predicate `git grep -i tesseract -- ':!docs/superpowers/**'` is clean before declaring no-op.

---

### Task 18: Cleanse verification gate

**Files:** none modified — verification only.

**Prerequisites — all of these MUST have landed first** (per amendments during PR-2 execution):

- T13 follow-on: `chore(cleanse): drop unused dev-deps from snk-ocr Cargo.toml` (commit `24f45e5` in PR-2 history). Removes `serial_test` and `tempfile` from snk-ocr dev-deps — they only existed for sidecar tests, which T13 deleted. Spec §8 checklist line for `crates/snk-ocr/Cargo.toml`.
- T17 follow-on: `chore(cleanse): reword release-strategy changelog to satisfy T18 predicate` (commit `5907b49`). `docs/superpowers/release-strategy.md` had "Tesseract" in a historical changelog row that's outside the T18 predicate's `docs/superpowers/{specs,plans,research,reviews}/**` exclusion list.
- T10 follow-on: `fix(ocr): expose get_ocr_words via build.rs COMMANDS + default capability` (commit `243091f`). Without this, the JS `invoke('plugin:snk-ocr|get_ocr_words')` from T11 bindings fails at runtime with a capability denial.

If any of the three above are missing, T18 fails. Either re-route to whoever's mid-flight, or land them as a single bundled `chore(cleanse): final spec §8 cleanups before T18` commit before this gate.

**Step 1: Run the cleanse predicate**

```bash
git grep -i tesseract -- ':!docs/superpowers/specs/**' ':!docs/superpowers/plans/**' ':!docs/superpowers/research/**' ':!docs/superpowers/reviews/**'
```

Expected output: **zero matches**. If anything appears, fix it before proceeding.

Also run:

```bash
git grep -i SNK_TESSERACT_PATH
git grep -i 'sidecar.rs'
```

Both should return zero matches.

**Step 2: Full workspace build + test + clippy**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean build + all tests pass + clippy clean on the host platform.

Plan amendment 2026-05-26 (quality-sentinel during T9 review): the clippy gate is added explicitly here because the cluster (T9 through T13) intentionally leaves sidecar.rs's symbols (`which`, `run_tesseract`, `invoke_tesseract`, `OcrOutput::confidence`) unreferenced from intermediate commits. T9 stops *reading* those symbols; T10 stops calling `crate::sidecar::set_bundled_resource_dir`; T13 deletes the file. Intermediate commits T9, T10, T11, T12 will FAIL `cargo clippy --workspace -- -D warnings` with 7 dead-code warnings each — that's expected interim state. T13 (`git rm sidecar.rs`) restores green clippy. **Spec-auditor and quality-sentinel: do NOT fail T10/T11/T12 individually for dead-code clippy warnings in sidecar.rs.** The cluster as a whole is the unit that must be clippy-green, enforced here.

**Step 3: Tauri build smoke**

```bash
pnpm tauri build --debug
```

Expected: successful debug build. Confirms Tesseract is not required to compile or bundle.

**Step 4: Confirm installer bundle does not contain `tesseract` directory**

After `pnpm tauri build --debug`, locate the bundled installer (path varies by platform). Inspect or extract it and verify no `tesseract/` directory or `*.traineddata` files inside.

- Windows NSIS: `target/release/bundle/nsis/snapper-keeper_*-setup.exe` — extract with `7z x` to inspect.
- Mac `.app`: `target/release/bundle/macos/snapper-keeper.app` — `find ... -name 'tesseract*'` returns nothing.

**Step 5: No commit needed**

This task is a verification gate. If anything failed, the gate blocks PR-2 merge. Fix root causes in earlier tasks.

---

### Task 19: PR-2 open

**Files:** none modified.

**Step 1: Push the branch**

```bash
git push -u origin feat/phase10-pr2-foundation
```

**Step 2: Open the PR against `feat/phase10-ocr-surfaces`**

```bash
gh pr create \
  --base feat/phase10-ocr-surfaces \
  --title "feat(phase10/PR2): foundation — native OCR backends + migration + Tesseract cleanse" \
  --body "$(cat <<'EOF'
## Summary

- New `OcrBackend` trait, `VisionBackend` (Mac) + `WinOcrBackend` (Win) — Tesseract sidecar gone.
- Migration V006: `ocr_text.words_json` + `ocr_text.engine` + `pii_spans` table (table populated in PR-3).
- New `snk-library::pii` module: CRUD on `pii_spans`.
- New `snk-pii` crate (stub; PR-3 fleshes out).
- New `ocr:ready` event + `get_ocr_words` command + TS bindings.
- Full Tesseract cleanse: sidecar.rs, bundled resources, CI bundling, README install, env override, settings description.

## Findings from spikes

- WinOcr engine construction: _<fill from Spike A findings>_
- WinOcr per-word confidence: _<fill from Spike A findings>_
- `objc2-vision` version resolved: _<fill from Spike B findings>_

## Test plan

- [ ] `cargo test --workspace` passes locally on Mac
- [ ] `cargo test --workspace` passes locally on Windows
- [ ] `pnpm tauri build --debug` succeeds on Mac
- [ ] `pnpm tauri build --debug` succeeds on Windows
- [ ] Installer bundle inspection confirms no `tesseract/` directory or `.traineddata` files
- [ ] `git grep -i tesseract -- ':!docs/superpowers/**'` returns zero
- [ ] Manual smoke: capture screenshot on Mac → `ocr_text.text` + `ocr_text.words_json` + `ocr_text.engine = "Vision (...)"` written within ~500ms
- [ ] Manual smoke: same on Windows → engine = `"Windows.Media.Ocr (...)"`

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Wait for CI; if green, merge with squash. The parent branch `feat/phase10-ocr-surfaces` now has the foundation.

---

## PR-3: PII plugin (`snk-pii`)

**Branch:** `feat/phase10-pr3-pii-plugin` off the now-updated `feat/phase10-ocr-surfaces` (PR-2 merged in).

Goal: implement `snk-pii` end-to-end. Scanner runs six regex matchers + post-filters, worker subscribes to `ocr:ready` and writes `pii_spans`, plugin commands serve the UI. Includes shared pixel-blur primitive in `snk-library` so PR-3 and the editor can both use it.

### Task 20: `snk-pii` crate scaffold (Cargo.toml, plugin.rs, types)

**Files:**
- Modify: `crates/snk-pii/Cargo.toml`
- Create: `crates/snk-pii/build.rs`
- Create: `crates/snk-pii/src/lib.rs` (replace stub)
- Create: `crates/snk-pii/src/plugin.rs`
- Create: `crates/snk-pii/src/error.rs`

**Step 1: Flesh out `Cargo.toml`**

```toml
[package]
name = "snk-pii"
version = "0.0.1"
links = "snk-pii"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[build-dependencies]
tauri-plugin = { workspace = true }

[dependencies]
snk-library = { path = "../snk-library" }
tauri.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
regex = "1"
once_cell = "1"

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create `build.rs`**

```rust
fn main() {
    tauri_plugin::Builder::new(&[]).build();
}
```

(Match the pattern used by other plugins, e.g. `crates/snk-ocr/build.rs`.)

**Step 3: Create `error.rs`**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
#[serde(tag = "kind")]
pub enum PiiError {
    #[error("ocr text missing for capture {capture_id}")]
    OcrMissing { capture_id: String },

    #[error("persist failed: {detail}")]
    Persist { detail: String },

    #[error("redact failed: {detail}")]
    Redact { detail: String },
}
```

**Step 4: Rewrite `lib.rs`**

```rust
//! snk-pii — PII detection plugin. Subscribes to `ocr:ready`, scans text,
//! persists `pii_spans` rows, emits `pii:scanned`. Frontend confirms or
//! dismisses suggestions per item.

pub mod error;
pub mod plugin;
pub mod scanner;
pub mod worker;

pub use error::PiiError;
pub use plugin::init;
```

**Step 5: Scaffold `plugin.rs` (full implementation in T25)**

```rust
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-pii")
        .setup(|_app, _api| Ok(()))
        .build()
}
```

**Step 6: Scaffold `scanner.rs` and `worker.rs` (real impls in T21–T25)**

```rust
// scanner.rs
//! Pattern matchers + post-filters for PII detection.
```

```rust
// worker.rs
//! Async worker that subscribes to ocr:ready and scans.
```

**Step 7: Build**

```bash
cargo build -p snk-pii
```

Expected: clean build.

**Step 8: Commit**

```bash
git add crates/snk-pii/Cargo.toml crates/snk-pii/build.rs crates/snk-pii/src/lib.rs crates/snk-pii/src/plugin.rs crates/snk-pii/src/scanner.rs crates/snk-pii/src/worker.rs crates/snk-pii/src/error.rs
git diff --cached
git commit -m "feat(pii): snk-pii crate scaffold (plugin + scanner/worker stubs)"
```

---

### Task 21: PII scanner — email, phone, credit_card (with Luhn)

**Files:**
- Modify: `crates/snk-pii/src/scanner.rs`

**Step 1: Write failing tests**

Append to `scanner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use snk_library::ocr::{BBox, OcrWord};
    use snk_library::pii::PiiCategory;

    fn w(text: &str, x: f32, line: u32) -> OcrWord {
        OcrWord {
            text: text.into(),
            bbox: BBox { x, y: 0.1, w: 0.08, h: 0.04 },
            confidence: 0.95,
            line,
        }
    }

    #[test]
    fn detects_email() {
        let text = "Contact alice@example.com for details.";
        let words = vec![w("Contact", 0.0, 0), w("alice@example.com", 0.1, 0), w("for", 0.3, 0)];
        let cands = scan(text, &words);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].category, PiiCategory::Email);
        assert_eq!(cands[0].matched_text, "alice@example.com");
    }

    #[test]
    fn rejects_email_with_bogus_tld() {
        // .xyzzy is not in the IANA TLD set
        let text = "weird@thing.xyzzy ok";
        let words = vec![w("weird@thing.xyzzy", 0.0, 0), w("ok", 0.2, 0)];
        let cands = scan(text, &words);
        assert!(cands.iter().all(|c| c.category != PiiCategory::Email));
    }

    #[test]
    fn detects_us_phone_dashed() {
        let text = "Call 555-867-5309 today";
        let words = vec![w("Call", 0.0, 0), w("555-867-5309", 0.1, 0), w("today", 0.3, 0)];
        let cands = scan(text, &words);
        assert!(cands.iter().any(|c| c.category == PiiCategory::Phone));
    }

    #[test]
    fn detects_credit_card_valid_luhn() {
        // 4111-1111-1111-1111 — valid Luhn test number
        let text = "Card: 4111 1111 1111 1111 exp 12/27";
        let words = vec![w("Card:", 0.0, 0), w("4111 1111 1111 1111", 0.1, 0)];
        let cands = scan(text, &words);
        assert!(cands.iter().any(|c| c.category == PiiCategory::CreditCard));
    }

    #[test]
    fn rejects_credit_card_invalid_luhn() {
        let text = "Ticket: 1234 5678 9012 3456 lookup";
        let words = vec![w("Ticket:", 0.0, 0), w("1234 5678 9012 3456", 0.1, 0)];
        let cands = scan(text, &words);
        assert!(cands.iter().all(|c| c.category != PiiCategory::CreditCard));
    }
}
```

**Step 2: Run — confirm fail**

```bash
cargo test -p snk-pii scanner::tests
```

Expected: FAIL — `scan` not defined.

**Step 3: Implement scanner**

Replace `scanner.rs` with:

```rust
use once_cell::sync::Lazy;
use regex::Regex;

use snk_library::ocr::{BBox, OcrWord};
use snk_library::pii::PiiCategory;

/// A detected PII candidate, before persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct PiiCandidate {
    pub category: PiiCategory,
    pub matched_text: String,
    pub bbox: BBox,
    pub confidence: f64,
}

pub fn scan(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    let mut out = Vec::new();
    out.extend(scan_email(text, words));
    out.extend(scan_phone(text, words));
    out.extend(scan_credit_card(text, words));
    // SSN, IP, API key added in T22.
    out.retain(|c| c.confidence >= 0.5);
    out
}

// --- Email ----------------------------------------------------------------

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,24}").unwrap()
});

fn scan_email(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    EMAIL_RE.find_iter(text)
        .filter(|m| valid_tld(m.as_str()))
        .map(|m| candidate_from_match(text, words, m.start()..m.end(), PiiCategory::Email, 0.90))
        .collect()
}

/// IANA TLD allow-list (curated subset for v1; expand as needed).
/// Source: https://data.iana.org/TLD/tlds-alpha-by-domain.txt
/// We ship a curated list of common TLDs to keep the binary small and the
/// post-filter strict — exotic TLDs (.xyz, .top, .icu) often appear in
/// false positives from random strings, so we keep the allow-list focused on
/// common ones. A future task can swap in the full IANA list if needed.
const COMMON_TLDS: &[&str] = &[
    "com", "org", "net", "edu", "gov", "mil", "int",
    "io", "co", "ai", "app", "dev", "info", "biz",
    "uk", "us", "ca", "au", "de", "fr", "jp", "cn", "in", "br",
    "es", "it", "nl", "se", "no", "fi", "dk", "pl", "ru", "kr", "mx",
];

fn valid_tld(email: &str) -> bool {
    let tld = email.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    COMMON_TLDS.contains(&tld.as_str())
}

// --- Phone ----------------------------------------------------------------

static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\+?\d{1,3}?[-.\s]?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}").unwrap()
});

fn scan_phone(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    PHONE_RE.find_iter(text)
        .filter(|m| {
            let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
            (10..=15).contains(&digits.len())
        })
        .map(|m| candidate_from_match(text, words, m.start()..m.end(), PiiCategory::Phone, 0.70))
        .collect()
}

// --- Credit Card with Luhn -------------------------------------------------

static CC_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d[ \-]*?){13,19}\b").unwrap()
});

fn scan_credit_card(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    CC_RE.find_iter(text)
        .filter(|m| {
            let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
            (13..=19).contains(&digits.len()) && luhn_ok(&digits)
        })
        .map(|m| candidate_from_match(text, words, m.start()..m.end(), PiiCategory::CreditCard, 0.95))
        .collect()
}

fn luhn_ok(digits: &str) -> bool {
    let mut sum = 0u32;
    let n = digits.len();
    for (i, c) in digits.chars().rev().enumerate() {
        let d = match c.to_digit(10) { Some(d) => d, None => return false };
        if i % 2 == 1 {
            let doubled = d * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += d;
        }
    }
    sum % 10 == 0
}

// --- Shared helpers --------------------------------------------------------

pub(crate) fn candidate_from_match(
    text: &str,
    words: &[OcrWord],
    char_range: std::ops::Range<usize>,
    category: PiiCategory,
    pattern_conf: f64,
) -> PiiCandidate {
    let matched_text = text[char_range.clone()].to_string();
    let (bbox, ocr_conf) = bbox_for_range(text, words, char_range);
    let confidence = ocr_conf.min(pattern_conf);
    PiiCandidate { category, matched_text, bbox, confidence }
}

/// Walk `words` to find which words overlap `char_range` in `text`, then
/// return their union bbox + average word confidence.
fn bbox_for_range(
    text: &str,
    words: &[OcrWord],
    char_range: std::ops::Range<usize>,
) -> (BBox, f64) {
    // Map char positions to word offsets. Words are joined back together as the
    // OCR text was assembled (newlines between lines, spaces between words on
    // the same line). For v1 we approximate by accumulating each word's text
    // length + 1 (for the separator).
    let _ = text;  // text not strictly needed once we have words list
    let mut acc = 0usize;
    let mut overlapping: Vec<&OcrWord> = Vec::new();
    for w in words {
        let start = acc;
        let end = acc + w.text.len();
        if end >= char_range.start && start <= char_range.end {
            overlapping.push(w);
        }
        acc = end + 1; // +1 for whitespace/newline separator
    }
    if overlapping.is_empty() {
        return (BBox { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, 0.0);
    }
    let min_x = overlapping.iter().map(|w| w.bbox.x).fold(f32::INFINITY, f32::min);
    let min_y = overlapping.iter().map(|w| w.bbox.y).fold(f32::INFINITY, f32::min);
    let max_x = overlapping.iter().map(|w| w.bbox.x + w.bbox.w).fold(f32::NEG_INFINITY, f32::max);
    let max_y = overlapping.iter().map(|w| w.bbox.y + w.bbox.h).fold(f32::NEG_INFINITY, f32::max);
    let avg_conf: f64 = overlapping.iter().map(|w| w.confidence).sum::<f64>() / overlapping.len() as f64;
    (BBox { x: min_x, y: min_y, w: max_x - min_x, h: max_y - min_y }, avg_conf)
}
```

**Step 4: Run — confirm pass**

```bash
cargo test -p snk-pii scanner::tests
```

Expected: all 5 tests pass.

**Step 5: Commit**

```bash
git add crates/snk-pii/src/scanner.rs
git diff --cached
git commit -m "feat(pii): scanner — email, phone, credit_card with Luhn"
```

---

### Task 22: PII scanner — SSN, IP, API keys

**Files:**
- Modify: `crates/snk-pii/src/scanner.rs`

**Step 1: Add failing tests**

Append to existing `mod tests`:

```rust
#[test]
fn detects_us_ssn_valid_range() {
    let text = "SSN: 123-45-6789 on file";
    let words = vec![w("SSN:", 0.0, 0), w("123-45-6789", 0.1, 0)];
    let cands = scan(text, &words);
    assert!(cands.iter().any(|c| c.category == PiiCategory::Ssn));
}

#[test]
fn rejects_ssn_in_invalid_range() {
    // 000-12-3456, 666-12-3456, 900-12-3456 all reserved/invalid
    for s in &["000-12-3456", "666-12-3456", "900-12-3456"] {
        let words = vec![w(s, 0.1, 0)];
        let cands = scan(s, &words);
        assert!(cands.iter().all(|c| c.category != PiiCategory::Ssn),
            "{s} should be rejected");
    }
}

#[test]
fn detects_ipv4() {
    let text = "Server at 192.168.1.42 is up";
    let words = vec![w("Server", 0.0, 0), w("at", 0.1, 0), w("192.168.1.42", 0.15, 0), w("is", 0.3, 0)];
    let cands = scan(text, &words);
    assert!(cands.iter().any(|c| c.category == PiiCategory::Ip));
}

#[test]
fn rejects_dotted_quad_with_bad_octet() {
    let text = "Version 12.345.6.7 is old";
    let words = vec![w("Version", 0.0, 0), w("12.345.6.7", 0.1, 0)];
    let cands = scan(text, &words);
    assert!(cands.iter().all(|c| c.category != PiiCategory::Ip),
        "octets > 255 must be rejected");
}

#[test]
fn detects_stripe_secret_key() {
    let text = "key=sk_test_DOCUMENTATION_PLACEHOLDER_00000000 more";
    let words = vec![w("key=sk_test_DOCUMENTATION_PLACEHOLDER_00000000", 0.0, 0)];
    let cands = scan(text, &words);
    assert!(cands.iter().any(|c| c.category == PiiCategory::ApiKey));
}

#[test]
fn detects_aws_access_key_id() {
    let text = "AWS: AKIAIOSFODNN7EXAMPLE";
    let words = vec![w("AWS:", 0.0, 0), w("AKIAIOSFODNN7EXAMPLE", 0.1, 0)];
    let cands = scan(text, &words);
    assert!(cands.iter().any(|c| c.category == PiiCategory::ApiKey));
}

#[test]
fn detects_github_pat() {
    let text = "token=ghp_abcdef1234567890abcdef1234567890abcd ok";
    let words = vec![w("token=ghp_abcdef1234567890abcdef1234567890abcd", 0.0, 0)];
    let cands = scan(text, &words);
    assert!(cands.iter().any(|c| c.category == PiiCategory::ApiKey));
}
```

**Step 2: Run — fail**

```bash
cargo test -p snk-pii scanner::tests
```

Expected: FAIL for the new tests (SSN/IP/API key not yet wired).

**Step 3: Add SSN/IP/API key scanners**

In `scanner.rs`, extend `scan` and add new functions:

```rust
pub fn scan(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    let mut out = Vec::new();
    out.extend(scan_email(text, words));
    out.extend(scan_phone(text, words));
    out.extend(scan_credit_card(text, words));
    out.extend(scan_ssn(text, words));
    out.extend(scan_ip(text, words));
    out.extend(scan_api_key(text, words));
    out.retain(|c| c.confidence >= 0.5);
    out
}

// --- SSN -------------------------------------------------------------------

static SSN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(\d{3})-(\d{2})-(\d{4})\b").unwrap());

fn scan_ssn(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    SSN_RE.captures_iter(text)
        .filter_map(|caps| {
            let m = caps.get(0).unwrap();
            let area = caps.get(1).unwrap().as_str();
            let valid = area != "000" && area != "666" && !area.starts_with('9');
            if !valid { return None; }
            Some(candidate_from_match(text, words, m.start()..m.end(), PiiCategory::Ssn, 0.80))
        })
        .collect()
}

// --- IP --------------------------------------------------------------------

static IPV4_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap()
});
static IPV6_RE: Lazy<Regex> = Lazy::new(|| {
    // Loose IPv6 — full RFC 4291 is huge; this catches the common forms.
    Regex::new(r"\b(?:[0-9A-Fa-f]{1,4}:){2,7}[0-9A-Fa-f]{1,4}\b").unwrap()
});

fn scan_ip(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    let mut out: Vec<PiiCandidate> = Vec::new();
    for caps in IPV4_RE.captures_iter(text) {
        let m = caps.get(0).unwrap();
        let ok = (1..=4).all(|i| {
            caps.get(i).unwrap().as_str().parse::<u16>().map(|n| n <= 255).unwrap_or(false)
        });
        if ok {
            out.push(candidate_from_match(text, words, m.start()..m.end(), PiiCategory::Ip, 0.60));
        }
    }
    for m in IPV6_RE.find_iter(text) {
        // Reject things that don't have at least one colon group (over-eager regex protection).
        if m.as_str().matches(':').count() >= 2 {
            out.push(candidate_from_match(text, words, m.start()..m.end(), PiiCategory::Ip, 0.55));
        }
    }
    out
}

// --- API keys --------------------------------------------------------------

static API_KEY_PATTERNS: Lazy<Vec<(Regex, f64)>> = Lazy::new(|| vec![
    (Regex::new(r"\bsk_(?:test|live)_[A-Za-z0-9]{24,}\b").unwrap(), 0.95),                  // Stripe
    (Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(), 0.95),                                   // AWS access key
    (Regex::new(r"\bghp_[A-Za-z0-9]{36}\b").unwrap(), 0.95),                                // GitHub PAT
    (Regex::new(r"\bxox[bpars]-[A-Za-z0-9-]{10,}\b").unwrap(), 0.90),                       // Slack
    (Regex::new(r"\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b").unwrap(), 0.85),  // JWT
]);

fn scan_api_key(text: &str, words: &[OcrWord]) -> Vec<PiiCandidate> {
    let mut out = Vec::new();
    for (re, conf) in API_KEY_PATTERNS.iter() {
        for m in re.find_iter(text) {
            out.push(candidate_from_match(text, words, m.start()..m.end(), PiiCategory::ApiKey, *conf));
        }
    }
    out
}
```

**Step 4: Run — pass**

```bash
cargo test -p snk-pii scanner
```

Expected: all 12 tests pass.

**Step 5: Commit**

```bash
git add crates/snk-pii/src/scanner.rs
git diff --cached
git commit -m "feat(pii): scanner — SSN (valid ranges), IPv4/IPv6, API key patterns"
```

---

### Task 23: PII worker — subscribe `ocr:ready`, scan, persist, emit `pii:scanned`

**Files:**
- Modify: `crates/snk-pii/src/worker.rs`
- Modify: `crates/snk-pii/src/plugin.rs`

**Step 1: Implement worker**

`crates/snk-pii/src/worker.rs`:

```rust
use std::sync::Arc;

use snk_library::ocr::OcrText;
use snk_library::pii::{insert, NewPiiSpan};
use snk_library::Db;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::scanner::{scan, PiiCandidate};

pub struct PiiWorker {
    tx: mpsc::UnboundedSender<String>,
}

impl PiiWorker {
    pub fn start(db: Arc<Db>, emit_scanned: Arc<dyn Fn(&str, usize) + Send + Sync>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(run(rx, db, emit_scanned));
        Self { tx }
    }

    pub fn enqueue(&self, capture_id: String) {
        if self.tx.send(capture_id).is_err() {
            error!("pii worker queue closed");
        }
    }
}

async fn run(
    mut rx: mpsc::UnboundedReceiver<String>,
    db: Arc<Db>,
    emit_scanned: Arc<dyn Fn(&str, usize) + Send + Sync>,
) {
    info!("pii worker started");
    while let Some(cap_id) = rx.recv().await {
        let db_clone = Arc::clone(&db);
        let cid = cap_id.clone();
        let result = tokio::task::spawn_blocking(move || scan_one(&db_clone, &cid)).await;
        match result {
            Ok(Ok(count)) => {
                emit_scanned(&cap_id, count);
            }
            Ok(Err(e)) => error!(capture_id = %cap_id, error = %e, "pii scan failed"),
            Err(e) => error!(capture_id = %cap_id, error = %e, "pii task panicked"),
        }
    }
    info!("pii worker stopped");
}

fn scan_one(db: &Db, capture_id: &str) -> Result<usize, String> {
    let row: OcrText = match snk_library::ocr::get(db, capture_id).map_err(|e| e.to_string())? {
        Some(r) => r,
        None => return Ok(0),  // no OCR yet — nothing to scan
    };
    let words = row.words.unwrap_or_default();
    if words.is_empty() {
        return Ok(0);
    }
    // De-dupe against existing pii_spans for this capture.
    let existing = snk_library::pii::list_for_capture(db, capture_id).map_err(|e| e.to_string())?;
    let cands: Vec<PiiCandidate> = scan(&row.text, &words);

    let mut new_count = 0usize;
    for c in cands {
        let dup = existing.iter().any(|e|
            e.category == c.category
            && e.matched_text == c.matched_text
            && (e.bbox_x - c.bbox.x).abs() < 0.01
            && (e.bbox_y - c.bbox.y).abs() < 0.01
        );
        if dup { continue; }
        insert(db, NewPiiSpan {
            capture_id,
            category: c.category,
            matched_text: &c.matched_text,
            bbox_x: c.bbox.x,
            bbox_y: c.bbox.y,
            bbox_w: c.bbox.w,
            bbox_h: c.bbox.h,
            confidence: c.confidence,
        }).map_err(|e| e.to_string())?;
        new_count += 1;
    }
    Ok(new_count)
}
```

**Step 2: Wire worker into plugin**

Replace `crates/snk-pii/src/plugin.rs`:

```rust
use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};

use snk_library::LibraryState;

use crate::worker::PiiWorker;

pub struct PiiState {
    pub worker: PiiWorker,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-pii")
        // commands wired in T24
        .setup(|app, _api| {
            let lib_state = app.state::<LibraryState>();
            let db = lib_state.db.clone();

            let app_for_emit = app.app_handle().clone();
            let emit_scanned: Arc<dyn Fn(&str, usize) + Send + Sync> = Arc::new(move |cap_id, n| {
                if let Err(e) = app_for_emit.emit("pii:scanned", serde_json::json!({
                    "capture_id": cap_id,
                    "pending_count": n,
                })) {
                    tracing::warn!(error = %e, "failed to emit pii:scanned");
                }
            });

            let worker = PiiWorker::start(Arc::clone(&db), emit_scanned);
            app.manage(PiiState { worker });

            // Subscribe to ocr:ready.
            let app_handle = app.app_handle().clone();
            app_handle.clone().listen("ocr:ready", move |event| {
                let cap_id = event.payload().trim_matches('"').to_string();
                if cap_id.is_empty() { return; }
                if let Some(pii) = app_handle.try_state::<PiiState>() {
                    pii.worker.enqueue(cap_id);
                }
            });

            Ok(())
        })
        .build()
}
```

**Step 3: Build**

```bash
cargo build -p snk-pii
```

Expected: clean.

**Step 4: Commit**

```bash
git add crates/snk-pii/src/worker.rs crates/snk-pii/src/plugin.rs
git diff --cached
git commit -m "feat(pii): worker subscribes to ocr:ready, scans, persists, emits pii:scanned"
```

---

### Task 24: PII commands + TS bindings

**Files:**
- Modify: `crates/snk-pii/src/plugin.rs` (add commands)
- Create: `packages/snk-pii/package.json`
- Create: `packages/snk-pii/src/index.ts`
- Create: `packages/snk-pii/tsconfig.json` (mirror sibling packages)

**Step 1: Add Rust commands**

Append to `crates/snk-pii/src/plugin.rs`:

```rust
use snk_library::pii::{PiiCategory, PiiSpan};

#[tauri::command]
pub fn list_pii_spans<R: Runtime>(
    app: tauri::AppHandle<R>, capture_id: String,
) -> Result<Vec<PiiSpan>, String> {
    let lib = app.state::<LibraryState>();
    snk_library::pii::list_for_capture(&lib.db, &capture_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_pending_pii<R: Runtime>(
    app: tauri::AppHandle<R>, capture_id: String,
) -> Result<Vec<PiiSpan>, String> {
    let lib = app.state::<LibraryState>();
    snk_library::pii::list_pending_for_capture(&lib.db, &capture_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dismiss_pii<R: Runtime>(
    app: tauri::AppHandle<R>, span_id: i64,
) -> Result<PiiSpan, String> {
    let lib = app.state::<LibraryState>();
    snk_library::pii::mark_dismissed(&lib.db, span_id).map_err(|e| e.to_string())?;
    snk_library::pii::get(&lib.db, span_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "span not found after dismiss".into())
}

// redact_pii lands in T25 once the blur primitive is shared from snk-library.
```

Then update the `Builder::new("snk-pii")` chain to register them:

```rust
Builder::<R>::new("snk-pii")
    .invoke_handler(tauri::generate_handler![
        list_pii_spans, list_pending_pii, dismiss_pii,
    ])
    // ... setup ...
```

**Step 2: Create TS package**

`packages/snk-pii/package.json` (mirror `packages/snk-ocr/package.json` for structure):

```json
{
  "name": "@snk/pii",
  "version": "0.0.1",
  "type": "module",
  "main": "./src/index.ts",
  "exports": {
    ".": "./src/index.ts"
  },
  "scripts": {
    "build": "tsc -p tsconfig.json --noEmit"
  },
  "devDependencies": {
    "typescript": "catalog:"
  }
}
```

`packages/snk-pii/tsconfig.json`: copy from `packages/snk-ocr/tsconfig.json` unchanged.

`packages/snk-pii/src/index.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type PiiCategory = 'email' | 'phone' | 'credit_card' | 'ssn' | 'ip' | 'api_key';

export interface PiiSpan {
  id: number;
  capture_id: string;
  category: PiiCategory;
  matched_text: string;
  bbox_x: number;
  bbox_y: number;
  bbox_w: number;
  bbox_h: number;
  confidence: number;
  redacted_at: number | null;
  dismissed_at: number | null;
  created_at: number;
}

export interface PiiScannedEvent {
  capture_id: string;
  pending_count: number;
}

export async function listPiiSpans(captureId: string): Promise<PiiSpan[]> {
  return invoke<PiiSpan[]>('plugin:snk-pii|list_pii_spans', { captureId });
}

export async function listPendingPii(captureId: string): Promise<PiiSpan[]> {
  return invoke<PiiSpan[]>('plugin:snk-pii|list_pending_pii', { captureId });
}

export async function dismissPii(spanId: number): Promise<PiiSpan> {
  return invoke<PiiSpan>('plugin:snk-pii|dismiss_pii', { spanId });
}

export async function redactPii(spanId: number): Promise<PiiSpan> {
  return invoke<PiiSpan>('plugin:snk-pii|redact_pii', { spanId });
}

export function onPiiScanned(handler: (e: PiiScannedEvent) => void): Promise<UnlistenFn> {
  return listen<PiiScannedEvent>('pii:scanned', (e) => handler(e.payload));
}
```

**Step 3: Add `@snk/pii` to `app/package.json` deps**

```bash
pnpm -F app add @snk/pii@workspace:*
```

**Step 4: Register `snk-pii` plugin in the app**

Find where other plugins are loaded in `app/src-tauri/src/lib.rs` (or `main.rs`). Add:

```rust
.plugin(snk_pii::init())
```

Add `snk-pii` as a path dep in `app/src-tauri/Cargo.toml`:

```toml
snk-pii = { path = "../../crates/snk-pii" }
```

**Step 5: Add capability for `snk-pii` commands**

In `app/src-tauri/capabilities/default.json`, add:

```json
{
  "identifier": "snk-pii:default",
  "permissions": ["list_pii_spans", "list_pending_pii", "dismiss_pii", "redact_pii"]
}
```

(Pattern matches existing plugin entries.)

**Step 6: Build full app**

```bash
cargo build -p snk-pii
pnpm tauri build --debug
```

Expected: clean.

**Step 7: Commit**

```bash
git add crates/snk-pii/src/plugin.rs packages/snk-pii/ app/src-tauri/Cargo.toml app/src-tauri/src/*.rs app/src-tauri/capabilities/default.json app/package.json pnpm-lock.yaml
git diff --cached
git commit -m "feat(pii): commands + TS bindings + app plugin registration"
```

---

### Task 25: Shared pixel-blur primitive + `redact_pii` command

**Files:**
- Identify current blur primitive location and refactor if needed.

**Step 1: Locate existing blur logic**

```bash
grep -rn 'blur\|gaussian\|pixelat' crates/ packages/ app/src/windows/editor/ | head -40
```

Phase 3 added the blur tool. Likely lives in `app/src/windows/editor/` (frontend canvas operation) or in `snk-library` / `snk-capture` as a Rust image function.

**Step 2: If blur is frontend-only, add a Rust primitive in `snk-library`**

Create `crates/snk-library/src/image_ops.rs`:

```rust
use std::path::Path;

use image::{GenericImageView, ImageBuffer, Rgba};

use crate::{LibraryError, Result};

/// Apply a Gaussian blur to a normalized 0..1 bbox region of the image at `path`.
/// Overwrites the file in place. Destructive — call site must have already
/// confirmed with the user.
pub fn blur_region_in_place(path: &Path, bbox: BBoxN, sigma: f32) -> Result<()> {
    let img = image::open(path).map_err(|e| LibraryError::ImageOps {
        detail: format!("open {}: {e}", path.display()),
    })?;
    let (w, h) = img.dimensions();
    let x = ((bbox.x * w as f32) as u32).min(w.saturating_sub(1));
    let y = ((bbox.y * h as f32) as u32).min(h.saturating_sub(1));
    let bw = ((bbox.w * w as f32) as u32).min(w - x);
    let bh = ((bbox.h * h as f32) as u32).min(h - y);
    if bw == 0 || bh == 0 {
        return Ok(());  // empty bbox — no-op
    }
    let sub = img.view(x, y, bw, bh).to_image();
    let blurred = image::imageops::blur(&sub, sigma);
    let mut canvas: ImageBuffer<Rgba<u8>, _> = img.to_rgba8();
    image::imageops::overlay(&mut canvas, &blurred, x as i64, y as i64);
    canvas.save(path).map_err(|e| LibraryError::ImageOps {
        detail: format!("save {}: {e}", path.display()),
    })?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct BBoxN { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
```

Add `pub mod image_ops;` to `crates/snk-library/src/lib.rs`. Add `ImageOps { detail: String }` to `LibraryError` if missing.

Add `image = { workspace = true }` to `[dependencies]` of `crates/snk-library/Cargo.toml` if not already there (it is, used by tests — promote to runtime).

**Step 3: Add `redact_pii` Rust command**

Append to `crates/snk-pii/src/plugin.rs`:

```rust
use snk_library::image_ops::{blur_region_in_place, BBoxN};

#[tauri::command]
pub fn redact_pii<R: Runtime>(
    app: tauri::AppHandle<R>, span_id: i64,
) -> Result<PiiSpan, String> {
    let lib = app.state::<LibraryState>();
    let span = snk_library::pii::get(&lib.db, span_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "span not found".to_string())?;
    let capture = snk_library::captures::get(&lib.db, &span.capture_id).map_err(|e| e.to_string())?;
    let full_path = lib.root.join(&capture.file_path);
    blur_region_in_place(
        &full_path,
        BBoxN { x: span.bbox_x, y: span.bbox_y, w: span.bbox_w, h: span.bbox_h },
        12.0,  // sigma; chosen to fully obscure typical OCR-detected text
    ).map_err(|e| e.to_string())?;
    snk_library::pii::mark_redacted(&lib.db, span_id).map_err(|e| e.to_string())?;
    snk_library::pii::get(&lib.db, span_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "span not found after redact".into())
}
```

Update `generate_handler!` macro list to include `redact_pii`.

**Step 4: Build**

```bash
cargo build --workspace
```

Expected: clean.

**Step 5: Commit**

```bash
git add crates/snk-library/src/image_ops.rs crates/snk-library/src/lib.rs crates/snk-library/Cargo.toml crates/snk-pii/src/plugin.rs
git diff --cached
git commit -m "feat(pii): redact_pii command + shared blur_region_in_place primitive"
```

---

### Task 26: PII integration test (synthetic OCR → scan → DB)

**Files:**
- Create: `crates/snk-pii/tests/integration_test.rs`

**Step 1: Write end-to-end test**

```rust
use snk_library::captures::{insert as insert_capture, NewCapture};
use snk_library::ocr::{upsert_full, BBox, OcrWord};
use snk_library::pii::list_pending_for_capture;
use std::path::PathBuf;

fn db() -> (tempfile::TempDir, std::sync::Arc<snk_library::Db>) {
    let tmp = tempfile::tempdir().unwrap();
    let db = snk_library::Db::open(&tmp.path().join("test.db")).unwrap();
    snk_library::migrate::migrate(&mut db.with_conn(|c| Ok(c.clone())).unwrap()).unwrap();
    // ^ may need adjustment depending on actual Db API; alternative is to use the
    //   test_support::fresh_db helper if exported.
    (tmp, std::sync::Arc::new(db))
}

fn make_words_from(text: &str) -> Vec<OcrWord> {
    text.split_whitespace().enumerate().map(|(i, w)| OcrWord {
        text: w.into(),
        bbox: BBox { x: 0.05 + i as f32 * 0.1, y: 0.1, w: 0.08, h: 0.04 },
        confidence: 0.95,
        line: 0,
    }).collect()
}

#[test]
fn end_to_end_email_detection_writes_span() {
    let (_t, db) = db();
    let cap = insert_capture(&db, NewCapture {
        file_path: PathBuf::from("test.png"),
        width: 1920, height: 1080,
        source_app: None, source_window_title: None, monitor: None,
    }).unwrap();
    let text = "Contact alice@example.com today";
    let words = make_words_from(text);
    upsert_full(&db, &cap.id, text, "eng", 0.95, &words, "test-engine").unwrap();

    // Run scanner directly (no event dispatch needed for the test).
    let cands = snk_pii::scanner::scan(text, &words);
    for c in cands {
        snk_library::pii::insert(&db, snk_library::pii::NewPiiSpan {
            capture_id: &cap.id,
            category: c.category,
            matched_text: &c.matched_text,
            bbox_x: c.bbox.x, bbox_y: c.bbox.y, bbox_w: c.bbox.w, bbox_h: c.bbox.h,
            confidence: c.confidence,
        }).unwrap();
    }

    let pending = list_pending_for_capture(&db, &cap.id).unwrap();
    assert!(pending.iter().any(|s| s.category == snk_library::pii::PiiCategory::Email));
}
```

**Step 2: Run**

```bash
cargo test -p snk-pii --test integration_test
```

Expected: pass. If the `Db` constructor signature differs, swap for the actual exported helper from `snk-library` (`test_support::fresh_db` is used in unit tests but may not be public; if not, use `Db::open(path)` and call `snk_library::migrate::migrate(...)` directly).

**Step 3: Commit**

```bash
git add crates/snk-pii/tests/integration_test.rs
git diff --cached
git commit -m "test(pii): end-to-end email detection writes pii_spans row"
```

---

### Task 27: PR-3 open

**Step 1: Push + open PR**

```bash
git push -u origin feat/phase10-pr3-pii-plugin

gh pr create \
  --base feat/phase10-ocr-surfaces \
  --title "feat(phase10/PR3): snk-pii plugin — scanner, worker, commands" \
  --body "$(cat <<'EOF'
## Summary

- New `snk-pii` plugin: regex-based scanner for 6 PII categories (email, phone, credit_card with Luhn, SSN with valid-range filter, IPv4/IPv6, API key patterns).
- Async worker subscribes to `ocr:ready`, scans, persists pending `pii_spans`, emits `pii:scanned`.
- Plugin commands: `list_pii_spans`, `list_pending_pii`, `redact_pii`, `dismiss_pii`.
- Shared `blur_region_in_place` primitive in `snk-library` for destructive redaction.
- TS bindings package `@snk/pii`.

## Test plan

- [ ] `cargo test -p snk-pii` passes (12 scanner unit tests + integration test)
- [ ] `cargo test --workspace` still passes
- [ ] Manual: capture a screenshot with an email visible → confirm `pii:scanned` event fires with pending_count >= 1 and `pii_spans` row exists
- [ ] Manual: call `dismiss_pii(id)` → row's `dismissed_at` is set, `list_pending_pii` no longer returns it
- [ ] Manual: call `redact_pii(id)` → image file modified (bbox region blurred), row's `redacted_at` is set

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Wait for CI; merge with squash.

---

## PR-4: UI surfaces

**Branch:** `feat/phase10-pr4-ui-surfaces` off updated `feat/phase10-ocr-surfaces`.

Goal: build the three shared React primitives (`<TextOverlay>`, `<PiiBadge>`, `<PiiReviewSheet>`), wire them into post-capture toolbar / annotation editor / library viewer, add OCR rows to the existing AboutSection, and close the obsoleted issues.

### Task 28: `<TextOverlay>` component

**Files:**
- Create: `app/src/components/TextOverlay.tsx`
- Create: `app/src/components/TextOverlay.css`
- Create: `app/src/components/TextOverlay.test.tsx`

**Step 1: Write failing test**

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TextOverlay } from './TextOverlay';
import type { OcrWord } from '@snk/ocr';

const fixtureWords: OcrWord[] = [
  { text: 'Hello', bbox: { x: 0.10, y: 0.05, w: 0.08, h: 0.04 }, confidence: 0.97, line: 0 },
  { text: 'world', bbox: { x: 0.19, y: 0.05, w: 0.08, h: 0.04 }, confidence: 0.95, line: 0 },
];

describe('<TextOverlay>', () => {
  it('renders one word region per word in selectionMode=hover', () => {
    render(<TextOverlay imageUrl="x.png" words={fixtureWords} selectionMode="hover" />);
    const regions = screen.getAllByRole('text');
    expect(regions).toHaveLength(2);
    expect(regions[0]).toHaveTextContent('Hello');
    expect(regions[1]).toHaveTextContent('world');
  });

  it('renders zero regions when words is empty', () => {
    render(<TextOverlay imageUrl="x.png" words={[]} selectionMode="hover" />);
    expect(screen.queryAllByRole('text')).toHaveLength(0);
  });

  it('does not render selectable regions when selectionMode=off', () => {
    render(<TextOverlay imageUrl="x.png" words={fixtureWords} selectionMode="off" />);
    // Off mode still renders divs for layout but they are aria-hidden.
    const regions = screen.queryAllByRole('text');
    expect(regions).toHaveLength(0);
  });
});
```

**Step 2: Run — fail**

```bash
pnpm -F app test TextOverlay
```

Expected: FAIL — `TextOverlay` not defined.

**Step 3: Implement component**

`app/src/components/TextOverlay.tsx`:

```tsx
import { useEffect, useRef, useState } from 'react';
import type { OcrWord } from '@snk/ocr';
import './TextOverlay.css';

export type TextOverlaySelectionMode = 'hover' | 'off';

export interface TextOverlayProps {
  imageUrl: string;
  words: OcrWord[];
  selectionMode: TextOverlaySelectionMode;
}

export function TextOverlay({ imageUrl, words, selectionMode }: TextOverlayProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<{ w: number; h: number } | null>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect;
      if (r) setSize({ w: r.width, h: r.height });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  return (
    <div ref={containerRef} className="text-overlay-container">
      <img src={imageUrl} alt="" className="text-overlay-image" draggable={false} />
      {size && selectionMode === 'hover' && words.map((w, i) => {
        const style = {
          position: 'absolute' as const,
          left: `${w.bbox.x * 100}%`,
          top: `${w.bbox.y * 100}%`,
          width: `${w.bbox.w * 100}%`,
          height: `${w.bbox.h * 100}%`,
        };
        return (
          <div
            key={i}
            role="text"
            className="text-overlay-word"
            style={style}
          >{w.text}</div>
        );
      })}
    </div>
  );
}
```

`app/src/components/TextOverlay.css`:

```css
.text-overlay-container {
  position: relative;
  display: inline-block;
  user-select: text;
}

.text-overlay-image {
  display: block;
  max-width: 100%;
  height: auto;
}

.text-overlay-word {
  cursor: text;
  color: transparent;
  font-size: 1px; /* text exists for selection, invisible for layout */
  line-height: 1;
  user-select: text;
  pointer-events: auto;
}

.text-overlay-word::selection {
  background-color: rgba(70, 130, 200, 0.35);
}
```

**Step 4: Run — pass**

```bash
pnpm -F app test TextOverlay
```

Expected: 3 tests pass.

**Step 5: Commit**

```bash
git add app/src/components/TextOverlay.tsx app/src/components/TextOverlay.css app/src/components/TextOverlay.test.tsx
git diff --cached
git commit -m "feat(ui): TextOverlay component — selectable per-word regions"
```

---

### Task 29: `<PiiBadge>` component

**Files:**
- Create: `app/src/components/PiiBadge.tsx`
- Create: `app/src/components/PiiBadge.test.tsx`

**Step 1: Failing test**

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PiiBadge } from './PiiBadge';

describe('<PiiBadge>', () => {
  it('renders nothing when count is 0', () => {
    const { container } = render(<PiiBadge count={0} onClick={() => {}} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders count when > 0', () => {
    render(<PiiBadge count={3} onClick={() => {}} />);
    expect(screen.getByText(/3 items detected/i)).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const onClick = vi.fn();
    render(<PiiBadge count={1} onClick={onClick} />);
    fireEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
```

**Step 2: Implement**

`app/src/components/PiiBadge.tsx`:

```tsx
export interface PiiBadgeProps {
  count: number;
  onClick: () => void;
}

export function PiiBadge({ count, onClick }: PiiBadgeProps) {
  if (count === 0) return null;
  return (
    <button
      type="button"
      className="pii-badge"
      onClick={onClick}
      title="Click to review detected PII"
    >
      <span aria-hidden="true">⚠</span>
      <span>{count} {count === 1 ? 'item' : 'items'} detected</span>
    </button>
  );
}
```

Add CSS to your existing design-tokens stylesheet (or inline; depends on project convention):

```css
.pii-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.4em;
  padding: 0.2em 0.6em;
  border-radius: 999px;
  background: var(--color-warning-bg, #fff3cd);
  color: var(--color-warning-fg, #856404);
  border: 1px solid var(--color-warning-border, #ffeeba);
  font-size: 0.85em;
  cursor: pointer;
}
.pii-badge:hover { filter: brightness(0.97); }
```

(If the project uses a centralized token file, add it there; otherwise inline in a sibling `.css`.)

**Step 3: Run + commit**

```bash
pnpm -F app test PiiBadge
git add app/src/components/PiiBadge.tsx app/src/components/PiiBadge.test.tsx
git diff --cached
git commit -m "feat(ui): PiiBadge component"
```

---

### Task 30: `<PiiReviewSheet>` component

**Files:**
- Create: `app/src/components/PiiReviewSheet.tsx`
- Create: `app/src/components/PiiReviewSheet.test.tsx`

**Step 1: Failing test**

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PiiReviewSheet } from './PiiReviewSheet';
import type { PiiSpan } from '@snk/pii';

const spans: PiiSpan[] = [
  {
    id: 1, capture_id: 'c1', category: 'email',
    matched_text: 'alice@example.com',
    bbox_x: 0.1, bbox_y: 0.1, bbox_w: 0.2, bbox_h: 0.04,
    confidence: 0.95, redacted_at: null, dismissed_at: null, created_at: 0,
  },
];

describe('<PiiReviewSheet>', () => {
  it('lists each pending span', () => {
    render(<PiiReviewSheet spans={spans} onRedact={() => {}} onDismiss={() => {}} />);
    expect(screen.getByText(/alice@example\.com/)).toBeInTheDocument();
  });

  it('calls onRedact with span id', () => {
    const onRedact = vi.fn();
    render(<PiiReviewSheet spans={spans} onRedact={onRedact} onDismiss={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: /redact/i }));
    expect(onRedact).toHaveBeenCalledWith(1);
  });

  it('calls onDismiss with span id', () => {
    const onDismiss = vi.fn();
    render(<PiiReviewSheet spans={spans} onRedact={() => {}} onDismiss={onDismiss} />);
    fireEvent.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(onDismiss).toHaveBeenCalledWith(1);
  });

  it('shows empty state when spans is empty', () => {
    render(<PiiReviewSheet spans={[]} onRedact={() => {}} onDismiss={() => {}} />);
    expect(screen.getByText(/no pending/i)).toBeInTheDocument();
  });
});
```

**Step 2: Implement**

`app/src/components/PiiReviewSheet.tsx`:

```tsx
import type { PiiSpan } from '@snk/pii';

export interface PiiReviewSheetProps {
  spans: PiiSpan[];
  onRedact: (spanId: number) => void;
  onDismiss: (spanId: number) => void;
}

function categoryLabel(category: PiiSpan['category']): string {
  switch (category) {
    case 'email': return 'Email';
    case 'phone': return 'Phone';
    case 'credit_card': return 'Credit Card';
    case 'ssn': return 'SSN';
    case 'ip': return 'IP Address';
    case 'api_key': return 'API Key';
  }
}

function mask(text: string, category: PiiSpan['category']): string {
  if (category === 'email') {
    const [u, d] = text.split('@', 2);
    return `${u?.slice(0, 3) ?? ''}••@${d ? '••••' + d.slice(-4) : ''}`;
  }
  if (text.length <= 6) return '••••••';
  return `${text.slice(0, 3)}••••${text.slice(-2)}`;
}

export function PiiReviewSheet({ spans, onRedact, onDismiss }: PiiReviewSheetProps) {
  if (spans.length === 0) {
    return <div className="pii-review-empty">No pending PII suggestions.</div>;
  }
  return (
    <div className="pii-review-sheet">
      {spans.map((s) => (
        <div key={s.id} className="pii-review-row">
          <span className="pii-review-category">{categoryLabel(s.category)}</span>
          <span className="pii-review-preview">{mask(s.matched_text, s.category)}</span>
          <button type="button" onClick={() => onRedact(s.id)}>Redact</button>
          <button type="button" onClick={() => onDismiss(s.id)}>Dismiss</button>
        </div>
      ))}
    </div>
  );
}
```

**Step 3: Run + commit**

```bash
pnpm -F app test PiiReviewSheet
git add app/src/components/PiiReviewSheet.tsx app/src/components/PiiReviewSheet.test.tsx
git diff --cached
git commit -m "feat(ui): PiiReviewSheet component — per-item redact/dismiss"
```

---

### Task 31: Wire post-capture toolbar (`Copy text` + `<PiiBadge>`)

**Files:**
- Modify: `app/src/windows/capture-toolbar/` (exact file depends on Phase 2 structure; find with `ls app/src/windows/capture-toolbar/`)

**Step 1: Locate toolbar entry point**

```bash
ls app/src/windows/capture-toolbar/
cat app/src/windows/capture-toolbar/CaptureToolbar.tsx  # or whatever the main file is
```

**Step 2: Add OCR-state hook + Copy text action + badge**

In the toolbar component, add:

```tsx
import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { getOcrWords, onOcrReady } from '@snk/ocr';
import { listPendingPii, dismissPii, redactPii, onPiiScanned, type PiiSpan } from '@snk/pii';
import { PiiBadge } from '../../components/PiiBadge';
import { PiiReviewSheet } from '../../components/PiiReviewSheet';
import { invoke } from '@tauri-apps/api/core';

// Inside the toolbar component, given `captureId`:
function useOcrText(captureId: string) {
  return useQuery({
    queryKey: ['ocr-text', captureId],
    queryFn: async () => {
      const row = await invoke<{ text: string } | null>('plugin:snk-library|get_ocr_text', { captureId });
      return row?.text ?? null;
    },
  });
}

function usePendingPii(captureId: string) {
  return useQuery({
    queryKey: ['pii-pending', captureId],
    queryFn: () => listPendingPii(captureId),
  });
}

// Component body:
export function CaptureToolbarOcrActions({ captureId }: { captureId: string }) {
  const qc = useQueryClient();
  const { data: ocrText, isLoading: ocrLoading } = useOcrText(captureId);
  const { data: pendingPii } = usePendingPii(captureId);
  const [sheetOpen, setSheetOpen] = useState(false);

  useEffect(() => {
    const unsubOcr = onOcrReady((id) => {
      if (id === captureId) qc.invalidateQueries({ queryKey: ['ocr-text', captureId] });
    });
    const unsubPii = onPiiScanned((e) => {
      if (e.capture_id === captureId) qc.invalidateQueries({ queryKey: ['pii-pending', captureId] });
    });
    return () => {
      void unsubOcr.then((u) => u());
      void unsubPii.then((u) => u());
    };
  }, [captureId, qc]);

  const copyText = async () => {
    if (ocrText) await writeText(ocrText);
  };

  return (
    <>
      <button type="button" onClick={copyText} disabled={!ocrText || ocrLoading}>
        {ocrLoading ? 'Analyzing…' : 'Copy text'}
      </button>
      <PiiBadge count={pendingPii?.length ?? 0} onClick={() => setSheetOpen(true)} />
      {sheetOpen && pendingPii && (
        <div role="dialog" className="capture-toolbar-pii-modal">
          <PiiReviewSheet
            spans={pendingPii}
            onRedact={async (id) => {
              await redactPii(id);
              qc.invalidateQueries({ queryKey: ['pii-pending', captureId] });
            }}
            onDismiss={async (id) => {
              await dismissPii(id);
              qc.invalidateQueries({ queryKey: ['pii-pending', captureId] });
            }}
          />
          <button type="button" onClick={() => setSheetOpen(false)}>Close</button>
        </div>
      )}
    </>
  );
}
```

Note: `plugin:snk-library|get_ocr_text` may not exist as a command yet. If so, expose it from `snk-library`'s plugin as a small read-only `get_ocr_text(capture_id)` command that returns the row's `text` field (and update TS bindings in `packages/snk-library`). Add this as a sub-task here:

**Sub-task: add `get_ocr_text` to `snk-library` if missing**

In `crates/snk-library/src/plugin.rs` (or wherever commands live), add:

```rust
#[tauri::command]
pub fn get_ocr_text<R: Runtime>(app: tauri::AppHandle<R>, capture_id: String) -> Result<Option<serde_json::Value>, String> {
    let state = app.state::<LibraryState>();
    let row = crate::ocr::get(&state.db, &capture_id).map_err(|e| e.to_string())?;
    Ok(row.map(|r| serde_json::to_value(&r).unwrap()))
}
```

Register in `generate_handler!`, add to capability, add TS binding. Skip if the project already has equivalent.

**Step 3: Integrate into existing toolbar layout**

Slot `<CaptureToolbarOcrActions captureId={...} />` into the toolbar's right-side button cluster.

**Step 4: Smoke-test in `pnpm tauri dev`**

Capture a screenshot containing an email address. Confirm:
- "Copy text" enables within ~500ms of capture.
- `⚠ 1 item detected` badge appears after the OCR.
- Click badge → review sheet shows the email span.
- Click "Redact" → image file is blurred at the bbox; badge count drops to 0.

**Step 5: Commit**

```bash
git add app/src/windows/capture-toolbar/ crates/snk-library/src/plugin.rs packages/snk-library/src/ app/src-tauri/capabilities/default.json
git diff --cached
git commit -m "feat(ui): post-capture toolbar — Copy text + PII badge + review sheet"
```

---

### Task 32: Wire annotation editor (overlay toggle + right-docked panel)

**Files:**
- Modify: `app/src/windows/editor/` (exact files per Phase 3 structure)

**Step 1: Locate editor**

```bash
ls app/src/windows/editor/
```

**Step 2: Add overlay toggle + integrate components**

- Add a state variable `overlayActive: boolean`, default `true`.
- Add a toolbar toggle button (text-cursor icon). Clicking toggles `overlayActive`.
- When user starts drawing (any annotation tool active and canvas mousedown), automatically set `overlayActive = false`.
- Layer `<TextOverlay imageUrl={captureUrl} words={words} selectionMode={overlayActive ? 'hover' : 'off'} />` between the image and the annotation canvas (z-index between the two).
- Add `<PiiBadge>` to the editor top bar. Click opens the review sheet as a **right-docked panel** (not a modal).

For the docked panel: if the editor already has a layout primitive for right-side panels, use it. If not, the simplest implementation:

```tsx
{piiPanelOpen && (
  <aside className="editor-pii-panel">
    <header>
      <span>Detected PII</span>
      <button onClick={() => setPiiPanelOpen(false)}>×</button>
    </header>
    <PiiReviewSheet spans={pendingPii ?? []} onRedact={...} onDismiss={...} />
  </aside>
)}
```

CSS makes `aside` a fixed-width column docked to the right edge of the editor.

**Editor redaction is non-destructive:** instead of calling `redactPii` (which is destructive), the editor adds a blur annotation to its edit stack at the span's bbox. Map: `onRedact={(id) => { addBlurAnnotation(span.bbox); markDismissed(id); }}` — the span is marked dismissed in DB (so the badge count drops) and the blur becomes part of the layered edit. On editor export, the layered blur bakes into the image.

If layered blur isn't accessible from the editor without reshape, fall back to calling `redactPii` (destructive) and document the discrepancy. The spec §6 surface 2 prefers non-destructive but allows fallback.

**Step 3: Smoke-test in `pnpm tauri dev`**

Open the editor on a capture with detected PII. Confirm:
- Overlay is on by default; text is selectable with cursor.
- Clicking the brush tool flips overlay off; drawing works.
- PII badge appears in top bar.
- Click badge → panel docks to the right.
- "Redact" adds a blur annotation (does NOT modify the file).

**Step 4: Commit**

```bash
git add app/src/windows/editor/
git diff --cached
git commit -m "feat(ui): editor — TextOverlay toggle + PII review docked panel"
```

---

### Task 33: Wire library viewer (tile badge dot + detail view overlay + badge)

**Files:**
- Modify: `app/src/windows/library/` (tile component + detail view)

**Step 1: Locate components**

```bash
ls app/src/windows/library/
```

**Step 2: Add badge dot to grid tile**

In the capture-tile component, query `listPendingPii(captureId)` (or fetch counts in bulk if Phase 6 has a batch endpoint). If count > 0, render a small `<span className="pii-tile-dot">` corner element with title showing the count.

```tsx
{pendingCount > 0 && (
  <span className="pii-tile-dot" title={`${pendingCount} PII items detected`} />
)}
```

CSS: small ⚠-colored dot positioned absolute, top-right of the tile.

**Step 3: Detail view — TextOverlay + Copy text + PII badge**

In the detail view (whatever opens when a tile is clicked), import the same `<CaptureToolbarOcrActions>` style hook chain from T31 and render alongside the existing detail toolbar. Render `<TextOverlay>` over the preview image with `selectionMode="hover"`.

PII review opens as a **modal** here (not docked panel — detail view typically has less horizontal real estate than the editor).

**Step 4: Smoke-test**

Open library after capturing several screenshots, some with PII. Confirm tiles with PII show the dot. Open a tile → text is selectable; Copy text works; badge opens review sheet.

**Step 5: Commit**

```bash
git add app/src/windows/library/
git diff --cached
git commit -m "feat(ui): library — tile PII dot + detail TextOverlay + Copy text + badge"
```

---

### Task 34: AboutSection OCR rows (engine + categories + last error)

**Files:**
- Modify: `app/src/windows/settings/AboutSection.tsx`

**Step 1: Add OCR rows**

```tsx
import { ocrStatus, type OcrStatus } from '@snk/ocr';
import { useQuery } from '@tanstack/react-query';

function useOcrStatus() {
  return useQuery({ queryKey: ['ocr-status'], queryFn: ocrStatus });
}

// In AboutSection JSX, append rows similar to existing ones:
function OcrRows() {
  const { data, isLoading } = useOcrStatus();
  if (isLoading || !data) return null;
  return (
    <>
      <SettingRow label="OCR engine" description={null}>
        <span>{data.backend} {data.version}</span>
      </SettingRow>
      <SettingRow label="PII categories" description={null}>
        <span>email, phone, credit card, SSN, IP, API keys</span>
      </SettingRow>
      {data.last_error && (
        <SettingRow label="Last OCR engine error" description={null}>
          <span style={{ color: 'var(--color-warning-fg)' }}>
            {(data.last_error as any).kind ?? 'error'}: {(data.last_error as any).reason ?? (data.last_error as any).detail ?? ''}
          </span>
        </SettingRow>
      )}
    </>
  );
}
```

Render `<OcrRows />` inside the existing About section.

**Step 2: Smoke-test**

Open Settings → About. Confirm engine row shows e.g. `Vision (macOS 15.x)` or `Windows.Media.Ocr (10.0.x)`, categories row lists the six, last error row only appears if OCR backend failed to construct.

**Step 3: Commit**

```bash
git add app/src/windows/settings/AboutSection.tsx
git diff --cached
git commit -m "feat(ui): About panel — OCR engine + PII categories + last error rows"
```

---

### Task 35: PR-4 open + parent integration + close issues

**Step 1: Push + open PR-4**

```bash
git push -u origin feat/phase10-pr4-ui-surfaces
gh pr create \
  --base feat/phase10-ocr-surfaces \
  --title "feat(phase10/PR4): UI surfaces — TextOverlay, PiiBadge, PiiReviewSheet wired everywhere" \
  --body "$(cat <<'EOF'
## Summary

- Three new shared React primitives: `<TextOverlay>` (selectable per-word regions), `<PiiBadge>` (⚠ N items detected), `<PiiReviewSheet>` (per-item Redact/Dismiss).
- Post-capture toolbar: Copy text action + PII badge → modal review sheet.
- Annotation editor: TextOverlay with toggle (default-on, off on draw) + PII badge → right-docked panel; editor redactions use layered blur (non-destructive).
- Library viewer: tile PII dot + detail TextOverlay + Copy text + PII modal.
- About panel: OCR engine + PII categories + last error rows.

## Test plan

- [ ] `pnpm -F app test` passes (new component tests + existing)
- [ ] `pnpm tauri build --debug` succeeds on Mac + Windows
- [ ] Manual smoke: capture an image with each of the 6 PII categories → all surface as detected
- [ ] Manual: editor redact adds blur annotation (non-destructive); toolbar/library redact modifies file (destructive)
- [ ] Manual: Copy text in toolbar copies OCR text to clipboard
- [ ] Manual: TextOverlay text selection works with Cmd/Ctrl+C in all three surfaces

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Wait for CI; merge with squash. `feat/phase10-ocr-surfaces` now has all four PRs.

**Step 2: Open parent integration PR against `main`**

```bash
git checkout feat/phase10-ocr-surfaces
git pull --ff-only origin feat/phase10-ocr-surfaces
git push origin feat/phase10-ocr-surfaces

gh pr create \
  --base main \
  --title "feat(phase10): OCR surfaces — Vision + WinOcr + PII redact + Text Actions + Tesseract cleanse" \
  --body "$(cat <<'EOF'
## Summary

Phase 10 ships native OCR (Apple Vision on macOS, Windows.Media.Ocr on Windows), PII auto-detect with per-item user confirmation, and an Apple-Live-Text-style selectable text overlay surfaced in the post-capture toolbar, annotation editor, and library viewer. **Tesseract has been completely removed** from the codebase (no sidecar, no bundled tessdata, no env override, no CI bundling, no README install).

## What's in

- PR-2 (foundation): `OcrBackend` trait + Vision + WinOcr backends + migration V006 + Tesseract cleanse
- PR-3 (PII plugin): `snk-pii` scanner + worker + commands + blur primitive share
- PR-4 (UI surfaces): TextOverlay + PiiBadge + PiiReviewSheet + all three surface wirings + About rows

## Cleanse verification

```
git grep -i tesseract -- ':!docs/superpowers/{specs,plans,research,reviews}/**'
```
Returns zero matches.

## Issue resolution

Closes #31 (adopted Vision; option 3 from the issue).
Closes #41 (no sidecar to sandbox).
Closes #40 (in-process OCR removes the bounded-queue motivation; startup-sweep behavior is no longer needed because the new pipeline doesn't have subprocess pressure that could drop OCR jobs).

## Bundle size

- macOS installer: −35 MB (tessdata + Tesseract binary removed)
- Windows installer: −50 MB (Tesseract distribution removed)

## Spec

[`docs/superpowers/specs/2026-05-25-phase10-ocr-surfaces-design.md`](docs/superpowers/specs/2026-05-25-phase10-ocr-surfaces-design.md)

## Plan

[`docs/superpowers/plans/2026-05-25-phase10-ocr-surfaces.md`](docs/superpowers/plans/2026-05-25-phase10-ocr-surfaces.md)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

**Step 3: After parent merges to main, close obsoleted issues**

```bash
gh issue close 31 --reason completed -c "Resolved by Phase 10 (adopted Vision per option 3). See merged commits on main."
gh issue close 41 --reason completed -c "Obsoleted by Phase 10 — no sidecar to sandbox."
gh issue close 40 --reason completed -c "Obsoleted by Phase 10 — in-process OCR removes subprocess pressure. Startup-sweep no longer needed; the new pipeline's typed errors surface backend unavailability immediately."
```

**Step 4: Update CLAUDE.md phase status (final pass)**

The phase status table entry added in T16 should now flip to "Done":

| 10 | OCR surfaces: Vision + WinOcr + PII redact + Text Actions; full bundled-engine removal | Done |

Commit this directly to `main` as a small docs follow-up, or fold into the parent PR before merge.

---

## Self-review against the spec

Cross-check each spec section against the plan:

| Spec section | Plan coverage | Notes |
|---|---|---|
| §1 Goals (1) replace Tesseract with native OCR | T7 (Vision), T8 (WinOcr), T9 (queue refactor) | ✓ |
| §1 Goals (2) per-word bboxes + confidence persisted | T2 (migration), T3 (words_json field), T9 (write path) | ✓ |
| §1 Goals (3) PII auto-detect 6 categories | T21, T22 | ✓ |
| §1 Goals (4) Text Actions selectable overlay in all 3 surfaces | T28 (component), T31, T32, T33 (wiring) | ✓ |
| §1 Goals (5) 100% Tesseract cleanse | T13–T18 + verification gate T18 | ✓ |
| §1 Non-goals (no re-OCR of legacy, no NER, no opt-in Tesseract, no cross-OS symmetry, no multi-language picker) | Plan respects — no tasks add these | ✓ |
| §2 ADR engine matrix | Implicit in T7 + T8 + Cargo.toml deps | ✓ |
| §3 `OcrBackend` trait + types | T5 (types/trait), T7 (Vision), T8 (WinOcr) | ✓ |
| §3 backend selection at plugin init | T10 | ✓ |
| §3 OcrQueue retains spawn_blocking, drops retry loop | T9 explicitly removes retry; T9 uses `spawn_blocking` | ✓ |
| §3 `ocr:ready` event + `get_ocr_words` command | T10 + T11 (TS) | ✓ |
| §4 Migration V006 contents (words_json, engine, pii_spans) | T2 (full SQL matches spec) | ✓ |
| §4 pii_spans lifecycle (pending/redacted/dismissed) | T4 (model), T25 (redact), T24 (dismiss) | ✓ |
| §4 Crate layout (snk-pii new, components in app/src/components/) | T1 (stub), T20 (full), T28–T30 | ✓ |
| §4 Plugin commands + events | T11, T24, T25 | ✓ |
| §5 PII categories + post-filters | T21 (email/phone/CC+Luhn), T22 (SSN/IP/API key) | ✓ |
| §5 Scanner flow (match → bbox → confidence → de-dupe → persist → emit) | T21–T23 (de-dupe in T23 worker `scan_one`) | ✓ |
| §5 Engine-version tracking on ocr_text | T2 (column), T9 (write path) | ✓ |
| §5 Destructive pixel redaction (toolbar/library), non-destructive in editor | T25 (destructive `redact_pii`), T32 (editor uses layered blur fallback) | ✓ |
| §5 Negative-control fixtures | T21 (Luhn reject), T22 (SSN invalid range, IP octet, etc) | ✓ |
| §6 `<TextOverlay>` selectable + ResizeObserver | T28 | ✓ |
| §6 `<PiiBadge>` warning token, count hide-when-0 | T29 | ✓ |
| §6 `<PiiReviewSheet>` per-item confirm, no bulk | T30 | ✓ |
| §6 Surface 1 post-capture toolbar (loading→ready, modal review) | T31 | ✓ |
| §6 Surface 2 editor (overlay toggle + right-docked panel + layered blur) | T32 | ✓ |
| §6 Surface 3 library viewer (tile dot + detail overlay + modal review) | T33 | ✓ |
| §6 Settings → About OCR engine + categories + last error rows | T34 | ✓ |
| §6 Empty / error states | T31 (Copy text disabled, OCR unavailable tooltip via existing useQuery loading states) | ✓ |
| §7 OcrError / PiiError typed | T5 (OcrError), T20 (PiiError) | ✓ |
| §7 Unit + integration tests | T2–T4, T7–T8, T12, T21–T22, T26 | ✓ |
| §7 Discovery spikes precede implementation | Spikes A + B | ✓ |
| §7 Build/release: deps add, Tesseract bundling removed, signing impact noted | T6 (deps), T15 (release.yml), T18 (verification) | ✓ |
| §7 SBOM auto-pickup via cdxgen | Implicit — cdxgen scans Cargo.toml + package.json automatically | ✓ |
| §8 Tesseract cleanse checklist | T13–T18 cover every checklist item | ✓ |
| §9 Implementation sequencing 4 PRs | Plan matches: PR-1 spikes, PR-2 foundation, PR-3 PII, PR-4 UI | ✓ |
| §10 Open questions | Addressed: migration number resolved to V006 in T2; About panel already shipped (T34 just adds rows); minimum macOS — note added below | ⚠ |
| §11 Acceptance criteria #1–#9 | All covered by smoke + automated tests in T18, T31–T34 | ✓ |

**§10 open question — minimum macOS:** The spec flags that the project doesn't pin `minimumSystemVersion` in `tauri.conf.json`. Vision requires macOS 14+. **Add to plan: pin `bundle.macOS.minimumSystemVersion = "14.0"` in T14** (or as a sub-task here in self-review):

### Plan amendment: pin macOS 14 minimum

Edit `app/src-tauri/tauri.conf.json` (in T14 or as a small follow-up commit):

```json
"bundle": {
  "macOS": {
    "minimumSystemVersion": "14.0"
  }
}
```

If the file already has `bundle.macOS`, just add the `minimumSystemVersion` key. Update `README.md` § "Requirements" to mention macOS 14+.

---

## Execution handoff

**Plan complete and saved to [`docs/superpowers/plans/2026-05-25-phase10-ocr-surfaces.md`](docs/superpowers/plans/2026-05-25-phase10-ocr-surfaces.md).**

Three execution options:

**1. Subagent-Driven (this session)** — Dispatch a fresh subagent per task with code-reviewer feedback between tasks. Linear progression, single-thread. Best for tight oversight. Use sub-skill **h-superpowers:subagent-driven-development**.

**2. Team-Driven (this session, experimental)** — Multiple persistent agents work in parallel with peer-to-peer messaging. Natural fit for this plan because PR-2/-3/-4 can have an implementer + reviewer pair, and within PR-2 the Vision (T7) and WinOcr (T8) backends can land in parallel. Requires Opus 4.6+ and `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`. Costs 2–4× more. Use sub-skill **h-superpowers:team-driven-development**.

**3. Parallel Session (separate)** — Open a new session in the worktree with `executing-plans`. Batch execution with human checkpoints between PRs. Use sub-skill **h-superpowers:executing-plans**.

Per the user's workflow (memory `feedback_spec_pr_before_worktree.md`), the **plan also gets PR'd to main first** before any worktree is created. After plan-PR merges, the implementation worktree is created from main (which then contains both spec + plan).

**Which approach?**
