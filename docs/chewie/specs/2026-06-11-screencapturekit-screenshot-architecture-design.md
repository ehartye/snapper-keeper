# Design: ScreenCaptureKit screenshot architecture for macOS

**Date:** 2026-06-11
**Status:** Approved (Approach 1 from brainstorming)

## Why

The current macOS screenshot stack is not structurally reliable on latest macOS. Today `snk-capture` uses `xcap`, which on macOS still relies on deprecated CoreGraphics-era capture behavior. That path has already shown multiple failure modes in this repo:

- raw `tauri dev` is not a trustworthy screenshot-validation runtime on macOS,
- region preview needed a bundled-runtime-specific fix just to render reliably,
- the "hide snapper-keeper windows during capture" setting resolves correctly and the app does call `hide()`, but the captured image still includes Snapper Keeper content.

At that point, the problem is no longer a missing delay or a broken toggle. The screenshot backend itself is the wrong place to keep accumulating patches.

This design replaces the macOS screenshot backend with a ScreenCaptureKit-backed architecture and introduces an explicit cross-platform screenshot abstraction so future platform-specific capture changes do not continue to leak through ad hoc `#[cfg]` branches.

## Goals

1. Introduce an explicit **cross-platform screenshot backend abstraction** inside `snk-capture`.
2. Make **ScreenCaptureKit** the source of truth for all macOS screenshot surfaces:
   - full-screen capture,
   - window capture,
   - region preview,
   - final region capture.
3. Make the macOS `hide_own_windows` behavior structural by excluding Snapper Keeper content in the capture backend itself instead of relying on timing races.
4. Keep Windows/Linux working through adapter implementations behind the same abstraction.
5. Preserve one explicit, reliable macOS screenshot-validation runtime based on the bundled app workflow.
6. Surface typed, consistent screenshot capability and runtime errors back to the frontend.

## Non-goals

- Video recording.
- OCR, annotation, library, or clipboard redesign.
- Rewriting Windows/Linux capture internals beyond what is necessary to fit the new abstraction.
- Moving immediately to a helper-process/service architecture unless ScreenCaptureKit integration still proves insufficient later.

## Architecture

### 1. Screenshot contract inside `snk-capture`

`snk-capture` gains an explicit backend boundary for screenshot work. The rest of the crate stops depending directly on `xcap`-shaped functions and instead depends on normalized capture contracts.

At a high level, the contract should cover two responsibilities:

1. **Capture catalog**
   - enumerate displays / monitors,
   - enumerate capturable windows,
   - map frontend-selected monitor/window targets into backend-native references.

2. **Screenshot backend**
   - capture full-screen,
   - capture a target window,
   - capture display pixels for region preview,
   - capture display pixels for final region output,
   - enforce own-window exclusion semantics when the setting is enabled.

The output shape remains normalized (`png_bytes`, width, height, monitor metadata) so `commands.rs`, `orchestrate.rs`, and the library persistence path stay platform-neutral.

### 2. macOS backend: ScreenCaptureKit as the source of truth

The macOS implementation moves to ScreenCaptureKit for all screenshot surfaces.

The design assumption is that own-window hiding on latest macOS should no longer be modeled as:

> hide our windows → wait a bit → hope the compositor is done → capture

Instead, the backend should construct ScreenCaptureKit content filters that exclude Snapper Keeper’s own app/window content whenever `capture.hide_own_windows` is enabled.

That means:

- **full-screen** captures target a display with an exclusion filter,
- **region preview** and **final region capture** both derive from the same filtered display source, with region cropping performed after pixel acquisition or via ScreenCaptureKit source-rect configuration,
- **window capture** targets an external window selected from the catalog layer; Snapper Keeper windows should never be offered as valid capture targets.

The macOS screenshot backend therefore owns both pixel acquisition and own-content exclusion semantics.

### 3. Windows/Linux adapter path

Windows/Linux remain behind adapter implementations that satisfy the same screenshot contract.

The design does **not** require a same-day rewrite of non-macOS capture internals. Those adapters may continue using the current backend path as long as they conform to the new contract. This keeps the first ScreenCaptureKit migration focused on macOS while still paying down the architectural debt of a macOS-only special case.

### 4. App-shell runtime responsibilities stay in the shell

The app shell (`app/src-tauri`) continues to own runtime concerns:

- bundled-app validation workflow (`pnpm dev:mac-capture` or its successor),
- runtime classification / unsupported-dev-mode messaging,
- bundle identity / signing visibility for macOS testing.

This design does **not** move runtime policy into the screenshot backend. The backend assumes it is being called from a runtime that is valid for screenshot testing; the shell remains responsible for making that runtime explicit and discoverable.

## Data flow

### Full-screen

1. Frontend requests full-screen capture.
2. `snk-capture` resolves the target display through the catalog layer.
3. On macOS, the ScreenCaptureKit backend captures from that display with Snapper Keeper excluded when the setting is enabled.
4. The normalized frame result flows through persistence and toolbar logic unchanged.

### Region preview and final region capture

1. Frontend selects the active monitor and requests a preview.
2. Backend captures filtered display pixels for that monitor.
3. `commands.rs` writes the preview PNG under the capture-scoped asset path for overlay rendering.
4. On drag completion, frontend sends the selected region back.
5. Backend captures from the same filtered display source and produces the final cropped image.

This keeps preview and final region capture on the same backend semantics instead of letting them drift.

### Window capture

1. Catalog layer enumerates capturable windows.
2. Snapper Keeper windows are excluded from user-selectable capture targets.
3. Backend captures the selected external window through the platform-native path.

## Runtime and permissions

The ScreenCaptureKit backend and the macOS runtime must be designed together.

### Supported runtime

The bundled app workflow remains the authoritative screenshot-validation runtime on macOS. Raw `tauri dev` may remain available for general UI work, but the screenshot stack should explicitly treat it as non-authoritative for screenshot correctness.

### Permission model

macOS permission handling should be expressed as one typed screenshot capability state at the backend boundary. The frontend should not need to infer backend specifics from vague OS failures.

The design should prefer ScreenCaptureKit-native capability and permission behavior where possible instead of continuing to split responsibility across:

- `xcap`,
- CoreGraphics preflight assumptions,
- shell-script identity handling,
- frontend heuristics.

## Error handling

`snk-capture` should continue returning typed errors, but macOS-specific screenshot failures should become clearer:

- unsupported runtime,
- missing screenshot permission,
- capture target not found,
- backend capture failure,
- own-window exclusion failure.

If Snapper Keeper’s own content cannot be excluded when the setting is enabled, that is a correctness failure, not a best-effort warning.

## Testing and validation

### Rust / crate-level testing

- Unit tests should cover the contract-level mapping logic and platform-neutral result normalization.
- macOS-specific implementation details that cannot be exercised in CI should be isolated behind small seams with the maximum practical unit coverage.
- Existing command/ACL smoke coverage should remain intact.

### Frontend / UX testing

- Existing frontend tests should be updated only where screenshot runtime or permission UX changes.
- The frontend should continue testing the supported-runtime guidance path.

### Manual macOS validation

The acceptance bar for macOS must cover **all screenshot surfaces** in the supported bundled runtime:

1. full-screen capture,
2. region preview,
3. final region capture,
4. window capture,
5. hide-own-windows enabled / disabled behavior,
6. permission UX and bundled-runtime guidance.

The success criterion is simple: when own-window hiding is enabled, Snapper Keeper content does not appear in screenshot output produced by the supported runtime.

## Risks and open questions

1. **Rust integration complexity.** ScreenCaptureKit bindings and permission/capability handling are more complex than the current backend. The design accepts that complexity because it aligns with the real macOS platform model.
2. **Cross-platform abstraction sizing.** If the contract is too thin, macOS-specific concerns will leak upward again. If it is too broad, the first migration gets bogged down. The implementation plan must keep the abstraction screenshot-focused and avoid premature video/general media concepts.
3. **Coordinate-space correctness.** Region preview/final capture must stay aligned across monitor scaling and platform coordinate systems. This is an implementation risk, not a reason to stay on the old backend.
4. **Window-target filtering semantics.** The implementation must define exactly how Snapper Keeper windows are identified and excluded on macOS, and how that maps into the normalized window list exposed to the frontend.

## Decisions log

- **2026-06-11 — ScreenCaptureKit replaces the macOS screenshot backend.** The current CoreGraphics/xcap path is no longer considered sufficient for latest macOS screenshot correctness.
- **2026-06-11 — the first ScreenCaptureKit spec includes all screenshot surfaces.** Full-screen, window, region preview, final region capture, own-window exclusion, permissions, and runtime flow are in scope together.
- **2026-06-11 — the design includes a cross-platform screenshot abstraction.** macOS does not get another one-off path bolted directly into the existing public shape.
- **2026-06-11 — helper/service architecture is deferred.** It remains a possible later move, but it is not the first response while a cleaner in-process ScreenCaptureKit architecture remains plausible.
