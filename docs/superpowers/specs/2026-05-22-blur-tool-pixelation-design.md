# Real Pixelation for the Annotate Blur Tool

## Problem

The current `BlurShape` is not a blur at all — it renders a 12-pixel translucent grey checkerboard at `rgba(_,_,_,0.85)` on top of the screenshot. Since the overlay is only 85% opaque, the underlying 15% of pixels still shows through, and the pattern is generic (it never samples the screenshot content). At any zoom level the original text remains readable. Users reaching for this tool expect redaction; today they're getting visual decoration.

## Goal

Make the blur tool actually obscure underlying content so that text under the rectangle is unrecoverable at any zoom level, without changing how it's used.

## Non-goals

- **Adjustable block size.** One sensible default; no slider, no dropdown, no preset reuse.
- **Alternative blur styles** (Gaussian smear, solid black redaction). Pixelation only for v1.
- **Pixelating annotation layers drawn on top of the screenshot.** The blur samples only the source screenshot. If a user draws an arrow over a blur, the arrow stays visible above it. Matches the mental model of "redact screen content."
- **Cryptographic-grade redaction.** Pixelation is theoretically reversible by experts in pathological cases. For a screenshot-annotation tool aimed at casual sharing this is acceptable; users wanting absolute redaction can stack a Rectangle filled with the theme's danger color on top.

## User-visible behaviour

Unchanged from today:

1. Pick the **Blur** tool from the toolbar.
2. Drag corner-to-corner on the canvas. A rectangle appears.
3. On release, the rectangle becomes a pixelated patch — 20-pixel opaque blocks sampled from the screenshot underneath.
4. Switch to the **Select** tool to drag, resize via the transformer handles, or delete it. Behaviour is identical to a `RectangleShape`.

There are no new toolbar buttons, no mode switches, no settings entries. The existing tool button just produces a different visual on release.

## Architecture

Render via Konva's built-in `Konva.Filters.Pixelate`. Konva filters require a cached node; we cache once per render and re-cache whenever the shape's geometry changes.

### Data flow

```
AnnotateCanvas mounts
  → loads the screenshot into a new Image()
  → stores it in the annotate store as sourceImage
BlurShape renders
  → reads sourceImage from the store
  → renders a Konva.Image with crop = {x, y, w, h} matching the shape
  → applies the Pixelate filter at pixelSize 20
  → calls node.cache() in a useEffect keyed on x/y/w/h
```

### Annotate store

Add a single field:

```ts
interface AnnotateState {
  // existing fields...
  sourceImage: HTMLImageElement | null;
  setSourceImage: (img: HTMLImageElement | null) => void;
}
```

`reset()` clears it back to `null` so the next capture loads cleanly.

### AnnotateCanvas

The canvas already loads the screenshot in a `useEffect`. Push the loaded `HTMLImageElement` into the store so children can read it. The `<KonvaImage>` it renders today for the visible screenshot continues to use the local state — the store copy is only for `BlurShape` to consume.

### BlurShape rewrite

Drop the Group + checkerboard rectangles entirely. The shape becomes:

```tsx
export function BlurShape({ shape, draggable, onSelect, registerNode }: ShapeProps) {
  const ref = useRef<Konva.Image | null>(null);
  const sourceImage = useAnnotateStore((s) => s.sourceImage);
  const updateShape = useAnnotateStore((s) => s.updateShape);

  const x = shape.x ?? 0;
  const y = shape.y ?? 0;
  const w = shape.width ?? 0;
  const h = shape.height ?? 0;

  // Re-cache whenever the source or geometry changes so the filter re-runs
  // against the right region.
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
      crop={{ x, y, width: w, height: h }}
      filters={[Konva.Filters.Pixelate]}
      pixelSize={20}
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

Two subtleties:

- **`image={sourceImage ?? undefined}`** — Konva accepts `undefined` and renders nothing during the brief moment between mount and image load. Once the store fills in the source the node re-renders and the cache useEffect fires.
- **`crop` matches `x/y/w/h` exactly**. The node's own `x/y` already position it in the layer, and `crop` tells Konva which source-image region to draw. The two coordinate systems happen to be the same — the screenshot fills the layer at (0,0) at native pixel size — so the shape's rect doubles as the crop region.

### Constants

A single named constant in the shape file:

```ts
const PIXELATE_BLOCK_SIZE = 20;
```

### During the draw gesture

`currentShape` is the in-progress draft that's set on every `mousemove` while the user is dragging a new shape. If `BlurShape` rendered the pixelation filter for the draft too, every mousemove would re-cache and re-filter — fine for small rects, but a slow lag for an accidental full-screen drag (cache cost scales with area).

`AnnotateCanvas` renders the currentShape with a cheaper placeholder: a Rectangle with the same blue dashed outline + 15%-blue fill that the crop tool uses for its draft state. The real `BlurShape` only renders for committed shapes in the `shapes` array. Visually this matches the existing "I'm drawing a region" feedback users already see for crop.

```tsx
{currentShape && currentShape.tool === 'blur' ? (
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

## Trade-offs flagged

| Decision | Trade-off |
|----------|-----------|
| Sample only the screenshot, not the live stage | Annotations drawn on top of a blur remain visible (intentional — matches the redact-screen-content mental model). |
| One cache per BlurShape | Memory ~ blur area × 4 bytes. For typical 1–5 small rects this is negligible (~tens of KB). |
| Re-cache on every geometry change | Cache is fast for small regions (<2ms for ~200×100). No perf concern. |
| Fixed 20px blocks | Largest size we can pick before chunkiness feels excessive; small enough to feel like a redaction box rather than a giant blob. macOS uses ~16–20px for the same tool. |

## Export

When the user clicks **Save** in the top bar, `AnnotateTopBar` calls `stage.toDataURL({ pixelRatio: 1 / stage.scaleX() })`. Konva's `toDataURL` serializes cached filter output as part of the rendered stage, so the exported PNG contains the pixelated patches, not the source pixels. No changes needed to the export path.

## Test plan

1. **Manual: redaction is real.** Draw a blur rect over readable text → save → open the exported PNG → zoom to 400% in Photos / Preview → confirm individual characters are unrecoverable.
2. **Manual: behaves like a rectangle.** Pick Select → click a blur shape → confirm transformer handles appear, drag to move, drag a handle to resize, press Delete to remove. All identical to a Rectangle.
3. **Manual: multiple blurs.** Place two blur rects in different regions → confirm each samples its own region and resizes independently.
4. **Manual: annotation overlay.** Draw an arrow that crosses a blur rect → confirm the arrow renders on top, undimmed.
5. **Manual: round-trip with crop.** Apply a confirmed crop region that intersects a blur rect → save → confirm the cropped PNG contains the pixelated patch and the crop bounds are respected.

No unit tests change — `BlurShape` continues to register a node via `registerNode` (covered indirectly by canvas tests) and `useAnnotateStore.deleteShape`, `updateShape`, etc. tests already cover the data layer. Konva's filter pipeline is the library's concern, not ours.

## Migration

Existing blur shapes are stored in the database as `tool: 'blur'` with `x/y/width/height`. The new renderer reads exactly the same shape fields, so reloading any previously annotated capture will repaint old blur rects with real pixelation. No data migration required.

## Files touched

- `app/src/windows/annotate/store.ts` — add `sourceImage` field and setter; clear in `reset()`.
- `app/src/windows/annotate/AnnotateCanvas.tsx` — set `sourceImage` on the store when the screenshot finishes loading.
- `app/src/windows/annotate/shapes/BlurShape.tsx` — full rewrite (Group → KonvaImage with crop + Pixelate filter + cache).
