# Region Snap Preview-Cache Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Stop the region-capture overlay from showing a stale full-screen preview on the second-and-subsequent snap. Make each call to `grab_screen_preview` mint a unique token so the WebView can't serve cached bytes and React can't bail on identical state.

**Architecture:** Rust returns `ScreenPreview { path, width, height, token }` where `token` is a fresh UUIDv7 string per call. The library window forwards `path + token` through the `overlay:preview` event. The capture overlay builds the background URL as `${convertFileSrc(path)}?v=${token}` and force-resets `previewSrc` to `null` before assigning the new URL so React always commits.

**Tech Stack:** Rust (Tauri command, `uuid` crate already a workspace dep), TypeScript binding in `@snk/capture`, React event listener in `CaptureOverlay`, vitest for TS tests, `cargo test` for Rust.

**Spec:** [`docs/superpowers/specs/2026-05-23-region-snap-preview-cache-design.md`](../specs/2026-05-23-region-snap-preview-cache-design.md)

**Plan-as-source-of-truth:** If you spot a bug in this plan while implementing, raise it to team-lead/user with a proposed fix before applying. Don't silently diverge.

**Worktree:** Per repo convention, work in `C:/Users/ehart/repos/snapper-keeper-worktrees/fix/region-snap-preview-cache/` on branch `fix/region-snap-preview-cache`. The execution skill will create this.

---

## Task 1: Add `mint_preview_token` helper with unit test

The live `grab_screen_preview` command needs a real `AppHandle` and a real screen grab, so it can't be unit-tested directly. Factor the token mint into a tiny pure helper, test that, and have the command call it.

**Files:**
- Modify: `crates/snk-capture/src/commands.rs`

**Step 1: Write the failing test**

Append to the bottom of `crates/snk-capture/src/commands.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_preview_token_yields_unique_strings() {
        let a = mint_preview_token();
        let b = mint_preview_token();
        assert_ne!(a, b, "two calls must return different tokens");
        assert!(!a.is_empty());
        assert!(!b.is_empty());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p snk-capture mint_preview_token_yields_unique_strings`
Expected: compile failure — `mint_preview_token` is undefined.

**Step 3: Add the helper**

Above the `ScreenPreview` struct in the same file, add:

```rust
/// Mint a fresh cache-busting token for one preview write.
/// UUIDv7 is monotonic so two consecutive calls always differ.
fn mint_preview_token() -> String {
    uuid::Uuid::now_v7().to_string()
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p snk-capture mint_preview_token_yields_unique_strings`
Expected: 1 passed.

**Step 5: Commit**

```bash
git add crates/snk-capture/src/commands.rs
git commit -m "test(capture): mint_preview_token returns unique strings"
```

---

## Task 2: Add `token` field to `ScreenPreview` and populate it

**Files:**
- Modify: `crates/snk-capture/src/commands.rs`

**Step 1: Extend the struct**

Replace the `ScreenPreview` struct definition with:

```rust
#[derive(serde::Serialize)]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub token: String,
}
```

**Step 2: Populate `token` from the helper**

In `grab_screen_preview`, change the returned struct literal from:

```rust
    Ok(ScreenPreview {
        path: preview_path.to_string_lossy().into_owned(),
        width: result.width,
        height: result.height,
    })
```

to:

```rust
    Ok(ScreenPreview {
        path: preview_path.to_string_lossy().into_owned(),
        width: result.width,
        height: result.height,
        token: mint_preview_token(),
    })
```

**Step 3: Verify build**

Run: `cargo build -p snk-capture`
Expected: clean build, no warnings about unused fields.

**Step 4: Run all snk-capture tests**

Run: `cargo test -p snk-capture`
Expected: all pass (existing tests don't touch `ScreenPreview`).

**Step 5: Commit**

```bash
git add crates/snk-capture/src/commands.rs
git commit -m "feat(capture): mint a cache-bust token per grab_screen_preview"
```

---

## Task 3: Extend the TS binding type and update its test

**Files:**
- Modify: `packages/snk-capture/src/index.ts`
- Modify: `packages/snk-capture/src/index.test.ts`

**Step 1: Update the failing test first**

In `packages/snk-capture/src/index.test.ts`, replace the `grabScreenPreview` test body with:

```ts
  it('grabScreenPreview returns the preview struct including token', async () => {
    mockedInvoke.mockResolvedValue({
      path: '/tmp/x.png',
      width: 800,
      height: 600,
      token: 'tok-abc',
    });
    const p = await grabScreenPreview();
    expect(p.path).toBe('/tmp/x.png');
    expect(p.width).toBe(800);
    expect(p.height).toBe(600);
    expect(p.token).toBe('tok-abc');
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview');
  });
```

**Step 2: Run test to verify it fails**

Run: `pnpm --filter @snk/capture test`
Expected: FAIL — `Property 'token' does not exist on type 'ScreenPreview'`.

**Step 3: Add `token` to the interface**

In `packages/snk-capture/src/index.ts`, replace the `ScreenPreview` interface with:

```ts
export interface ScreenPreview {
  path: string;
  width: number;
  height: number;
  token: string;
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm --filter @snk/capture test`
Expected: all pass.

**Step 5: Commit**

```bash
git add packages/snk-capture/src/index.ts packages/snk-capture/src/index.test.ts
git commit -m "feat(capture-bindings): expose preview token in ScreenPreview"
```

---

## Task 4: Forward the token through the `overlay:preview` event

`LibraryWindow.handleRegion` currently emits `{ path: preview.path }`. Add `token` to the payload and extend the test that already covers this path.

**Files:**
- Modify: `app/src/windows/library/LibraryWindow.tsx:101-113`
- Modify: `app/src/windows/library/LibraryWindow.test.tsx` (the existing `'region hotkey grabs a preview and shows the overlay'` test)

**Step 1: Strengthen the existing test**

Find the test starting at `it('region hotkey grabs a preview and shows the overlay', …)` in `LibraryWindow.test.tsx`. We need access to the overlay's `emit` mock. Replace the test body with:

```tsx
  it('region hotkey grabs a preview and emits overlay:preview with path+token', async () => {
    let regionHandler: ((e: { payload: unknown }) => void) | null = null;
    vi.mocked(listen).mockImplementation((event, handler) => {
      if (event === 'hotkey:capture-region') {
        regionHandler = handler as typeof regionHandler;
      }
      return Promise.resolve(() => {});
    });

    const overlayEmit = vi.fn().mockResolvedValue(undefined);
    vi.mocked(WebviewWindow.getByLabel).mockImplementation(async (label: string) => {
      if (label === 'capture-overlay') {
        return {
          emit: overlayEmit,
          show: vi.fn().mockResolvedValue(undefined),
          setFocus: vi.fn().mockResolvedValue(undefined),
        } as unknown as WebviewWindow;
      }
      return null;
    });

    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|grab_screen_preview') {
        return Promise.resolve({
          path: '/tmp/p.png',
          width: 1,
          height: 1,
          token: 'tok-xyz',
        });
      }
      return Promise.resolve([]);
    });

    renderWithQuery(<LibraryWindow />);
    await waitFor(() => expect(regionHandler).not.toBeNull());

    await act(async () => regionHandler!({ payload: undefined }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('plugin:snk-capture|grab_screen_preview');
      expect(WebviewWindow.getByLabel).toHaveBeenCalledWith('capture-overlay');
      expect(overlayEmit).toHaveBeenCalledWith('overlay:preview', {
        path: '/tmp/p.png',
        token: 'tok-xyz',
      });
    });
  });
```

**Step 2: Run test to verify it fails**

Run: `pnpm --filter @snk/app test LibraryWindow`

Expected: FAIL — the emit assertion sees `{ path: '/tmp/p.png' }` without `token`.

(If you'd rather filter by test name: `pnpm --filter @snk/app test -t 'region hotkey'`.)

**Step 3: Update `handleRegion` to forward the token**

In `app/src/windows/library/LibraryWindow.tsx`, find `handleRegion`. Replace the emit call:

```ts
        await overlay.emit('overlay:preview', { path: preview.path });
```

with:

```ts
        await overlay.emit('overlay:preview', {
          path: preview.path,
          token: preview.token,
        });
```

**Step 4: Run test to verify it passes**

Run the same vitest command from step 2. Expected: pass.

Also run the full library test file to confirm no regressions:

Run: `pnpm --filter @snk/app test LibraryWindow`
Expected: all pass.

**Step 5: Commit**

```bash
git add app/src/windows/library/LibraryWindow.tsx app/src/windows/library/LibraryWindow.test.tsx
git commit -m "fix(capture): forward preview token through overlay:preview event"
```

---

## Task 5: Cache-bust the URL and force-reset state in `CaptureOverlay`

**Files:**
- Modify: `app/src/windows/capture-overlay/CaptureOverlay.tsx:86-94`

**Step 1: Update the listener**

In `CaptureOverlay.tsx`, find the `useEffect` that registers the `overlay:preview` listener (around line 86). Replace this block:

```tsx
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ path: string }>('overlay:preview', (event) => {
      setPreviewSrc(convertFileSrc(event.payload.path));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);
```

with:

```tsx
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ path: string; token: string }>('overlay:preview', (event) => {
      // Cache-bust: WebView caches by URL, and the preview file path
      // never changes. Per-call ?v=<token> forces a fresh fetch and
      // the null-then-set guarantees React commits even on identical
      // strings.
      const url = `${convertFileSrc(event.payload.path)}?v=${event.payload.token}`;
      setPreviewSrc(null);
      setPreviewSrc(url);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);
```

**Step 2: Type-check the change**

Run: `pnpm --filter @snk/app typecheck`
Expected: clean type-check.

**Step 3: Run the full frontend test suite**

Run: `pnpm --filter @snk/app test`
Expected: all pass. No new test is added for `CaptureOverlay` — the surface is thin and would need heavy Tauri mocking for marginal value (the spec calls this out explicitly).

**Step 4: Lint**

Run: `pnpm --filter @snk/app lint`
Expected: clean.

**Step 5: Commit**

```bash
git add app/src/windows/capture-overlay/CaptureOverlay.tsx
git commit -m "fix(capture): cache-bust preview URL and force state commit"
```

---

## Task 6: Full workspace check + manual smoke

**Step 1: Run the whole Rust test suite**

Run: `cargo test --workspace`
Expected: all pass.

**Step 2: Run the whole frontend test suite**

Run: `pnpm -r test`
Expected: all pass.

**Step 3: Manual smoke (requires interactive desktop session)**

Per project CLAUDE.md, Windows smoke needs an interactive desktop — not an SSH session. If you're in SSH, skip this step and leave a note in the PR description; CI verifies the compile.

If on interactive desktop:

1. From the worktree, run `pnpm tauri dev` (or whatever launches the app — check `package.json` scripts).
2. Trigger region snap. Drag and complete a region. The dim backdrop should show the live desktop.
3. Switch apps so the desktop visibly changes (open a different window, scroll something).
4. Trigger region snap again. **The dim backdrop must show the new desktop state, not the previous one.**
5. Cancel with Esc. Repeat once more to confirm consistency.

If any of steps 2–5 show stale content, stop and report the failure mode — don't merge.

**Step 4: There is no separate commit for this task.**

This task is verification only. Move on to opening the PR once it's clean.

---

## Pull-request prep

After all tasks land:

- Branch: `fix/region-snap-preview-cache`
- Commits (in order):
  1. `test(capture): mint_preview_token returns unique strings`
  2. `feat(capture): mint a cache-bust token per grab_screen_preview`
  3. `feat(capture-bindings): expose preview token in ScreenPreview`
  4. `fix(capture): forward preview token through overlay:preview event`
  5. `fix(capture): cache-bust preview URL and force state commit`
- PR title suggestion: `fix(capture): stop region overlay from showing stale preview`
- PR body should link the design doc and call out the smoke caveat for SSH sessions.
