# Blur Tool Pixelation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Replace the existing `BlurShape` (a 12px translucent grey checkerboard that doesn't redact anything) with real pixelation that samples the underlying screenshot and is unreadable at any zoom.

**Architecture:** The annotate store gains a `sourceImage: HTMLImageElement | null` field set by `AnnotateCanvas` once the screenshot loads. `BlurShape` becomes a single `Konva.Image` that crops the source image to the shape's rect and applies `Konva.Filters.Pixelate` at 20-px blocks, cached via `node.cache()` from a `useEffect` keyed on geometry. `AnnotateCanvas` shows a cheap dashed-Rect placeholder for the in-progress draft of a blur drag so we don't re-cache on every mousemove.

**Tech Stack:** React 18 + TypeScript, Konva 10 (`react-konva`), Zustand 4, Vitest 2 for the store tests. No new dependencies. Spec: `docs/superpowers/specs/2026-05-22-blur-tool-pixelation-design.md`.

---

## Background context the engineer needs

- The annotate window has its own Zustand store at `app/src/windows/annotate/store.ts`. All other annotate components subscribe to it. Tests for the store live at `app/src/windows/annotate/store.test.ts`.
- The Konva-bound files (`AnnotateCanvas.tsx`, `useDrawing.tsx`, every file under `shapes/`) are excluded from Vitest coverage via `app/vitest.config.ts` — `happy-dom` has no canvas implementation, so unit tests against Konva nodes don't work. **Tasks that touch only those files have no automated tests; they're verified with manual smoke + `pnpm typecheck` + `pnpm lint`.**
- Konva filters require `node.cache()` to be called before they render. See https://konvajs.org/docs/filters/Pixelate.html.
- The currently-loaded screenshot inside `AnnotateCanvas` is a plain `HTMLImageElement` kept in a `useState`. We don't replace that — we add a second handle to it from the store so `BlurShape` can read it without prop drilling through `ShapeRenderer`.

## Worktree

Before doing anything, set up an isolated worktree per the `using-git-worktrees` skill. Branch name: `feature/blur-pixelation`. All commits in this plan land on that branch.

---

## Task 1: Add `sourceImage` field + setter to the annotate store

**Files:**
- Modify: `app/src/windows/annotate/store.ts`
- Modify: `app/src/windows/annotate/store.test.ts`

**Step 1: Write the failing tests**

Open `app/src/windows/annotate/store.test.ts`. Find the existing `describe('useAnnotateStore', ...)` block. Add these two test cases at the end of it, just before the closing `});`:

```ts
  it('sourceImage starts null and round-trips through setSourceImage', () => {
    expect(useAnnotateStore.getState().sourceImage).toBeNull();
    const img = new window.Image();
    useAnnotateStore.getState().setSourceImage(img);
    expect(useAnnotateStore.getState().sourceImage).toBe(img);
    useAnnotateStore.getState().setSourceImage(null);
    expect(useAnnotateStore.getState().sourceImage).toBeNull();
  });

  it('reset clears sourceImage back to null', () => {
    const img = new window.Image();
    useAnnotateStore.getState().setSourceImage(img);
    expect(useAnnotateStore.getState().sourceImage).toBe(img);
    useAnnotateStore.getState().reset();
    expect(useAnnotateStore.getState().sourceImage).toBeNull();
  });
```

**Step 2: Run tests to verify they fail**

```
pnpm --filter @snk/app exec vitest run src/windows/annotate/store.test.ts
```

Expected: 2 failing tests with errors like `setSourceImage is not a function` or `Cannot read properties of undefined (reading 'sourceImage')`.

**Step 3: Implement the field + setter**

In `app/src/windows/annotate/store.ts`, make three edits:

(a) Add the field to the `AnnotateState` interface. Find the existing interface block and add `sourceImage: HTMLImageElement | null;` after `selectedId: string | null;` and `setSourceImage: (img: HTMLImageElement | null) => void;` after the existing setter declarations. The relevant chunk should read:

```ts
interface AnnotateState {
  // ... existing fields above ...
  selectedId: string | null;
  sourceImage: HTMLImageElement | null;

  // ... existing setters above ...
  setSelectedId: (id: string | null) => void;
  setSourceImage: (img: HTMLImageElement | null) => void;
  reset: () => void;
}
```

(b) Add `sourceImage: null` to `initialState`. The block becomes:

```ts
const initialState = {
  tool: 'select' as AnnotationTool,
  color: '#ef4444',
  strokePreset: 'medium' as StrokePreset,
  shapes: [] as AnnotationShape[],
  undoStack: [] as AnnotationShape[][],
  redoStack: [] as AnnotationShape[][],
  nextStepNumber: 1,
  cropRegion: null as CropRegion | null,
  cropConfirmed: false,
  isDrawing: false,
  currentShape: null as AnnotationShape | null,
  selectedId: null as string | null,
  sourceImage: null as HTMLImageElement | null,
};
```

(c) Add the setter to the store body. Find the existing `setSelectedId` line and add right after it:

```ts
  setSelectedId: (id) => set({ selectedId: id }),
  setSourceImage: (img) => set({ sourceImage: img }),
  reset: () => set(initialState),
```

**Step 4: Run tests to verify they pass**

```
pnpm --filter @snk/app exec vitest run src/windows/annotate/store.test.ts
```

Expected: all tests pass (including the existing ones and the 2 new ones).

**Step 5: Run the full app typecheck**

```
pnpm --filter @snk/app typecheck
```

Expected: no errors.

**Step 6: Commit**

```
git add app/src/windows/annotate/store.ts app/src/windows/annotate/store.test.ts
git commit -m "feat(annotate): add sourceImage field to store

The blur tool will read the loaded screenshot from the store so it
can sample the underlying pixels for real pixelation. Pure state
addition — no consumers yet.
"
```

---

## Task 2: Push the loaded screenshot into the store from AnnotateCanvas

**Files:**
- Modify: `app/src/windows/annotate/AnnotateCanvas.tsx`

This file is excluded from coverage; verification is `pnpm typecheck`, `pnpm lint`, and a manual smoke that the existing annotate UX still works.

**Step 1: Add the setter import and call it from the image-load effect**

Open `app/src/windows/annotate/AnnotateCanvas.tsx`.

Near the top of the `AnnotateCanvas` function body, where the other store subscriptions live, add:

```tsx
  const setSourceImage = useAnnotateStore((s) => s.setSourceImage);
```

Then find the existing `useEffect` that loads the screenshot:

```tsx
  useEffect(() => {
    const img = new window.Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => setImage(img);
    img.src = imageSrc;
  }, [imageSrc]);
```

Replace it with:

```tsx
  useEffect(() => {
    const img = new window.Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      setImage(img);
      setSourceImage(img);
    };
    img.src = imageSrc;
    // Drop the store reference when we unmount or the src changes so the
    // next capture loads cleanly. (reset() also clears it.)
    return () => {
      setSourceImage(null);
    };
  }, [imageSrc, setSourceImage]);
```

**Step 2: Typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Expected: both clean.

**Step 3: Smoke test that the annotator still works**

```
pnpm --filter @snk/app tauri dev
```

In the running app:

1. Capture a screenshot (`Ctrl+Shift+3`).
2. Click the thumbnail in the library to open the annotator.
3. Confirm the screenshot displays correctly (this validates that pushing `img` into the store doesn't break the existing `<KonvaImage image={image}>`).
4. Draw an arrow somewhere with the Arrow tool — confirm it renders. (Sanity that nothing else regressed.)
5. Close the annotator.

Stop the dev server.

**Step 4: Commit**

```
git add app/src/windows/annotate/AnnotateCanvas.tsx
git commit -m "feat(annotate): publish the loaded screenshot to the store

BlurShape needs direct access to the source HTMLImageElement to apply
Konva's Pixelate filter against it. AnnotateCanvas now mirrors its
local image state into useAnnotateStore.sourceImage. Cleared on src
change and on reset()."
```

---

## Task 3: Render a cheap placeholder while a blur shape is being drawn

**Files:**
- Modify: `app/src/windows/annotate/AnnotateCanvas.tsx`

Excluded from coverage; verified by typecheck + lint + manual.

**Step 1: Replace the unconditional `<ShapeRenderer shape={currentShape}>` for the blur case**

In `app/src/windows/annotate/AnnotateCanvas.tsx`, find this line inside the second `<Layer>`:

```tsx
            {currentShape && <ShapeRenderer shape={currentShape} />}
```

Replace it with:

```tsx
            {currentShape && currentShape.tool === 'blur' ? (
              // While the user is dragging out a new blur rect, draw a
              // cheap dashed Rect instead of running Pixelate + cache on
              // every mousemove. The real BlurShape only renders for
              // committed shapes in the `shapes` array.
              <Rect
                x={currentShape.x ?? 0}
                y={currentShape.y ?? 0}
                width={currentShape.width ?? 0}
                height={currentShape.height ?? 0}
                stroke="#3b82f6"
                strokeWidth={2}
                dash={[6, 3]}
                fill="rgba(59, 130, 246, 0.15)"
              />
            ) : currentShape ? (
              <ShapeRenderer shape={currentShape} />
            ) : null}
```

`Rect` is already imported at the top of the file from `react-konva` (it's used for the crop region), so there's no import to add.

**Step 2: Typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Expected: clean.

**Step 3: Smoke test the placeholder**

```
pnpm --filter @snk/app tauri dev
```

In the annotator:

1. Pick the **Blur** tool from the left toolbar.
2. Slowly drag a rectangle across the canvas.
3. Confirm the in-progress rectangle renders as a dashed blue outline with a faint blue fill (the new placeholder), not as the existing grey checkerboard.
4. Release the mouse — confirm the committed shape STILL shows the old grey checkerboard for now (we haven't rewritten `BlurShape` yet). This is the expected intermediate state.
5. Confirm other tools (Arrow, Rectangle, Pen) still draw normally — i.e. the `?:` ternary correctly falls through to `ShapeRenderer` for non-blur tools.

Stop the dev server.

**Step 4: Commit**

```
git add app/src/windows/annotate/AnnotateCanvas.tsx
git commit -m "feat(annotate): cheap dashed-Rect placeholder during blur draw

BlurShape's upcoming Konva.Filters.Pixelate implementation requires
node.cache() on every geometry change. Re-caching once per mousemove
during the initial drag would lag on large drags. While currentShape
is a blur draft, render the same dashed blue rect the crop tool uses
for its draft state instead — the real BlurShape only renders for
committed shapes."
```

---

## Task 4: Rewrite BlurShape to apply Konva.Filters.Pixelate

**Files:**
- Modify: `app/src/windows/annotate/shapes/BlurShape.tsx` (full rewrite)

Excluded from coverage; verified by typecheck + lint + manual smoke (the spec's full test plan).

**Step 1: Replace the BlurShape implementation entirely**

Open `app/src/windows/annotate/shapes/BlurShape.tsx` and replace its **entire contents** with:

```tsx
import { useEffect, useRef } from 'react';
import { Image as KonvaImage } from 'react-konva';
import Konva from 'konva';

import { useAnnotateStore } from '../store';
import type { ShapeProps } from './index';

// 20px gives macOS-style chunky pixelation. Large enough that even
// uppercase letters lose their identifiable shape entirely; small
// enough to still feel like a "redaction box" rather than a giant blob.
const PIXELATE_BLOCK_SIZE = 20;

export function BlurShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Image | null>(null);
  const sourceImage = useAnnotateStore((s) => s.sourceImage);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  const x = shape.x ?? 0;
  const y = shape.y ?? 0;
  const w = shape.width ?? 0;
  const h = shape.height ?? 0;

  // Konva filters only render against a cached node. Re-cache whenever
  // the source image or the shape geometry changes so the filter
  // re-runs against the right region.
  useEffect(() => {
    const node = ref.current;
    if (!node || !sourceImage || w <= 0 || h <= 0) return;
    node.cache();
    node.getLayer()?.batchDraw();
  }, [sourceImage, x, y, w, h]);

  useEffect(() => {
    registerNode?.(ref.current);
    return () => registerNode?.(null);
  }, [registerNode]);

  return (
    <KonvaImage
      ref={ref}
      image={sourceImage ?? undefined}
      x={x}
      y={y}
      width={w}
      height={h}
      // `crop` tells Konva to draw only this region of the source image.
      // The screenshot fills the layer at (0,0) at native pixel size,
      // so the shape's rect and the crop region happen to be identical.
      crop={{ x, y, width: w, height: h }}
      filters={[Konva.Filters.Pixelate]}
      pixelSize={PIXELATE_BLOCK_SIZE}
      draggable={draggable}
      onMouseDown={onSelect}
      onTap={onSelect}
      onDragEnd={(e) => {
        updateShape(shape.id, { x: e.target.x(), y: e.target.y() });
      }}
      onTransformEnd={() => {
        const node = ref.current;
        if (!node) return;
        const scaleX = node.scaleX();
        const scaleY = node.scaleY();
        node.scaleX(1);
        node.scaleY(1);
        updateShape(shape.id, {
          x: node.x(),
          y: node.y(),
          width: Math.max(5, w * scaleX),
          height: Math.max(5, h * scaleY),
        });
      }}
    />
  );
}
```

Three things to note about this rewrite:

1. **`import Konva from 'konva'`** (default import, not `type`) — `Konva.Filters.Pixelate` is a runtime value, not just a type.
2. **`image={sourceImage ?? undefined}`** — Konva accepts `undefined` and renders nothing for the brief moment between mount and the source image being set. Once the store fills in the source, the node re-renders and the cache effect fires.
3. **The drag/transform handlers** are intentionally a direct port of the previous BlurShape's. The change is purely the rendered output; selection / move / resize / delete must remain identical to other shapes.

**Step 2: Typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Expected: clean.

**Step 3: Run the full test suite to confirm no store regressions**

```
pnpm --filter @snk/app test
```

Expected: all 126 tests pass (124 existing + 2 added in Task 1).

**Step 4: Commit**

```
git add app/src/windows/annotate/shapes/BlurShape.tsx
git commit -m "feat(annotate): real pixelation via Konva.Filters.Pixelate

The previous BlurShape rendered a 12px translucent grey checkerboard
that didn't sample the screenshot — at 0.85 alpha, 15% of the
underlying pixels leaked through and text was fully readable at zoom.

Replace with a single KonvaImage that crops the source screenshot to
the shape's rect bounds and applies Konva.Filters.Pixelate at
PIXELATE_BLOCK_SIZE=20. Cached via node.cache() from a useEffect keyed
on geometry so the filter re-runs on resize/move.

Drag/select/transform/delete behaviour unchanged."
```

---

## Task 5: Manual verification — full test plan from the spec

**Step 1: Start the dev server**

```
pnpm --filter @snk/app tauri dev
```

**Step 2: Test redaction is real**

1. Capture a screenshot of a webpage with readable text (e.g. github.com/your-repo).
2. Open it in the annotator.
3. Pick the Blur tool, drag a rect over a paragraph of text.
4. Release — confirm the rect is now an opaque mosaic of ~20px coloured squares sampled from the underlying screenshot (the underlying letters should be unrecognisable).
5. Switch to the Select tool, then click Save in the top bar.
6. Confirm "saved ✓" flashes.
7. Open the library, click the annotated capture's "annotated" badge, locate the saved PNG on disk (`%APPDATA%\com.snapper-keeper.app\captures\YYYY\MM\<uuid>.annotated.png`), open it in Windows Photos.
8. Zoom to 400% — confirm individual characters under the blur are unrecoverable.

**Step 3: Test it behaves like a Rectangle**

In the same annotation session:

1. Pick the Select tool.
2. Click the blur rect — confirm the Transformer handles appear.
3. Drag the rect to a new position — confirm it moves and the pixelation updates to sample the NEW underlying region.
4. Drag a corner handle to resize — release — confirm the pixelation snaps to the new bounds.
5. Press Delete — confirm the shape is removed.

**Step 4: Test the placeholder during draw**

1. Pick the Blur tool again.
2. Start a drag and **without releasing**, observe the in-progress shape.
3. Confirm it's a dashed blue outline with a faint blue fill — NOT the final pixelation.
4. Release — confirm the placeholder is replaced by the real pixelation.

**Step 5: Test multiple blur shapes**

1. Place a second blur rect on a different region.
2. Confirm both shapes render their own pixelation independently (different colours, sampled from different regions).
3. Drag one — the other stays put.

**Step 6: Test annotation overlay rendering on top**

1. Pick the Arrow tool.
2. Draw an arrow that crosses a blur rect.
3. Confirm the arrow renders on top of the pixelation, fully visible.

**Step 7: Test round-trip with crop**

1. Pick the Crop tool.
2. Draw a crop region that intersects a blur rect (the crop region should partially cover the blur shape).
3. Click "Apply ✓" on the floating button.
4. Click Save.
5. Open the saved annotated PNG — confirm the cropped output contains the pixelated patch and the crop bounds are respected.

**Step 8: Stop the dev server**

If everything in Steps 2–7 passes, the implementation is done. If anything fails, capture the symptom and either patch in a follow-up commit or revert.

---

## Acceptance criteria

The plan is complete when:

- All 4 implementation commits are on the `feature/blur-pixelation` branch.
- `pnpm --filter @snk/app test` passes (126 tests).
- `pnpm --filter @snk/app typecheck` passes.
- `pnpm --filter @snk/app lint` passes.
- `pnpm --filter @snk/app build` passes.
- The full manual test plan from Task 5 passes end-to-end without follow-up fixes.

Then hand off to `h-superpowers:finishing-a-development-branch` to merge into `main`.

---

## Coverage exclusions reminder

`app/vitest.config.ts` excludes `src/windows/annotate/shapes/**` from coverage. Task 4 touches one of those files and adds no automated tests — that's by design and matches the existing pattern (`RectangleShape`, `ArrowShape`, etc. have no unit tests either). Task 1 is the only task that ships test code; the other tasks rely on the manual checks in Task 5 and the existing test suite passing.
