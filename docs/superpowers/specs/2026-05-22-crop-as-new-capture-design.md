# Crop Save Creates a New Capture

## Problem

The just-shipped annotation-state work persists shapes + crop on the same capture row. That means a capture can have ONE crop saved alongside it. Users want to make MULTIPLE cropped versions of a single original — e.g. crop the top half, save; crop the bottom half, save again as a separate gallery entry. The current "one annotated derivative per row" model can't express that.

## Goal

When the user saves with a confirmed crop region, the result becomes a new gallery entry (a brand-new capture row + file), leaving the original untouched. Multiple crop-derivatives from the same original are first-class captures that can be searched, tagged, pinned, and annotated independently.

## Non-goals

- **Tracking parent/child lineage.** Derivations are first-class captures with no `parent_id` column. Lose lineage info; gain simplicity. Revisit only if a "show parent" or "siblings" feature is requested.
- **Identifier suffixes in filenames.** No `.cropped.png` / `.annotated.png` distinction. Each new capture uses the standard `captures/YYYY/MM/<uuid>.png` layout.
- **Replacing the annotated-only save flow.** Saving annotations without a crop continues to write to `annotated_path` on the same row and update `annotation_state` in place, exactly as today.
- **Inheriting tags or pinned status.** Derivations start untagged and unpinned. User manages independently.
- **Migrating existing rows.** Anything saved under the just-shipped `annotation_state` model stays valid. The new code only branches when a NEW save happens with a confirmed crop.

## Behaviour summary

| User action | Result |
|---|---|
| Open capture, draw shapes, no crop, Save | Updates `annotated_path` + `annotation_state` on the current row (unchanged) |
| Open capture, draw a confirmed crop (with or without shapes), Save | Creates a new capture row + new PNG file. The original is untouched. Editor state stays where it was (crop still visible) — `isDirty` clears via `markClean()`. |
| Open a derivation (which is just another capture), draw shapes, no NEW crop, Save | Updates the derivation's `annotated_path` + `annotation_state` — same as the original case |
| Open a derivation, draw a NEW crop, Save | Creates another derivation (a "crop of a crop") |

The editor doesn't auto-reset after a crop-save. The user sees what they just saved and can keep editing → another save = another derivation. Done? → close the annotator normally.

## Architecture

### Save flow branching

`AnnotateTopBar.handleSave` decides which Tauri command to invoke based on whether the store has a confirmed crop:

```ts
const hasCrop = store.cropRegion !== null && store.cropConfirmed;
if (hasCrop) {
  const derived = await deriveCapture(captureId, png, cw, ch, shiftedState);
  // editor state stays as-is; markClean clears the dirty flag
} else {
  await saveAnnotation(captureId, png, state); // existing path
}
useAnnotateStore.getState().markClean();
```

The shifted state is the same `AnnotationState` shape but with shape coordinates moved relative to the new image's origin:

```ts
const shiftedState: AnnotationState = {
  version: 1,
  shapes: store.shapes.map(shiftShape(-crop.x, -crop.y)),
  crop_region: null,       // crop is now baked into the file
  crop_confirmed: false,
};
```

`shiftShape` is a pure helper that updates `x`/`y` for box-positioned shapes (`rectangle`, `ellipse`, `blur`, `text`, `step-marker`) and `points[]` for path-positioned shapes (`arrow`, `pen`, `highlighter`).

### New Tauri command: `derive_capture`

Lives in `crates/snk-annotate/src/commands.rs` alongside `save_annotation`. Signature:

```rust
#[tauri::command]
pub fn derive_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    parent_id: String,
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    state_json: String,
) -> Result<Capture> {
    let parent = captures::get(&state.db, &parent_id)?;

    // Pre-generate the uuid so the on-disk filename matches the DB row.
    let new_id = uuid::Uuid::now_v7();
    let relative = files::capture_relative_path(&new_id, "png");
    files::write_atomic(&state.root, &relative, &png_data)?;

    // Insert with the pre-chosen id, inheriting source metadata from
    // the parent so the new capture's "where it came from" is honest.
    let new_capture = captures::insert_with_id(&state.db, new_id, NewCapture {
        file_path: relative.clone(),
        width, height,
        source_app: parent.source_app.clone(),
        source_window_title: parent.source_window_title.clone(),
        monitor: parent.monitor.clone(),
    })?;

    // Persist the shifted editable state on the new row.
    captures::set_annotation_state(&state.db, &new_capture.id, &state_json)?;

    // Emit capture:saved so the OCR pipeline runs on the derivation,
    // mirroring how snk-capture announces a fresh capture.
    let _ = app.emit("capture:saved", &new_capture.id);

    captures::get(&state.db, &new_capture.id).map_err(Into::into)
}
```

### `insert_with_id` library helper

The existing `captures::insert` generates its own UUID. We need to write the PNG file before knowing the uuid we'll INSERT, OR we need an `insert` variant that accepts a pre-chosen uuid. The latter is cleaner.

```rust
pub fn insert_with_id(db: &Db, id: Uuid, new: NewCapture) -> Result<Capture> {
    // Same body as insert, but takes the uuid as a parameter instead of
    // calling Uuid::now_v7() inline. insert() becomes a thin wrapper:
    //     pub fn insert(db: &Db, new: NewCapture) -> Result<Capture> {
    //         insert_with_id(db, Uuid::now_v7(), new)
    //     }
}
```

This refactor is small and backward-compatible (no calling sites change). Two new unit tests:

- `insert_with_id_uses_the_provided_uuid` — pre-generate a uuid, insert with it, assert the returned capture's id matches.
- `insert_with_id_does_not_collide_with_default_insert` — call both back-to-back, assert ids differ.

### TS binding

```ts
export function deriveCapture(
  parentId: string,
  pngData: number[],
  width: number,
  height: number,
  state: AnnotationState,
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-annotate|derive_capture', {
    parentId,
    pngData,
    width,
    height,
    stateJson: JSON.stringify(state),
  });
}
```

`AnnotationState` is unchanged. `saveAnnotation` is unchanged.

### Shape coordinate shift helper

A new pure function in `app/src/windows/annotate/cropDerivation.ts` (or wherever fits — a small new file is cleanest):

```ts
export function shiftShapesForCrop(
  shapes: AnnotationShape[],
  crop: { x: number; y: number },
): AnnotationShape[] {
  return shapes.map((s) => {
    const dx = -crop.x;
    const dy = -crop.y;
    if (s.points) {
      // arrow, pen, highlighter: [x0, y0, x1, y1, ...]
      const shifted = s.points.map((v, i) => v + (i % 2 === 0 ? dx : dy));
      return { ...s, points: shifted };
    }
    return { ...s, x: (s.x ?? 0) + dx, y: (s.y ?? 0) + dy };
  });
}
```

Unit-testable. Add three tests:

- `shiftShapesForCrop translates rectangle x/y by the negative crop offset`
- `shiftShapesForCrop translates arrow points pairwise`
- `shiftShapesForCrop preserves stroke and other fields`

### OCR for derivations

The Rust `derive_capture` command emits `capture:saved` with the new id. `snk-ocr/src/plugin.rs` already listens for this event and runs OCR async on the new capture's file. No changes to `snk-ocr` are needed — derivations automatically get OCR text on the same pipeline as fresh captures.

This means derivations will be findable in search by their on-image text — same as originals. OCR runs on the cropped+annotated PNG content, so it reflects what's actually visible in that derivation.

### Editor state after a crop-save

The editor stays where it is. The crop + shapes you just saved remain on the canvas, but `isDirty` flips to false because the state has been persisted (as a separate derivation). The user can:

- **Keep editing** → another Save produces another derivation. The first one stays in the gallery.
- **Close** → the dirty-guard doesn't fire (clean state). The editor closes normally.
- **Use Ctrl+Z** → undoes back through the in-memory history, which includes the in-progress edits but not the save-as-derivation itself (save isn't an undoable action — same as today).

### What happens to the original's `annotation_state`?

Unchanged. The original's stored state (whatever it was before this save) stays exactly as it was. The save operation only writes to the NEW row, not the parent.

If the user wants to also save the current canvas state ONTO the original (e.g. they had drawn shapes meant to be part of the original's saved layer, not just the crop), they'd need to do a separate annotated-only save (i.e. remove the crop and Save again). That's an edge case; the simple rule "crop save = derivation, no-crop save = update original" is more predictable than trying to do both.

## Trade-offs flagged

| Decision | Trade-off |
|---|---|
| No `parent_id` column | Lose lineage info: no UI for "show parent" or "find siblings". Easy to add later if requested. |
| Editor stays put after crop-save | Slight surprise that the just-saved derivation isn't visually highlighted, but avoids navigation churn. User explicitly opens it from the gallery if they want to keep editing it. |
| Inherit source metadata, fresh tags/pinned | Source attribution is honest ("this came from app X at time Y"). Tags/pinned start fresh so derivations don't auto-clutter pinned views. |
| `created_at` is save time, not parent time | Derivation appears at the top of the gallery (recent). Different timeline than the parent's. Easy to reason about. |
| OCR runs on derivations via `capture:saved` | More OCR work, but tiny per-derivation cost. Search consistency wins. |

## Test plan

### Rust unit tests

1. `insert_with_id_uses_the_provided_uuid` — explicit id passes through.
2. `insert_with_id_does_not_collide_with_default_insert` — different ids round-trip.
3. (No new tests on the derive_capture command — it's Tauri integration glue, like save_annotation.)

### TS unit tests

1. `shiftShapesForCrop translates rectangle x/y by the negative crop offset`.
2. `shiftShapesForCrop translates arrow points pairwise`.
3. `shiftShapesForCrop preserves stroke + other fields`.
4. `deriveCapture binding forwards parentId, png, dimensions, and serialized state` (mirrors the existing `saveAnnotation` test).

### Manual smoke

1. **Crop creates a new gallery entry.** Open original → draw a crop → Save → confirm "saved ✓" flash. Close annotator. Gallery shows TWO entries: original + cropped derivation. Open the derivation: its source image is the cropped PNG, no crop region overlay.
2. **Multiple crops from one original.** Open the original again, draw a DIFFERENT crop, Save. Gallery now has three entries (original + crop A + crop B). Both crops sample different regions of the original.
3. **Crop + annotation in one save.** Open original, draw a crop + an arrow over the crop region, Save. Open the derivation: the cropped PNG with the arrow baked in displays; `annotation_state` has the arrow at the SHIFTED coordinates, so the canvas shows the arrow positioned correctly.
4. **Re-edit a derivation.** Open the cropped derivation. Add a rectangle. Save (no new crop). It saves in-place (no new gallery entry). Re-open: original arrow + new rectangle both load.
5. **Crop a crop.** Open the derivation. Draw another crop. Save. A second-level derivation appears.
6. **OCR on derivation.** Crop a region with readable text. After a few seconds, search for a word from the cropped text → the derivation comes up in results.
7. **Annotated-only save still works.** Open an original, draw an arrow, no crop, Save. The original's `annotated_path` updates as before; no new gallery entry.
8. **Originals untouched.** Open the original after step 1. Confirm no annotated overlay loads (it's the bare original screenshot). Confirm no crop region. Confirm `annotation_state` on this row is whatever it was BEFORE the crop-save.

## Files touched

- `crates/snk-library/src/captures.rs` — factor `insert` to delegate to a new `insert_with_id(db, uuid, NewCapture)`; add unit tests.
- `crates/snk-annotate/src/commands.rs` — add `derive_capture` command alongside `save_annotation`.
- `crates/snk-annotate/src/plugin.rs` — register the new command in `tauri::generate_handler!`.
- `packages/snk-annotate/src/index.ts` — export `deriveCapture` binding.
- `packages/snk-annotate/src/index.test.ts` — add a forwarding test for `deriveCapture`.
- `app/src/windows/annotate/cropDerivation.ts` — new pure helper `shiftShapesForCrop`.
- `app/src/windows/annotate/cropDerivation.test.ts` — new unit tests.
- `app/src/windows/annotate/AnnotateTopBar.tsx` — `handleSave` branches on `cropConfirmed`.
