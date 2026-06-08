# Design: Real local app-bundle runtime for macOS capture development

**Date:** 2026-06-08
**Status:** Approved (Approach 1 from brainstorming)

## Why

Latest macOS privacy behavior is treating screen-capture as an **app-runtime identity** problem, not a leaf-binary problem. In this repo, raw `tauri dev` launches the app through a terminal-driven chain (`pnpm` / `node` / `cargo run`), and capture is implemented through `xcap`'s CoreGraphics path. The current result is unstable Screen Recording behavior in development even after multiple attempts to patch permissions, entitlements, and local signing around the debug binary.

The core issue is structural: raw `tauri dev` is not the right source of truth for macOS capture validation. The application should be tested under a runtime shape that looks like a real macOS app, with a real bundle identity that TCC can reason about consistently.

This design fixes that at the shell/runtime layer instead of continuing to fight it from inside `snk-capture`.

## Goals

1. Provide one **explicit, reliable macOS capture-testing workflow** for latest macOS.
2. Avoid requiring a full installer / DMG just to test capture behavior locally.
3. Keep plugin boundaries unchanged: `snk-capture` remains the capture owner; the app shell owns runtime concerns.
4. Preserve fast frontend/UI iteration separately from capture-validation runtime.
5. Make it obvious, during validation, which app identity and bundle path macOS is evaluating.

## Non-goals

- Making raw terminal-driven `tauri dev` the trusted macOS capture runtime.
- Replacing `xcap` with a native ScreenCaptureKit backend in this spec.
- Preserving older macOS behavior as part of this first fix.
- Refactoring plugin boundaries or introducing cross-plugin coupling to work around runtime identity problems.

## Architecture

### Runtime ownership

`snk-capture` continues to own:

- permission checks,
- capture orchestration,
- permission-denied UX,
- region / window / full-screen capture commands.

`app/src-tauri` owns:

- the macOS development runtime shape,
- local bundle creation / launch workflow,
- signing identity verification,
- any runtime-only messaging about unsupported raw-dev capture behavior.

This keeps the fix aligned with the real fault line: **macOS app identity belongs to the shell, not the capture plugin**.

### Official macOS capture runtime

Add a dedicated macOS command (named in this design as `pnpm dev:mac-capture`) that:

1. builds a local `.app` bundle,
2. signs it with a stable local identity,
3. launches that bundle from a stable local path,
4. runs the existing application and capture code inside that bundle runtime.

This command is intentionally **app-bundle-first**, not installer-first. The local `.app` is the artifact; a DMG / installer is not part of the workflow.

### Dual development workflows

On macOS, the repo should present two explicit workflows:

1. **UI iteration:** raw `tauri dev` remains acceptable for normal frontend / UI work.
2. **Capture validation:** `pnpm dev:mac-capture` is the authoritative path whenever screen-capture behavior matters.

This is not a workaround disguised as parity. It is an explicit acknowledgement that latest macOS capture behavior depends on app runtime identity in a way raw `tauri dev` does not model well.

## Developer workflow

### `tauri dev`

Keep `pnpm --filter @snk/app tauri dev` available for normal iteration, but document that it is **not** the macOS capture source of truth.

### `pnpm dev:mac-capture`

The new command should:

1. produce a local `.app` bundle without requiring installer generation,
2. ensure the launched app has the expected bundle identifier / signing identity,
3. surface the bundle path and identity in command output,
4. relaunch manually on code changes rather than pretending Rust hot-reload is reliable for this case.

Manual restart is acceptable here because the goal is runtime correctness, not hot-reload convenience.

## Error handling and UX

`snk-capture` should continue to surface typed permission-denied errors. On macOS, that UX should point users toward the **official capture-validation runtime** when capture is attempted from an unsupported raw-dev path.

The dev command should fail loudly if:

- the `.app` bundle was not produced,
- the expected bundle identifier is missing,
- the signing identity does not match the intended local identity,
- the launched runtime is not the bundled app.

## Validation requirements

The first implementation plan must include proof that the runtime shape is correct. Validation must show:

1. the local app appears in **System Settings → Privacy & Security → Screen Recording** under the expected identity,
2. the grant survives relaunch,
3. region / full-screen / window capture work from the local `.app` runtime,
4. the command output identifies the running bundle path, executable path, bundle identifier, and signing identity.

If raw `tauri dev` still fails to capture afterward, that is acceptable. The requirement is not “make every dev path work”; the requirement is “define one reliable macOS capture-testing path and make it explicit.”

## Risks

1. **Bundle-build speed:** local `.app` generation may be slower than raw `tauri dev`. Accepted for the capture-validation path.
2. **Documentation drift:** if the repo continues to imply `tauri dev` is enough for macOS capture testing, developers will fall back into the broken path. The workflow must be documented unambiguously.
3. **Dependency blind spots:** `xcap` still uses the CoreGraphics path. This spec deliberately avoids solving backend choice until runtime identity is fixed first.

## Decisions log

- **2026-06-08 — latest-macOS-first.** This spec optimizes for the newest macOS behavior rather than preserving older-development-runtime assumptions.
- **2026-06-08 — real app bundle over raw dev runtime.** For capture validation, a local `.app` bundle is the authoritative macOS runtime; raw `tauri dev` is not.
- **2026-06-08 — no installer requirement.** The solution must not require a DMG / installer for normal local capture testing.
- **2026-06-08 — backend replacement deferred.** Native ScreenCaptureKit may be the long-term macOS backend, but it is not part of this first spec.
