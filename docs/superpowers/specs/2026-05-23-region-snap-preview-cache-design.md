# Region snap preview-cache fix

**Status:** Draft
**Date:** 2026-05-23
**Scope:** Bug fix — single behavior, no feature work.

## Problem

On the second (and subsequent) region capture in a session, the dimmed
backdrop in the region-select overlay shows the screenshot from the
**previous** capture, not the current desktop. Users see a stale "base"
image and can't accurately frame their region against what's currently
on screen.

Reproduction:

1. Trigger region snap. Drag a region. Overlay shows the live desktop —
   correct.
2. Switch apps or change the screen contents.
3. Trigger region snap again. Overlay shows the desktop state from
   step 1 — wrong.

The captured PNG that lands in the library is correct (Rust grabs a
fresh frame at drag-end). Only the dim backdrop preview is stale.

## Root cause

`grab_screen_preview` (in `crates/snk-capture/src/commands.rs`) writes
the full-screen PNG to a fixed path: `<appDataDir>/.preview.png`. The
frontend pipes that path through `convertFileSrc()` to produce a
WebView-compatible URL and assigns it to `previewSrc` state in
`CaptureOverlay`.

Because the path never changes, `convertFileSrc()` returns the
**identical URL string** every time. Two layers cache it:

- The WebView's HTTP cache holds the bytes from the first load.
- React's `useState` setter bails out when `setPreviewSrc(newUrl)` is
  called with a string equal to the current value, so the `<div>`
  background never re-paints.

Even if React re-rendered, the WebView would serve the cached bytes —
so both layers need to be addressed.

## Approach

Cache-bust the URL with a per-call token and force a React state reset
on each preview event.

### Rust side (`crates/snk-capture/src/commands.rs`)

Add a `token: String` field to `ScreenPreview`. Populate with
`uuid::Uuid::now_v7().to_string()` on each call. The PNG file is still
written to `.preview.png` (overwrite in place — no GC concern).

```rust
#[derive(serde::Serialize)]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub token: String,
}
```

### TS binding (`packages/snk-capture/src/index.ts`)

Extend the `ScreenPreview` type / return shape to include `token`.

### Library window (`app/src/windows/library/LibraryWindow.tsx`)

`handleRegion` passes both fields through:

```ts
const preview = await grabScreenPreview();
await overlay.emit('overlay:preview', {
  path: preview.path,
  token: preview.token,
});
```

### Capture overlay (`app/src/windows/capture-overlay/CaptureOverlay.tsx`)

Listener builds the URL with a query-string buster and force-resets
state so React commits even if the URL string happened to repeat:

```ts
listen<{ path: string; token: string }>('overlay:preview', (event) => {
  const url = `${convertFileSrc(event.payload.path)}?v=${event.payload.token}`;
  setPreviewSrc(null);
  setPreviewSrc(url);
});
```

No other CaptureOverlay state changes — `rect` and `dragging` already
reset in both `cancel()` and the successful `handleMouseUp()` path.

## Out of scope

- Window/full-screen captures do not use the preview file, so this fix
  does not touch them.
- No GC of the preview file: in-place overwrite is sufficient because
  the path is stable and a single file is reused forever.
- Annotation/crop derivations are unaffected — they read from the
  capture's permanent library path, not `.preview.png`.

## Testing

**Rust unit tests** (`crates/snk-capture/src/commands.rs`):

- Add a test that calls `grab_screen_preview` (or extracts the
  token-mint helper) twice and asserts the two `token` fields differ.
  If the live-capture path is awkward to unit-test, factor token minting
  into a tiny helper and test that directly.

**TS unit tests**:

- `packages/snk-capture/src/index.test.ts`: update the
  `grabScreenPreview` mock and assertions to include the `token` field.
- `app/src/windows/library/LibraryWindow.test.tsx`: assert
  `overlay:preview` is emitted with both `path` and `token`.

**No new CaptureOverlay test.** The event listener is thin and would
require heavy Tauri mocking for marginal value; the binding + library
tests cover the contract end-to-end.

**Manual smoke** (requires interactive desktop session per project
policy):

1. Launch app. Trigger region snap. Cancel.
2. Switch apps so the desktop visibly changes.
3. Trigger region snap again. Confirm dim backdrop shows the new
   desktop state, not the previous one.
4. Repeat once more to confirm it's not just a one-shot fix.

## Risk

Very low. The change touches one Rust struct field, one TS type, one
emit payload, and one event listener body. The preview file's on-disk
behavior is unchanged. Worst case if a regression slips: same stale-
preview bug as today.
