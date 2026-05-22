# Annotation State Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Re-opening an edited capture restores the annotator to the exact shapes + crop the user last saved.

**Architecture:** Adds an `annotation_state TEXT` column on `captures` (NULL = legacy/unedited). `save_annotation` writes both the rendered PNG and a JSON snapshot of `{ version: 1, shapes, crop_region, crop_confirmed }` in one atomic Rust-side update. `AnnotateWindow` parses the JSON on capture open and pushes it into the store before the canvas mounts. Save remains the only persistence trigger; a new `isDirty` flag gates the three exit paths (done button / X / Esc) behind `window.confirm()`.

**Tech Stack:** Rust + rusqlite + rusqlite_migration for the schema and command; React + Zustand for the store + UI; Vitest + happy-dom for TS tests; Rust integration-style tests via the existing `crates/snk-library` test harness. No new dependencies.

---

## Background context the engineer needs

- The annotate window has its own Zustand store at `app/src/windows/annotate/store.ts`. Crop draw/apply/cancel are already undoable (recent change). The store currently has fields: `shapes`, `cropRegion`, `cropConfirmed`, plus undo/redo stacks of `HistoryEntry`. You're adding **one** more field: `isDirty`.
- Captures are persisted via the `snk-library` crate (`crates/snk-library/src/captures.rs`). The `Capture` struct already has `annotated_path: Option<String>` set by `set_annotated_path`. You're adding `annotation_state: Option<String>` in the same shape — a raw JSON string the frontend produces and consumes.
- The annotate save flow today: `AnnotateTopBar.handleSave` exports a PNG → calls `saveAnnotation(captureId, png)` → `crates/snk-annotate/src/commands.rs` writes the PNG to disk and updates `annotated_path`. You're extending the JS binding + Rust command to also accept and persist a state JSON in the same atomic library call.
- Migrations live in `crates/snk-library/migrations/V00X__name.sql`. They're embedded via `include_str!` in `crates/snk-library/src/migrate.rs`. Adding a migration means adding the `.sql` file AND registering it in `migrate.rs`. The current latest is `V003__ocr_fts.sql`; yours is `V004`.
- `SELECT *` is used in `get` and `list` — you don't have to touch SQL strings. The only Rust mechanical change is `row_to_capture` plus the struct.
- `AnnotateCanvas.tsx`, `BlurShape.tsx`, etc. are all in Vitest's coverage exclude list (Konva-bound, happy-dom has no canvas). Tasks that touch only those files verify with `pnpm typecheck` + `pnpm lint` + manual smoke. Tests are added where the code is testable (the store, the Rust crate, the TS binding).
- Spec: `docs/superpowers/specs/2026-05-22-annotation-state-persistence-design.md`.

## Worktree convention

Work directly on `main` if the user requests, otherwise create a sibling worktree `C:/Users/ehart/repos/snapper-keeper-worktrees/feature/annotation-state-persistence/` per project convention (h-superpowers:using-git-worktrees).

---

## Task 1: Add the migration + extend the `Capture` struct

**Files:**
- Create: `crates/snk-library/migrations/V004__annotation_state.sql`
- Modify: `crates/snk-library/src/migrate.rs`
- Modify: `crates/snk-library/src/captures.rs`

**Step 1: Write the failing test**

In `crates/snk-library/src/captures.rs`, in the existing `#[cfg(test)] mod tests` block, add:

```rust
    #[test]
    fn new_capture_has_null_annotation_state() {
        let db = test_db();
        let c = insert(&db, new_capture("a.png", 100, 100)).unwrap();
        let fetched = get(&db, &c.id).unwrap();
        assert!(fetched.annotation_state.is_none());
    }
```

Use the same helper functions the existing tests use (`test_db()`, `new_capture(...)`); they're already in scope.

**Step 2: Run to verify it fails**

```
cargo test -p snk-library new_capture_has_null_annotation_state
```

Expected: FAIL — `no field annotation_state on Capture`.

**Step 3: Create the SQL migration**

Create `crates/snk-library/migrations/V004__annotation_state.sql`:

```sql
-- Phase 8 — annotation editor state.
-- Stores the editable annotation state (shapes + crop) as JSON so users
-- can re-open a previously-edited capture and continue editing rather
-- than starting from scratch. NULL means the capture has never been
-- annotated under this system (legacy rows + brand-new captures).

ALTER TABLE captures ADD COLUMN annotation_state TEXT;
```

**Step 4: Register the migration**

Edit `crates/snk-library/src/migrate.rs`. Replace the const + `migrations()` function:

```rust
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");
const V003: &str = include_str!("../migrations/V003__ocr_fts.sql");
const V004: &str = include_str!("../migrations/V004__annotation_state.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V001), M::up(V002), M::up(V003), M::up(V004)])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 4,
            recoverable: e.to_string().contains("Backup"),
        })?;
    Ok(())
}
```

(The `to: 3` → `to: 4` change matters for error reporting consistency.)

**Step 5: Extend the `Capture` struct**

In `crates/snk-library/src/captures.rs`, modify the `Capture` struct (around line 8). Add `annotation_state` right after `annotated_path`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capture {
    pub id: String,
    pub file_path: String,
    pub annotated_path: Option<String>,
    pub annotation_state: Option<String>,
    pub width: u32,
    pub height: u32,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
    pub monitor: Option<String>,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
    pub pinned: bool,
}
```

**Step 6: Update `row_to_capture`**

Same file, find the `row_to_capture` helper (around line 89). Add the new line right after `annotated_path`:

```rust
fn row_to_capture(row: &rusqlite::Row<'_>) -> rusqlite::Result<Capture> {
    Ok(Capture {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        annotated_path: row.get("annotated_path")?,
        annotation_state: row.get("annotation_state")?,
        width: row.get::<_, i64>("width")? as u32,
        height: row.get::<_, i64>("height")? as u32,
        source_app: row.get("source_app")?,
        source_window_title: row.get("source_window_title")?,
        monitor: row.get("monitor")?,
        created_at: row.get("created_at")?,
        deleted_at: row.get("deleted_at")?,
        pinned: row.get("pinned")?,
    })
}
```

**Step 7: Update the `insert` function's returned struct**

Same file, find `pub fn insert` (around line 33). The final `Ok(Capture { ... })` block needs the new field. Add `annotation_state: None` right after `annotated_path: None`:

```rust
    Ok(Capture {
        id,
        file_path,
        annotated_path: None,
        annotation_state: None,
        width: new.width,
        height: new.height,
        source_app: new.source_app,
        source_window_title: new.source_window_title,
        monitor: new.monitor,
        created_at,
        deleted_at: None,
        pinned: false,
    })
```

**Step 8: Verify the test passes**

```
cargo test -p snk-library new_capture_has_null_annotation_state
```

Expected: PASS.

**Step 9: Verify the full library test suite passes**

```
cargo test -p snk-library
```

Expected: every existing test still passes (the new column is additive, `SELECT *` picks it up automatically).

**Step 10: Verify the workspace still builds**

```
cargo build -p snapper-keeper-app
```

Expected: success. (The app references the `Capture` struct through `snk-library`; adding a field is backward-compatible at the Rust level — but downstream `Capture` field-init expressions might exist that now miss the new field. There are some; see Task 4. For now `cargo build` should succeed because we haven't broken any field-init patterns inside the crate itself.)

**Step 11: Commit**

```
git add crates/snk-library/migrations/V004__annotation_state.sql \
        crates/snk-library/src/migrate.rs \
        crates/snk-library/src/captures.rs
git commit -m "feat(library): add annotation_state column to captures

Stores the editable annotation state (shapes + crop region) as JSON
so users can re-open a previously-edited capture and pick up where
they left off rather than redrawing.

NULL means the capture has never been annotated under this system
(legacy rows + brand-new captures). The Capture struct gains an
annotation_state: Option<String> field that row_to_capture reads
back from SELECT *.

Adds V004 migration. set_annotation_state writer comes next."
```

---

## Task 2: Add `set_annotation_state` library function

**Files:**
- Modify: `crates/snk-library/src/captures.rs`

**Step 1: Write the failing tests**

In `crates/snk-library/src/captures.rs`, in the existing `#[cfg(test)] mod tests` block, add three tests:

```rust
    #[test]
    fn set_annotation_state_updates_column() {
        let db = test_db();
        let c = insert(&db, new_capture("a.png", 100, 100)).unwrap();
        let json = r#"{"version":1,"shapes":[],"crop_region":null,"crop_confirmed":false}"#;
        set_annotation_state(&db, &c.id, json).unwrap();
        let updated = get(&db, &c.id).unwrap();
        assert_eq!(updated.annotation_state.as_deref(), Some(json));
    }

    #[test]
    fn set_annotation_state_overwrites_previous_value() {
        let db = test_db();
        let c = insert(&db, new_capture("a.png", 100, 100)).unwrap();
        set_annotation_state(&db, &c.id, r#"{"version":1,"first":true}"#).unwrap();
        set_annotation_state(&db, &c.id, r#"{"version":1,"second":true}"#).unwrap();
        let updated = get(&db, &c.id).unwrap();
        assert!(updated.annotation_state.unwrap().contains("second"));
    }

    #[test]
    fn set_annotation_state_returns_not_found_for_missing_id() {
        let db = test_db();
        let result = set_annotation_state(&db, "no-such-id", "{}");
        match result {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
```

**Step 2: Run to verify they fail**

```
cargo test -p snk-library set_annotation_state
```

Expected: FAIL — `set_annotation_state` does not exist.

**Step 3: Implement `set_annotation_state`**

In the same file, add this function right after the existing `set_annotated_path` function (around line 305):

```rust
pub fn set_annotation_state(db: &Db, id: &str, state_json: &str) -> Result<()> {
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE captures SET annotation_state = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![state_json, id],
        )?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("capture {id}"),
            });
        }
        Ok(())
    })
}
```

This mirrors `set_annotated_path` (above it in the file) exactly — same `db.with_conn` shape, same `if changed == 0` guard, same `NotFound` payload.

**Step 4: Verify the tests pass**

```
cargo test -p snk-library set_annotation_state
```

Expected: all 3 PASS.

**Step 5: Verify the full library test suite**

```
cargo test -p snk-library
```

Expected: every test passes.

**Step 6: Commit**

```
git add crates/snk-library/src/captures.rs
git commit -m "feat(library): set_annotation_state writes the JSON column

Mirrors set_annotated_path: takes a raw JSON string (the frontend is
the only writer and the only reader), guards deleted_at IS NULL, and
returns NotFound when the row does not exist. Three unit tests cover
the happy path, overwrite, and not-found cases."
```

---

## Task 3: Extend `save_annotation` command to take state JSON

**Files:**
- Modify: `crates/snk-annotate/src/commands.rs`

This file isn't unit-tested today (the rest of `snk-annotate` is integration-glue). Verification is `cargo build -p snapper-keeper-app` + `cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings`.

**Step 1: Replace the command**

Open `crates/snk-annotate/src/commands.rs` and replace the entire file contents with:

```rust
use tauri::{Runtime, State};

use snk_library::plugin::LibraryState;
use snk_library::{captures, files, Capture};

use crate::Result;

#[tauri::command]
pub fn save_annotation<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
    png_data: Vec<u8>,
    state_json: String,
) -> Result<Capture> {
    let capture = captures::get(&state.db, &capture_id)?;

    let annotated_relative = captures::annotated_relative_path(&capture.file_path);
    files::write_atomic(&state.root, &annotated_relative, &png_data)?;

    let annotated_str = annotated_relative
        .to_str()
        .ok_or_else(|| crate::AnnotateError::InvalidInput {
            reason: "non-utf8 annotated path".into(),
        })?
        .to_string();

    // Both writes go to the same Db wrapper; rusqlite execs are individually
    // atomic. We accept the small window between them — set_annotated_path
    // is harmless on its own (the PNG is already on disk) and the next
    // save will overwrite both. Wrapping in an explicit transaction would
    // require a Db API change we don't need today.
    captures::set_annotated_path(&state.db, &capture_id, &annotated_str)?;
    captures::set_annotation_state(&state.db, &capture_id, &state_json)?;

    captures::get(&state.db, &capture_id).map_err(Into::into)
}
```

The only behavioral changes from the previous version:
- New `state_json: String` parameter.
- New `captures::set_annotation_state(...)` call between writing the path and re-fetching.

**Step 2: Build the app to confirm the signature still satisfies callers**

```
cargo build -p snapper-keeper-app
```

Expected: success. (No other Rust caller of `save_annotation` exists — it's only invoked over Tauri IPC.)

**Step 3: Clippy + fmt**

```
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
cargo fmt --all
```

Expected: clean; `fmt` should produce no diff (the new function follows the same shape as the prior one).

**Step 4: Commit**

```
git add crates/snk-annotate/src/commands.rs
git commit -m "feat(annotate): save_annotation persists editable state JSON

Extends the command signature with state_json: String. The PNG write
+ annotated_path update + annotation_state update all run against the
same Db wrapper inside one command call. (See the comment in the
source about transactional scope — acceptable trade-off today.)"
```

---

## Task 4: Update the TS bindings to thread the JSON through

**Files:**
- Modify: `packages/snk-library/src/types.ts`
- Modify: `packages/snk-annotate/src/index.ts`
- Modify: `packages/snk-annotate/src/index.test.ts`

**Step 1: Update the failing test first**

Replace the contents of `packages/snk-annotate/src/index.test.ts` with:

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from '@tauri-apps/api/core';
import { saveAnnotation } from './index';

const mockedInvoke = vi.mocked(invoke);

describe('@snk/annotate bindings', () => {
  beforeEach(() => mockedInvoke.mockReset().mockResolvedValue(undefined));

  it('saveAnnotation forwards captureId, PNG bytes, and serialized state', async () => {
    mockedInvoke.mockResolvedValue({ id: 'cap-1' });
    const png = [137, 80, 78, 71];
    const state = {
      version: 1 as const,
      shapes: [],
      crop_region: null,
      crop_confirmed: false,
    };
    const result = await saveAnnotation('cap-1', png, state);
    expect(result).toEqual({ id: 'cap-1' });
    expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-annotate|save_annotation', {
      captureId: 'cap-1',
      pngData: png,
      stateJson: JSON.stringify(state),
    });
  });
});
```

**Step 2: Run to verify it fails**

```
pnpm --filter @snk/annotate exec vitest run
```

Expected: FAIL — either a TS compile error or a mismatch assertion (depending on stub state).

**Step 3: Update the `Capture` TS type**

Open `packages/snk-library/src/types.ts` and add `annotation_state: string | null;` to the `Capture` interface, right after `annotated_path`:

```ts
export interface Capture {
  id: string;
  file_path: string;
  annotated_path: string | null;
  annotation_state: string | null;
  width: number;
  height: number;
  source_app: string | null;
  source_window_title: string | null;
  monitor: string | null;
  created_at: number;
  deleted_at: number | null;
  pinned: boolean;
}
```

**Step 4: Replace `saveAnnotation`**

Open `packages/snk-annotate/src/index.ts` and replace its contents:

```ts
import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';

export * from './types';

// Editable annotation state, serialized into captures.annotation_state.
// The Rust side stores this verbatim — the schema lives here.
export interface AnnotationState {
  version: 1;
  shapes: unknown[]; // exactly what's in store.shapes; opaque to the binding
  crop_region: { x: number; y: number; width: number; height: number } | null;
  crop_confirmed: boolean;
}

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

**Step 5: Verify the test passes**

```
pnpm --filter @snk/annotate exec vitest run
```

Expected: 1 test passing.

**Step 6: Verify all bindings typecheck**

```
pnpm typecheck
```

Expected: clean. (The `shapes: unknown[]` keeps the binding from coupling to the in-app shape type; the app passes its real shapes through and they just transit.)

**Step 7: Verify the full app tests still pass**

```
pnpm --filter @snk/app test
```

Expected: still 131 tests passing — no app code is updated yet, but the bindings still satisfy their existing consumers because `saveAnnotation` is only called from `AnnotateTopBar` (which Task 6 updates).

Wait — Step 7 will actually FAIL here, because `AnnotateTopBar` still calls `saveAnnotation(captureId, png)` without the new third argument. That's a TS error, not a runtime test failure, and `pnpm --filter @snk/app test` runs vitest which DOES compile TS via esbuild but does NOT fail the run on TS errors. The tests will still pass, but `pnpm typecheck` will fail. **Skip to Step 8** if typecheck fails — that's expected and Task 6 fixes it.

**Step 8: Verify the app typecheck fails (expected)**

```
pnpm --filter @snk/app typecheck
```

Expected: 1 error in `AnnotateTopBar.tsx` — `saveAnnotation` expected 3 args, got 2. This is the intentional gap Task 6 closes.

**Step 9: Commit**

```
git add packages/snk-library/src/types.ts \
        packages/snk-annotate/src/index.ts \
        packages/snk-annotate/src/index.test.ts
git commit -m "feat(annotate): bindings thread editable state through saveAnnotation

Adds the AnnotationState type (version:1, shapes, crop_region,
crop_confirmed) and changes saveAnnotation's signature to take it as
a third arg, serializing with JSON.stringify before the invoke. The
Capture TS type gains the matching annotation_state: string | null
field so loads have something to read.

App typecheck is temporarily red at this commit (AnnotateTopBar still
passes the old 2-arg call). Task 6 closes the gap."
```

---

## Task 5: Add `isDirty` + `markClean` to the annotate store

**Files:**
- Modify: `app/src/windows/annotate/store.ts`
- Modify: `app/src/windows/annotate/store.test.ts`

**Step 1: Write the failing tests**

Add these inside the existing `describe('useAnnotateStore', ...)` block, after the last existing test:

```ts
  it('isDirty starts false on a fresh store', () => {
    expect(useAnnotateStore.getState().isDirty).toBe(false);
  });

  it('addShape, updateShape, deleteShape, setCropRegion, confirmCrop all set isDirty', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    expect(useAnnotateStore.getState().isDirty).toBe(false);

    s.updateShape('sh-a', { x: 5 });
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    s.deleteShape('sh-a');
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    s.setCropRegion({ x: 0, y: 0, width: 10, height: 10 });
    expect(useAnnotateStore.getState().isDirty).toBe(true);

    s.markClean();
    s.confirmCrop();
    expect(useAnnotateStore.getState().isDirty).toBe(true);
  });

  it('markClean preserves shapes, crop, and history', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    s.setCropRegion({ x: 0, y: 0, width: 50, height: 50 });
    s.markClean();
    const after = useAnnotateStore.getState();
    expect(after.shapes).toHaveLength(1);
    expect(after.cropRegion).toEqual({ x: 0, y: 0, width: 50, height: 50 });
    expect(after.undoStack.length).toBeGreaterThan(0);
    expect(after.isDirty).toBe(false);
  });

  it('reset clears isDirty', () => {
    const s = useAnnotateStore.getState();
    s.addShape(shapeA);
    expect(useAnnotateStore.getState().isDirty).toBe(true);
    s.reset();
    expect(useAnnotateStore.getState().isDirty).toBe(false);
  });
```

**Step 2: Run to verify they fail**

```
pnpm --filter @snk/app exec vitest run src/windows/annotate/store.test.ts
```

Expected: 4 failing tests — `markClean is not a function` and `isDirty is undefined`.

**Step 3: Add `isDirty` to the store**

Open `app/src/windows/annotate/store.ts`.

(a) In the `AnnotateState` interface, add `isDirty: boolean;` after `sourceImage`:

```ts
interface AnnotateState {
  // ... existing fields ...
  sourceImage: HTMLImageElement | null;
  isDirty: boolean;

  // ... existing setters ...
  setSourceImage: (img: HTMLImageElement | null) => void;
  markClean: () => void;
  reset: () => void;
}
```

(b) In `initialState`, add `isDirty: false` after `sourceImage`:

```ts
const initialState = {
  // ... existing entries ...
  sourceImage: null as HTMLImageElement | null,
  isDirty: false,
};
```

(c) Five existing mutators need to additionally set `isDirty: true`. Modify each by adding `isDirty: true` to the `set({...})` call inside:

- `addShape` — set already wraps the new state; add `isDirty: true` to the object.
- `updateShape` — same.
- `deleteShape` — same.
- `setCropRegion` — same.
- `confirmCrop` — same.

For example, `addShape` becomes:

```ts
  addShape: (shape) => {
    const state = get();
    set({
      undoStack: [...state.undoStack, snapshot(state)],
      redoStack: [],
      shapes: [...state.shapes, shape],
      nextStepNumber:
        shape.tool === 'step-marker' ? state.nextStepNumber + 1 : state.nextStepNumber,
      isDirty: true,
    });
  },
```

Apply the same `isDirty: true` addition to `updateShape`, `deleteShape`, `setCropRegion`, and `confirmCrop`.

(d) Add the `markClean` setter, near `setSourceImage`:

```ts
  setSourceImage: (img) => set({ sourceImage: img }),
  markClean: () => set({ isDirty: false }),
  reset: () => set(initialState),
```

(e) Do **not** touch `undo` or `redo`. Restoring from history should NOT mark the store dirty — that's the whole point of undo. If the user undoes back to the saved state, they're clean again from their POV, but tracking that precisely is more complexity than warranted. Current accepted behavior: undo leaves `isDirty` unchanged (was true before undo → stays true; was false before undo → stays false), which means right after a fresh load Ctrl+Z is a no-op since there's nothing in the stack.

**Step 4: Verify the tests pass**

```
pnpm --filter @snk/app exec vitest run src/windows/annotate/store.test.ts
```

Expected: all 24 tests pass (20 prior + 4 new).

**Step 5: Verify the full app tests still pass**

```
pnpm --filter @snk/app test
```

Expected: 135 tests passing (131 prior + 4 new).

**Step 6: Commit**

```
git add app/src/windows/annotate/store.ts app/src/windows/annotate/store.test.ts
git commit -m "feat(annotate): track isDirty + markClean on the store

addShape, updateShape, deleteShape, setCropRegion, and confirmCrop
all flip isDirty: true. markClean() resets it; reset() also clears it
as part of going back to initialState. undo/redo intentionally do not
touch the flag — restoring from history is not a 'change' in the
user's mental model.

The flag enables the upcoming dirty-confirmation dialog on the three
exit paths (done button / X close / Esc)."
```

---

## Task 6: AnnotateTopBar serializes state on Save and marks clean

**Files:**
- Modify: `app/src/windows/annotate/AnnotateTopBar.tsx`

Excluded from coverage; verified by typecheck + lint + manual.

**Step 1: Update `handleSave` to serialize state and mark clean**

Open `app/src/windows/annotate/AnnotateTopBar.tsx`. Find the existing `handleSave`:

```tsx
  const handleSave = useCallback(async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      const png = await exportPng();
      if (!png) throw new Error('nothing to save');
      await saveAnnotation(captureId, png);
      flashSave('ok');
    } catch (e) {
      console.error('save annotation failed', e);
      flashSave('err');
    } finally {
      saving.current = false;
    }
  }, [captureId, exportPng, flashSave]);
```

Replace it with:

```tsx
  const handleSave = useCallback(async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      const png = await exportPng();
      if (!png) throw new Error('nothing to save');
      const store = useAnnotateStore.getState();
      const state = {
        version: 1 as const,
        shapes: store.shapes,
        crop_region: store.cropRegion,
        crop_confirmed: store.cropConfirmed,
      };
      await saveAnnotation(captureId, png, state);
      useAnnotateStore.getState().markClean();
      flashSave('ok');
    } catch (e) {
      console.error('save annotation failed', e);
      flashSave('err');
    } finally {
      saving.current = false;
    }
  }, [captureId, exportPng, flashSave]);
```

Three changes:
- Reads the live store state to build the serializable snapshot.
- Passes that snapshot as the third arg to `saveAnnotation`.
- On success, calls `markClean()` so the dirty-guard knows we're synced.

`useAnnotateStore` is already imported at the top of the file.

**Step 2: Verify typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Both should be clean now — the Task 4 typecheck regression closes here.

**Step 3: Verify tests**

```
pnpm --filter @snk/app test
```

Expected: 135 tests passing.

**Step 4: Commit**

```
git add app/src/windows/annotate/AnnotateTopBar.tsx
git commit -m "feat(annotate): handleSave persists editable state, marks clean

Snapshots store.shapes + store.cropRegion + store.cropConfirmed into
an AnnotationState payload and passes it as the third arg to
saveAnnotation. On success, calls markClean() so subsequent edits
re-trigger isDirty and the dirty-guard prompts.

Closes the typecheck gap introduced by the bindings change."
```

---

## Task 7: Create the dirty-confirmation helper

**Files:**
- Create: `app/src/windows/annotate/dirtyGuard.ts`

**Step 1: Write the helper**

Create `app/src/windows/annotate/dirtyGuard.ts`:

```ts
import { useAnnotateStore } from './store';

const DISCARD_PROMPT = 'You have unsaved annotation changes. Discard them?';

// Returns true when the caller should proceed (no dirty state, or the
// user explicitly chose to discard). Returns false when the caller must
// cancel the exit (dirty state + user clicked Cancel). Used by every
// path that leaves the annotator: the done button, the window's X
// intercept, and the Esc keyboard handler.
export function confirmDiscardIfDirty(): boolean {
  const { isDirty } = useAnnotateStore.getState();
  if (!isDirty) return true;
  return window.confirm(DISCARD_PROMPT);
}
```

`window.confirm` is synchronous and blocking; the helper stays synchronous. No async wrapper needed.

**Step 2: Verify typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Both clean.

**Step 3: Commit (with the wire-ups in Task 8 — these are tightly coupled, so we batch one commit)**

Skip — Task 8 commits this together with the three exit-path updates.

---

## Task 8: Wire the dirty guard into the three exit paths

**Files:**
- Modify: `app/src/windows/annotate/AnnotateTopBar.tsx`
- Modify: `app/src/windows/annotate/AnnotateCanvas.tsx`
- Modify: `app/src/windows/annotate/AnnotateWindow.tsx`

All three are excluded from coverage. Verification: typecheck + lint + manual.

**Step 1: AnnotateTopBar — done button**

Open `app/src/windows/annotate/AnnotateTopBar.tsx`. Add the import near the top:

```tsx
import { confirmDiscardIfDirty } from './dirtyGuard';
```

Find `handleDone`:

```tsx
  const handleDone = useCallback(async () => {
    useAnnotateStore.getState().reset();
    const win = getCurrentWindow();
    await win.hide();
  }, []);
```

Replace with:

```tsx
  const handleDone = useCallback(async () => {
    if (!confirmDiscardIfDirty()) return;
    useAnnotateStore.getState().reset();
    const win = getCurrentWindow();
    await win.hide();
  }, []);
```

**Step 2: AnnotateCanvas — Esc keyboard handler**

Open `app/src/windows/annotate/AnnotateCanvas.tsx`. Add the import:

```tsx
import { confirmDiscardIfDirty } from './dirtyGuard';
```

Find the existing `handleKeyDown`'s Escape branch:

```tsx
      if (e.key === 'Escape') {
        if (selectedId) {
          setSelectedId(null);
          return;
        }
        useAnnotateStore.getState().reset();
        getCurrentWindow().hide();
        return;
      }
```

Replace with:

```tsx
      if (e.key === 'Escape') {
        if (selectedId) {
          setSelectedId(null);
          return;
        }
        if (!confirmDiscardIfDirty()) return;
        useAnnotateStore.getState().reset();
        getCurrentWindow().hide();
        return;
      }
```

**Step 3: AnnotateWindow — X close intercept**

Open `app/src/windows/annotate/AnnotateWindow.tsx`. Add the import:

```tsx
import { confirmDiscardIfDirty } from './dirtyGuard';
```

Find the close-intercept useEffect:

```tsx
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        event.preventDefault();
        useAnnotateStore.getState().reset();
        setCaptureId(null);
        await getCurrentWindow().hide();
      })
      .then((fn) => {
        cleanup = fn;
      })
      .catch((e) => console.error('annotate close listener failed', e));
    return () => cleanup?.();
  }, []);
```

Replace with:

```tsx
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        event.preventDefault();
        if (!confirmDiscardIfDirty()) return;
        useAnnotateStore.getState().reset();
        setCaptureId(null);
        await getCurrentWindow().hide();
      })
      .then((fn) => {
        cleanup = fn;
      })
      .catch((e) => console.error('annotate close listener failed', e));
    return () => cleanup?.();
  }, []);
```

**Step 4: Verify typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Both clean.

**Step 5: Verify tests**

```
pnpm --filter @snk/app test
```

Expected: 135 tests passing.

**Step 6: Commit**

```
git add app/src/windows/annotate/dirtyGuard.ts \
        app/src/windows/annotate/AnnotateTopBar.tsx \
        app/src/windows/annotate/AnnotateCanvas.tsx \
        app/src/windows/annotate/AnnotateWindow.tsx
git commit -m "feat(annotate): confirm before discarding unsaved edits

New dirtyGuard helper exposes confirmDiscardIfDirty() — returns true
to proceed, false to cancel. Wired into the three exit paths:
- AnnotateTopBar.handleDone (the done button)
- AnnotateCanvas Esc handler (when nothing is selected)
- AnnotateWindow onCloseRequested (the X / hide-to-tray)

Uses a native window.confirm rather than a Tauri dialog plugin so
we don't add a dependency for a single one-line prompt. The look is
browser-y; acceptable for a side project."
```

---

## Task 9: Load saved state on capture open

**Files:**
- Modify: `app/src/windows/annotate/AnnotateWindow.tsx`

**Step 1: Add a load-effect that pushes annotation_state into the store**

Open `app/src/windows/annotate/AnnotateWindow.tsx`. Add a `useEffect` after the existing `capture` useQuery declaration (around line 65). Insert this block just before the `if (!captureId || !capture.data || !root.data) return ...` guard:

```tsx
  // When a capture loads, hydrate the store from its persisted
  // annotation_state. The fresh reset() in the open-listener wipes the
  // store first, so this either populates it (re-edit case) or leaves
  // it empty (first edit / legacy capture).
  useEffect(() => {
    if (!capture.data?.annotation_state) return;
    try {
      const parsed = JSON.parse(capture.data.annotation_state) as {
        version: number;
        shapes: unknown[];
        crop_region: { x: number; y: number; width: number; height: number } | null;
        crop_confirmed: boolean;
      };
      if (parsed.version !== 1) return;
      useAnnotateStore.setState({
        shapes: parsed.shapes as never,
        cropRegion: parsed.crop_region,
        cropConfirmed: parsed.crop_confirmed,
        undoStack: [],
        redoStack: [],
        isDirty: false,
      });
    } catch (e) {
      console.warn('annotation_state parse failed; opening clean', e);
    }
  }, [capture.data]);
```

Notes:
- The `shapes as never` cast is intentional — the type lives in `@snk/annotate` (`AnnotationShape`) but the binding kept it as `unknown[]` to avoid coupling. The store's `shapes` field is `AnnotationShape[]`; the cast hops over the binding's intentional opacity.
- `undoStack` and `redoStack` are intentionally reset — fresh history per session, as the spec says.
- `isDirty: false` because hydrating a saved state is by definition "clean".
- A parse failure falls through silently — the store is already clean from the open-listener's `reset()`, so the user gets a fresh canvas rather than a crash.

**Step 2: Verify typecheck + lint**

```
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Both clean.

**Step 3: Verify tests**

```
pnpm --filter @snk/app test
```

Expected: 135 tests passing.

**Step 4: Commit**

```
git add app/src/windows/annotate/AnnotateWindow.tsx
git commit -m "feat(annotate): hydrate the store from capture.annotation_state on open

When the capture query resolves with a non-null annotation_state, parse
it and push shapes + crop into the store. Undo stacks reset (fresh
history per session, per spec). isDirty starts false. Malformed JSON
falls through to a clean canvas with a console.warn, not a crash."
```

---

## Task 10: Manual verification — full spec test plan

**Step 1: Start the dev server**

```
pnpm --filter @snk/app tauri dev
```

**Step 2: First edit + restore round-trip**

1. Capture a screenshot of any web page.
2. Open it in the annotator from the library window's thumbnail grid.
3. Draw an arrow, a rectangle, and a confirmed crop region. Click Save → confirm "saved ✓" flashes.
4. Click the **done** button (no confirm dialog should appear — `isDirty` is now false).
5. From the library, click the same capture's thumbnail again to re-open.
6. **Verify:** the arrow, rectangle, and crop all reappear in their saved positions, with the crop dim still applied (green outline, dim outside).

**Step 3: Dirty-guard on the done button**

1. With the re-opened capture, draw one more arrow. Don't save.
2. Click **done**.
3. **Verify:** native confirm dialog appears with "You have unsaved annotation changes. Discard them?". Click **Cancel**.
4. **Verify:** the annotator stays open, your new arrow is still there.
5. Click **done** again → click **OK** → window hides.
6. Re-open the same capture.
7. **Verify:** the new arrow is gone (because you discarded it). The original saved state (arrow + rect + crop) is intact.

**Step 4: Dirty-guard on the X close**

1. Same capture, draw a new shape.
2. Click the window's X button.
3. **Verify:** confirm dialog appears. Click **OK** to discard.
4. Re-open → only the previously-saved state.

**Step 5: Dirty-guard on Esc**

1. Same capture, draw another shape.
2. Press **Esc** with nothing selected.
3. **Verify:** confirm dialog. Click **Cancel** → window stays.
4. Press **Esc** again → click **OK** → window hides.

**Step 6: Esc still deselects when something IS selected**

1. Re-open the capture.
2. Click the arrow (so it shows the Transformer handles).
3. Press **Esc**.
4. **Verify:** the selection clears but the window stays — no confirm dialog (because dirty check only happens when nothing is selected).

**Step 7: Legacy capture (annotation_state IS NULL)**

This requires a row that has `annotated_path` set but `annotation_state` NULL — i.e. a capture annotated before this feature shipped. If you don't have one, skip this step or simulate it with sqlite:

```
# In a separate shell, point to the snapper-keeper user data DB:
sqlite3 "%APPDATA%\com.snapper-keeper.app\library.db" \
  "UPDATE captures SET annotation_state = NULL WHERE id = '<some-id>';"
```

Then open that capture in the annotator.

**Verify:** clean canvas (no shapes, no crop, no errors in dev tools console).

**Step 8: Brand-new capture**

1. Take a fresh screenshot.
2. Open it in the annotator without making any edits.
3. **Verify:** clean canvas. No crop dim. Done button works without prompting.

**Step 9: Double-save + reload**

1. Open a previously-saved capture.
2. Draw an extra shape, Save.
3. Move an existing shape, Save again.
4. Close + reopen.
5. **Verify:** the state from the *second* save is what you see — not stale.

**Step 10: Stop the dev server**

If all manual checks pass, the implementation is done. If anything fails, file a follow-up commit referencing the failing step.

---

## Acceptance criteria

The plan is complete when:

- All 9 implementation commits land on the working branch.
- `cargo test -p snk-library` passes (with the 4 new tests).
- `cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- `pnpm --filter @snk/app test` passes (135 tests).
- `pnpm typecheck` and `pnpm lint` both clean.
- All 10 manual verification steps pass.

Then hand off to `h-superpowers:finishing-a-development-branch` for merge.

---

## Notes on test coverage

`AnnotateCanvas.tsx`, `AnnotateWindow.tsx`, `AnnotateTopBar.tsx`, and shape files are in Vitest's coverage exclude list (Konva + Tauri integration glue). Tasks 6, 7, 8, and 9 add no automated tests for those files — by design, matching the existing project convention. The dirty-guard helper (`dirtyGuard.ts`) is technically testable in isolation but the entire surface is `window.confirm()` which can't be exercised in happy-dom, so it's effectively a manual-verified surface too. The store + bindings + Rust crate are where automated tests live; that's by design.
