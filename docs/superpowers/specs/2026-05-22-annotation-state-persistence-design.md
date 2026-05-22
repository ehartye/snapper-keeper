# Annotation State Persistence

## Problem

Today the annotator persists only the rendered PNG (`captures.annotated_path`). The editable state — shapes array, crop region, crop confirmation — is reset when the annotate window closes and lost forever. A user who comes back to a screenshot to extend or tweak their edits has to start from scratch, including re-drawing the exact crop rectangle they used last time.

## Goal

Re-opening a previously-edited capture restores the annotator to exactly the state the user last saved: same shapes, same crop region, same confirmation. New captures or legacy ones that were edited before this feature shipped open clean, as today.

## Non-goals

- **Auto-save on every edit.** Save is still triggered explicitly by the Save button.
- **Versioned history per capture.** Only the most recent saved state is kept. No "view past edits" feature.
- **Editing the rendered `.annotated.png` directly.** That file remains the immutable bitmap output; the editable state is parallel data.

## Storage

One new column on the `captures` table:

```sql
ALTER TABLE captures ADD COLUMN annotation_state TEXT;
```

`NULL` means "never annotated, or annotated before this feature shipped — open clean." A non-null value is a JSON document with the schema below. The column lives on the row so it's transactional with `annotated_path` and queryable later if we want (e.g. a "shapes containing OCR text X" search).

### JSON shape

```ts
type AnnotationState = {
  version: 1;
  shapes: AnnotationShape[];      // exactly what's in store.shapes
  crop_region: CropRegion | null; // exactly what's in store.cropRegion
  crop_confirmed: boolean;
};
```

The Rust mirror lives in `snk-library`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationState {
    pub version: u32,
    pub shapes: serde_json::Value, // opaque pass-through — Rust never inspects
    pub crop_region: Option<CropRegion>,
    pub crop_confirmed: bool,
}
```

`shapes` is an opaque `serde_json::Value` from Rust's perspective. The frontend produces it and consumes it; Rust just transports + stores. This keeps the shape schema (which lives in TypeScript) from leaking into the Rust crate as a duplicate definition.

The `version: 1` field is a forward-compatibility hatch. If we ever change the shape schema in a breaking way, the loader can branch on `version` and either migrate or open clean.

## Migration

Adds a single migration file:

```
crates/snk-library/migrations/<n>_annotation_state.sql
ALTER TABLE captures ADD COLUMN annotation_state TEXT;
```

Existing rows get NULL. No data migration; the column is purely additive.

## API changes

### Rust — `snk-library`

Add to `crates/snk-library/src/captures.rs`:

```rust
pub fn set_annotation_state(db: &Db, id: &str, state_json: &str) -> Result<()> {
    // UPDATE captures SET annotation_state = ?1 WHERE id = ?2 AND deleted_at IS NULL
}
```

Update the existing `Capture` struct (returned by `get`, `list`) so the JSON column rides along:

```rust
pub struct Capture {
    // ...existing fields...
    pub annotated_path: Option<String>,
    pub annotation_state: Option<String>, // raw JSON, not yet parsed
}
```

The list/get query selects the new column. Frontend parses lazily when it actually opens the annotator.

### Rust — `snk-annotate`

Change `save_annotation` to take the state JSON alongside the PNG:

```rust
#[tauri::command]
pub fn save_annotation<R: Runtime>(
    capture_id: String,
    png_data: Vec<u8>,
    state_json: String,
    state: tauri::State<'_, SnkState>,
    app: tauri::AppHandle<R>,
) -> Result<Capture, AnnotateError> {
    // 1. Write the PNG to disk (existing)
    // 2. Update captures.annotated_path (existing)
    // 3. Update captures.annotation_state with state_json (NEW)
    // Returns the updated Capture row.
}
```

Both writes happen in the same transaction inside snk-library so they're atomic — either the row gets both updates or neither.

### TS — `@snk/annotate`

```ts
export function saveAnnotation(
  captureId: string,
  pngData: number[],
  state: AnnotationState,
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-annotate|save_annotation', {
    captureId,
    pngData,
    stateJson: JSON.stringify(state),
  });
}
```

`Capture` already comes back from `@snk/library`; the new `annotation_state: string | null` field flows along on the type without a new query.

## Frontend behaviour

### Loading

`AnnotateWindow` (or wherever the capture is fetched today) reads `capture.annotation_state`. If non-null, JSON-parse and push into the store **before** `AnnotateCanvas` mounts:

```ts
useEffect(() => {
  if (!capture) return;
  if (capture.annotation_state) {
    try {
      const parsed = JSON.parse(capture.annotation_state) as AnnotationState;
      if (parsed.version === 1) {
        useAnnotateStore.setState({
          shapes: parsed.shapes,
          cropRegion: parsed.crop_region,
          cropConfirmed: parsed.crop_confirmed,
          undoStack: [],
          redoStack: [],
        });
      }
    } catch {
      // Malformed JSON — open clean rather than crash.
    }
  }
}, [capture]);
```

Undo/redo stacks intentionally start empty after a load. The user starts fresh history from this load point; we don't try to deserialize the full undo timeline.

### Saving

`AnnotateTopBar.handleSave` serializes the store before invoking the plugin:

```ts
const state: AnnotationState = {
  version: 1,
  shapes: store.shapes,
  crop_region: store.cropRegion,
  crop_confirmed: store.cropConfirmed,
};
await saveAnnotation(captureId, png, state);
// Mark clean on success
useAnnotateStore.getState().markClean();
```

### Dirty tracking

Add to the store:

```ts
interface AnnotateState {
  // ...existing...
  isDirty: boolean;
  markClean: () => void;
}
```

`addShape`, `updateShape`, `deleteShape`, `setCropRegion`, `confirmCrop`, `undo`, and `redo` all set `isDirty: true`. `markClean()` (called from `handleSave` on success and from the load effect on a clean restore) sets it back to `false`. `reset()` sets it to `false` too.

### Close confirmation

Three exit paths need to check `isDirty`:

1. **The "done" button** in `AnnotateTopBar.handleDone`.
2. **The window's X / "hide to tray" intercept** in `AnnotateWindow`'s `onCloseRequested`.
3. **Esc with no shape selected** in `AnnotateCanvas.handleKeyDown`.

All three call a shared helper:

```ts
async function confirmDiscardIfDirty(): Promise<boolean> {
  const { isDirty } = useAnnotateStore.getState();
  if (!isDirty) return true;
  // Native confirm() works in Tauri webview; matches the project's choice
  // of not adding the plugin-dialog dependency for one-off prompts.
  return window.confirm(
    'You have unsaved annotation changes. Discard them?'
  );
}
```

If the helper returns `false`, the exit is cancelled (e.g. `event.preventDefault()` on `onCloseRequested`). If `true`, the exit proceeds and `reset()` runs as today.

The keyboard handler in `AnnotateCanvas` becomes:

```ts
if (e.key === 'Escape') {
  if (selectedId) { setSelectedId(null); return; }
  void (async () => {
    if (await confirmDiscardIfDirty()) {
      useAnnotateStore.getState().reset();
      getCurrentWindow().hide();
    }
  })();
  return;
}
```

## Trade-offs flagged

| Decision | Trade-off |
|----------|-----------|
| One JSON column, not a separate table | 1:1 with capture row — simpler. Loses the ability to query inside the JSON without `json_extract`. Acceptable; we don't need that today. |
| Rust treats `shapes` as opaque `serde_json::Value` | Schema lives only in TS; no duplication. Cost: Rust can't validate shape integrity at write time. Acceptable; the frontend is the only writer. |
| Undo stack does NOT survive across loads | Simpler implementation and matches what most editors do. A re-load is a fresh edit session from the user's POV. |
| Native `window.confirm()` not the Tauri dialog plugin | One fewer dependency. Cost: the prompt looks browser-y, not OS-native. Acceptable for a side project; can swap to `@tauri-apps/plugin-dialog` later if it grates. |

## Test plan

### Rust unit tests (new, in `crates/snk-library/src/captures.rs`)

1. `set_annotation_state_updates_column` — writes JSON, reads back from `get`, asserts equal.
2. `set_annotation_state_returns_not_found_for_soft_deleted` — soft-delete a row, attempt set, get `NotFound`.
3. `get_returns_null_annotation_state_when_never_set` — fresh capture, `annotation_state` is None.

### TS store tests (new, in `app/src/windows/annotate/store.test.ts`)

1. `isDirty starts false` — fresh store.
2. `addShape sets isDirty` — also covers updateShape, deleteShape.
3. `setCropRegion sets isDirty`.
4. `confirmCrop sets isDirty`.
5. `markClean clears isDirty without touching other state` — preserves shapes, crop, undo/redo.
6. `reset clears isDirty` — covered by existing reset test, just extend the assertion.

### Manual

1. Capture a screenshot, open annotator, draw shapes + a confirmed crop, click Save → click X. Reopen the capture → shapes and crop are restored.
2. Same as above but Ctrl+Z several times before Save → undo state isn't restored, but the saved state is.
3. Draw a shape, don't save, click X → confirmation dialog. Cancel → window stays. Accept → window closes; reopen → no shape (only the previous save).
4. Open a never-annotated capture → clean state, no shapes, no crop.
5. Reopen an annotated capture that's from before this feature shipped (i.e. `annotated_path` set, `annotation_state` NULL) → clean state, no errors. (Verified by manually NULL-ing the column in sqlite for an existing row.)
6. Edit and save twice in a row → second open shows the second save's state, not a stale one.

## Files touched

- `crates/snk-library/migrations/<next>_annotation_state.sql` — new migration.
- `crates/snk-library/src/captures.rs` — extend `Capture` struct, update SELECT queries, add `set_annotation_state`, add 3 unit tests.
- `crates/snk-annotate/src/commands.rs` — extend `save_annotation` to also write the state JSON.
- `packages/snk-library/src/types.ts` — add `annotation_state: string | null` to the `Capture` TS type.
- `packages/snk-annotate/src/index.ts` — change `saveAnnotation` signature.
- `packages/snk-annotate/src/index.test.ts` — update mock invocation expectations.
- `app/src/windows/annotate/store.ts` — add `isDirty` field, `markClean` setter, wire all mutators to flip it, extend `reset` to clear it.
- `app/src/windows/annotate/store.test.ts` — add `isDirty` tests.
- `app/src/windows/annotate/AnnotateWindow.tsx` — load state on capture fetch; wire close intercept to dirty check.
- `app/src/windows/annotate/AnnotateCanvas.tsx` — Esc handler uses the dirty check.
- `app/src/windows/annotate/AnnotateTopBar.tsx` — `handleSave` serializes state and calls `markClean()`; `handleDone` uses dirty check.
- Shared helper file (small) for `confirmDiscardIfDirty` — likely `app/src/windows/annotate/dirty-guard.ts`.
