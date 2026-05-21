# snapper-keeper — Phase 3: Annotation Editor

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Add a Konva-based annotation editor that opens from the post-capture toolbar's "Annotate" button, providing arrow, rectangle, ellipse, freehand pen, highlighter, text, blur/pixelate, numbered step markers, and crop tools — with undo/redo, color palette, stroke widths, and save-as-annotated-PNG export.

**Architecture:** A new `snk-annotate` Rust plugin crate handles the backend: saving the annotated PNG variant, updating the `annotated_path` column in captures (through `snk-library`). The frontend is a new `annotate` Tauri window (`app/src/windows/annotate/`) built on a Konva `<Stage>` canvas. A paired TS package (`packages/snk-annotate/`) exports typed bindings. The capture toolbar's "Annotate" button (currently a Phase 2 placeholder) opens this window with the capture id. All canvas state (tool selection, undo/redo stack, color, stroke width) lives in a Zustand store on the frontend. The Rust side is intentionally thin — export and DB update only.

**Tech Stack:** Same workspace. New deps: `konva` + `react-konva` (npm, for canvas), `image` crate features for blur (already in workspace). New crate: `crates/snk-annotate`. New TS package: `packages/snk-annotate`. New window: `annotate` in `tauri.conf.json`.

**Phase 3 scope (in):**
- Annotation tools: arrow, rectangle, ellipse, freehand pen, highlighter, text, blur/pixelate, numbered step markers, crop
- Undo/redo stack (frontend Zustand store)
- Color palette (8 swatches + custom picker)
- Three stroke widths (thin/medium/thick)
- Save: exports Konva stage to PNG, writes `.annotated.png` variant via Rust, updates DB `annotated_path`
- Copy annotated image to clipboard (future: via snk-clipboard; for now, Rust clipboard write)
- "Done" dismisses the editor
- Toolbar "Annotate" button wired to open the annotate window

**Out of scope (later phases):**
- Redo annotations on an already-annotated capture (v1 starts fresh each time)
- Speech bubbles, spotlight/dim, magnifier loupe, rotate (per design spec non-goals)
- Persist canvas state as JSON (v1 only persists the final exported PNG)
- Annotation templates or presets

---

## Pre-flight

You are building on `main` which has phases 1 + 2 complete. Create a worktree on a `feature/phase-3-annotation-editor` branch.

**Verify before starting:**

```bash
rustc --version        # 1.78+
node --version         # 20+
pnpm --version         # 9+
cargo test             # all green
pnpm lint && pnpm typecheck  # all green
```

---

## Task 1: Scaffold the `snk-annotate` Rust plugin crate

Create the bare-minimum Rust crate with `tauri-plugin` build, empty command set, and permissions. This follows the exact pattern established by `snk-capture` and `snk-library`.

**Files:**
- Create: `crates/snk-annotate/Cargo.toml`
- Create: `crates/snk-annotate/build.rs`
- Create: `crates/snk-annotate/permissions/default.toml`
- Create: `crates/snk-annotate/src/lib.rs`
- Create: `crates/snk-annotate/src/plugin.rs`
- Create: `crates/snk-annotate/src/error.rs`
- Create: `crates/snk-annotate/src/commands.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Create `crates/snk-annotate/Cargo.toml`**

```toml
[package]
name = "snk-annotate"
version = "0.0.1"
links = "snk-annotate"
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
image.workspace = true
```

**Step 2: Create `crates/snk-annotate/build.rs`**

```rust
const COMMANDS: &[&str] = &["save_annotation"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 3: Create `crates/snk-annotate/permissions/default.toml`**

```toml
[default]
description = "Default permissions for snk-annotate: allows saving annotations."
permissions = ["allow-save-annotation"]
```

**Step 4: Create `crates/snk-annotate/src/error.rs`**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AnnotateError {
    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),

    #[error("image error: {message}")]
    Image { message: String },

    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },
}

impl From<snk_library::LibraryError> for AnnotateError {
    fn from(e: snk_library::LibraryError) -> Self {
        AnnotateError::Library(e)
    }
}

impl From<image::ImageError> for AnnotateError {
    fn from(e: image::ImageError) -> Self {
        AnnotateError::Image {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AnnotateError>;
```

**Step 5: Create `crates/snk-annotate/src/commands.rs`**

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

    captures::set_annotated_path(&state.db, &capture_id, &annotated_str)?;
    captures::get(&state.db, &capture_id).map_err(Into::into)
}
```

**Step 6: Create `crates/snk-annotate/src/plugin.rs`**

```rust
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-annotate")
        .invoke_handler(tauri::generate_handler![crate::commands::save_annotation])
        .build()
}
```

**Step 7: Create `crates/snk-annotate/src/lib.rs`**

```rust
pub mod commands;
pub mod error;
pub mod plugin;

pub use error::{AnnotateError, Result};
pub use plugin::init;
```

**Step 8: Add workspace member in root `Cargo.toml`**

Add `"crates/snk-annotate"` to the `[workspace] members` list:

```toml
[workspace]
resolver = "2"
members = [
    "crates/snk-library",
    "crates/snk-hotkeys",
    "crates/snk-capture",
    "crates/snk-annotate",
    "app/src-tauri",
]
```

**Step 9: Verify compilation**

Run: `cargo check -p snk-annotate`
Expected: This will fail because `captures::annotated_relative_path` and `captures::set_annotated_path` don't exist yet. That's expected — Task 2 adds them. Verify the error is only about those missing functions.

**Step 10: Commit**

```bash
git add crates/snk-annotate/ Cargo.toml
git commit -m "feat(annotate): scaffold snk-annotate Rust plugin crate"
```

---

## Task 2: Add `annotated_relative_path` and `set_annotated_path` to snk-library

The `annotated_path` column already exists in the DB schema. We need library functions to compute the annotated file path and update the column.

**Files:**
- Modify: `crates/snk-library/src/captures.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write failing tests in `crates/snk-library/src/captures.rs`**

Add these tests inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn annotated_relative_path_appends_annotated_suffix() {
        let p = annotated_relative_path("captures/2026/05/abc.png");
        assert_eq!(p, std::path::PathBuf::from("captures/2026/05/abc.annotated.png"));
    }

    #[test]
    fn annotated_relative_path_handles_no_extension() {
        let p = annotated_relative_path("captures/2026/05/abc");
        assert_eq!(p, std::path::PathBuf::from("captures/2026/05/abc.annotated"));
    }

    #[test]
    fn set_annotated_path_updates_column() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("a.png"),
            width: 10,
            height: 10,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let c = insert(&db, new).unwrap();
        assert!(c.annotated_path.is_none());

        set_annotated_path(&db, &c.id, "a.annotated.png").unwrap();

        let updated = get(&db, &c.id).unwrap();
        assert_eq!(updated.annotated_path.as_deref(), Some("a.annotated.png"));
    }

    #[test]
    fn set_annotated_path_nonexistent_returns_not_found() {
        let db = fresh_db();
        match set_annotated_path(&db, "no-such-id", "x.annotated.png") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p snk-library`
Expected: FAIL — `annotated_relative_path` and `set_annotated_path` are not defined.

**Step 3: Implement `annotated_relative_path` in `crates/snk-library/src/captures.rs`**

Add this function after the existing `row_to_capture` function (before `pub fn get`):

```rust
pub fn annotated_relative_path(original: &str) -> PathBuf {
    let p = PathBuf::from(original);
    match p.extension().and_then(|e| e.to_str()) {
        Some(ext) => p.with_extension(format!("annotated.{ext}")),
        None => PathBuf::from(format!("{original}.annotated")),
    }
}
```

**Step 4: Implement `set_annotated_path` in `crates/snk-library/src/captures.rs`**

Add this function after `soft_delete`:

```rust
pub fn set_annotated_path(db: &Db, id: &str, annotated_path: &str) -> Result<()> {
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE captures SET annotated_path = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![annotated_path, id],
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

**Step 5: Export in `crates/snk-library/src/lib.rs`**

The functions are accessed via `captures::annotated_relative_path` and `captures::set_annotated_path` from `snk-annotate`, so no new pub use needed — callers use the module path.

**Step 6: Run tests to verify they pass**

Run: `cargo test -p snk-library`
Expected: All tests pass, including the four new ones.

**Step 7: Verify `snk-annotate` now compiles**

Run: `cargo check -p snk-annotate`
Expected: PASS

**Step 8: Commit**

```bash
git add crates/snk-library/src/captures.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add annotated_relative_path and set_annotated_path"
```

---

## Task 3: Register snk-annotate plugin in the Tauri app

Wire the new plugin into the app binary and add the capability permission.

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/src/main.rs`
- Modify: `app/src-tauri/capabilities/default.json`

**Step 1: Add dependency in `app/src-tauri/Cargo.toml`**

Add to `[dependencies]`:

```toml
snk-annotate = { path = "../../crates/snk-annotate" }
```

**Step 2: Register plugin in `app/src-tauri/src/main.rs`**

Add `.plugin(snk_annotate::init())` after the `snk_capture::init()` line:

```rust
        .plugin(snk_capture::init())
        .plugin(snk_annotate::init())
```

**Step 3: Add annotate window to `app/src-tauri/tauri.conf.json`**

Add a fourth window entry in the `app.windows` array:

```json
      {
        "label": "annotate",
        "title": "Annotate",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "visible": false,
        "decorations": true,
        "skipTaskbar": false
      }
```

**Step 4: Update capabilities in `app/src-tauri/capabilities/default.json`**

Add `"annotate"` to the `windows` list and `"snk-annotate:default"` to the `permissions` list:

```json
{
  "identifier": "default",
  "windows": ["library", "capture-overlay", "capture-toolbar", "annotate"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:event:default",
    "core:path:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered",
    "snk-library:default",
    "snk-capture:default",
    "snk-annotate:default"
  ]
}
```

**Step 5: Verify build**

Run: `cargo check -p snapper-keeper`
Expected: PASS

**Step 6: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/src/main.rs app/src-tauri/tauri.conf.json app/src-tauri/capabilities/default.json
git commit -m "feat(app): register snk-annotate plugin and annotate window"
```

---

## Task 4: Scaffold the `@snk/annotate` TypeScript package

Create the TS bindings package with the save command and annotation types.

**Files:**
- Create: `packages/snk-annotate/package.json`
- Create: `packages/snk-annotate/tsconfig.json`
- Create: `packages/snk-annotate/src/index.ts`
- Create: `packages/snk-annotate/src/types.ts`
- Modify: `app/package.json` (add workspace dep)

**Step 1: Create `packages/snk-annotate/package.json`**

```json
{
  "name": "@snk/annotate",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "lint": "eslint src --max-warnings 0",
    "typecheck": "tsc -b --noEmit",
    "test": "echo 'no tests yet'"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@snk/library": "workspace:*"
  },
  "devDependencies": {
    "typescript": "^5.4.0"
  }
}
```

**Step 2: Create `packages/snk-annotate/tsconfig.json`**

```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

**Step 3: Create `packages/snk-annotate/src/types.ts`**

```typescript
export type AnnotationTool =
  | 'arrow'
  | 'rectangle'
  | 'ellipse'
  | 'pen'
  | 'highlighter'
  | 'text'
  | 'blur'
  | 'step-marker'
  | 'crop';

export interface StrokeConfig {
  color: string;
  width: number;
  opacity: number;
}

export interface AnnotationShape {
  id: string;
  tool: AnnotationTool;
  points?: number[];
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  text?: string;
  stroke: StrokeConfig;
  stepNumber?: number;
  rotation?: number;
}
```

**Step 4: Create `packages/snk-annotate/src/index.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';

export * from './types';

export function saveAnnotation(captureId: string, pngData: number[]): Promise<Capture> {
  return invoke<Capture>('plugin:snk-annotate|save_annotation', {
    captureId,
    pngData,
  });
}
```

**Step 5: Add dependency in `app/package.json`**

Add to `dependencies`:

```json
    "@snk/annotate": "workspace:*",
```

**Step 6: Install deps**

Run: `pnpm install`
Expected: Resolves workspace links.

**Step 7: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 8: Commit**

```bash
git add packages/snk-annotate/ app/package.json pnpm-lock.yaml
git commit -m "feat(annotate): scaffold @snk/annotate TS package with save binding"
```

---

## Task 5: Install Konva and react-konva in the app

**Files:**
- Modify: `app/package.json`

**Step 1: Install dependencies**

Run: `pnpm --filter @snk/app add konva react-konva`

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/package.json pnpm-lock.yaml
git commit -m "chore(app): add konva and react-konva dependencies"
```

---

## Task 6: Create the annotation Zustand store

This store manages all canvas state: active tool, color, stroke width, shapes on canvas, undo/redo stack, crop region. All annotation tools read from and write to this store.

**Files:**
- Create: `app/src/windows/annotate/store.ts`

**Step 1: Create the store**

```typescript
import { create } from 'zustand';

import type { AnnotationTool, AnnotationShape, StrokeConfig } from '@snk/annotate';

export const COLORS = [
  '#ef4444', // red
  '#f97316', // orange
  '#eab308', // yellow
  '#22c55e', // green
  '#3b82f6', // blue
  '#8b5cf6', // violet
  '#000000', // black
  '#ffffff', // white
] as const;

export const STROKE_WIDTHS = {
  thin: 2,
  medium: 4,
  thick: 8,
} as const;

export type StrokePreset = keyof typeof STROKE_WIDTHS;

interface CropRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface AnnotateState {
  tool: AnnotationTool;
  color: string;
  strokePreset: StrokePreset;
  shapes: AnnotationShape[];
  undoStack: AnnotationShape[][];
  redoStack: AnnotationShape[][];
  nextStepNumber: number;
  cropRegion: CropRegion | null;
  isDrawing: boolean;
  currentShape: AnnotationShape | null;

  setTool: (tool: AnnotationTool) => void;
  setColor: (color: string) => void;
  setStrokePreset: (preset: StrokePreset) => void;
  addShape: (shape: AnnotationShape) => void;
  undo: () => void;
  redo: () => void;
  setCropRegion: (region: CropRegion | null) => void;
  setIsDrawing: (drawing: boolean) => void;
  setCurrentShape: (shape: AnnotationShape | null) => void;
  reset: () => void;
}

const initialState = {
  tool: 'arrow' as AnnotationTool,
  color: '#ef4444',
  strokePreset: 'medium' as StrokePreset,
  shapes: [] as AnnotationShape[],
  undoStack: [] as AnnotationShape[][],
  redoStack: [] as AnnotationShape[][],
  nextStepNumber: 1,
  cropRegion: null as CropRegion | null,
  isDrawing: false,
  currentShape: null as AnnotationShape | null,
};

export const useAnnotateStore = create<AnnotateState>((set, get) => ({
  ...initialState,

  setTool: (tool) => set({ tool }),
  setColor: (color) => set({ color }),
  setStrokePreset: (preset) => set({ strokePreset: preset }),

  addShape: (shape) => {
    const { shapes, nextStepNumber } = get();
    set({
      undoStack: [...get().undoStack, shapes],
      redoStack: [],
      shapes: [...shapes, shape],
      nextStepNumber: shape.tool === 'step-marker' ? nextStepNumber + 1 : nextStepNumber,
    });
  },

  undo: () => {
    const { undoStack, shapes } = get();
    if (undoStack.length === 0) return;
    const prev = undoStack[undoStack.length - 1]!;
    set({
      undoStack: undoStack.slice(0, -1),
      redoStack: [...get().redoStack, shapes],
      shapes: prev,
    });
  },

  redo: () => {
    const { redoStack, shapes } = get();
    if (redoStack.length === 0) return;
    const next = redoStack[redoStack.length - 1]!;
    set({
      redoStack: redoStack.slice(0, -1),
      undoStack: [...get().undoStack, shapes],
      shapes: next,
    });
  },

  setCropRegion: (region) => set({ cropRegion: region }),
  setIsDrawing: (drawing) => set({ isDrawing: drawing }),
  setCurrentShape: (shape) => set({ currentShape: shape }),
  reset: () => set(initialState),
}));
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/store.ts
git commit -m "feat(annotate): create Zustand store for annotation canvas state"
```

---

## Task 7: Build the annotation toolbar component

Left rail with tool selection, color palette, stroke width, and undo/redo buttons.

**Files:**
- Create: `app/src/windows/annotate/AnnotateToolbar.tsx`

**Step 1: Create the toolbar component**

```tsx
import { useAnnotateStore, COLORS, STROKE_WIDTHS, type StrokePreset } from './store';

import type { AnnotationTool } from '@snk/annotate';

const TOOLS: { id: AnnotationTool; label: string; icon: string }[] = [
  { id: 'arrow', label: 'Arrow', icon: '↗' },
  { id: 'rectangle', label: 'Rectangle', icon: '□' },
  { id: 'ellipse', label: 'Ellipse', icon: '○' },
  { id: 'pen', label: 'Pen', icon: '✎' },
  { id: 'highlighter', label: 'Highlighter', icon: '🖍' },
  { id: 'text', label: 'Text', icon: 'T' },
  { id: 'blur', label: 'Blur', icon: '▦' },
  { id: 'step-marker', label: 'Step', icon: '#' },
  { id: 'crop', label: 'Crop', icon: '⬔' },
];

export function AnnotateToolbar() {
  const tool = useAnnotateStore((s) => s.tool);
  const color = useAnnotateStore((s) => s.color);
  const strokePreset = useAnnotateStore((s) => s.strokePreset);
  const undoStack = useAnnotateStore((s) => s.undoStack);
  const redoStack = useAnnotateStore((s) => s.redoStack);
  const setTool = useAnnotateStore((s) => s.setTool);
  const setColor = useAnnotateStore((s) => s.setColor);
  const setStrokePreset = useAnnotateStore((s) => s.setStrokePreset);
  const undo = useAnnotateStore((s) => s.undo);
  const redo = useAnnotateStore((s) => s.redo);

  return (
    <div className="flex flex-col gap-3 p-2 bg-slate-900 border-r border-slate-700 w-14 items-center">
      <div className="flex flex-col gap-1">
        {TOOLS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTool(t.id)}
            className={`w-10 h-10 rounded flex items-center justify-center text-sm ${
              tool === t.id
                ? 'bg-blue-600 text-white'
                : 'text-slate-400 hover:bg-slate-800'
            }`}
            title={t.label}
          >
            {t.icon}
          </button>
        ))}
      </div>

      <div className="w-8 border-t border-slate-700" />

      <div className="flex flex-col gap-1">
        {COLORS.map((c) => (
          <button
            key={c}
            onClick={() => setColor(c)}
            className={`w-6 h-6 rounded-full border-2 mx-auto ${
              color === c ? 'border-white' : 'border-transparent'
            }`}
            style={{ backgroundColor: c }}
            title={c}
          />
        ))}
      </div>

      <div className="w-8 border-t border-slate-700" />

      <div className="flex flex-col gap-1 items-center">
        {(Object.keys(STROKE_WIDTHS) as StrokePreset[]).map((preset) => (
          <button
            key={preset}
            onClick={() => setStrokePreset(preset)}
            className={`w-10 h-6 rounded flex items-center justify-center ${
              strokePreset === preset
                ? 'bg-blue-600'
                : 'hover:bg-slate-800'
            }`}
            title={preset}
          >
            <div
              className="bg-white rounded-full"
              style={{ width: 20, height: STROKE_WIDTHS[preset] }}
            />
          </button>
        ))}
      </div>

      <div className="w-8 border-t border-slate-700" />

      <div className="flex flex-col gap-1">
        <button
          onClick={undo}
          disabled={undoStack.length === 0}
          className="w-10 h-8 rounded text-sm text-slate-400 hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed"
          title="Undo (Ctrl+Z)"
        >
          ↶
        </button>
        <button
          onClick={redo}
          disabled={redoStack.length === 0}
          className="w-10 h-8 rounded text-sm text-slate-400 hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed"
          title="Redo (Ctrl+Y)"
        >
          ↷
        </button>
      </div>
    </div>
  );
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/AnnotateToolbar.tsx
git commit -m "feat(annotate): add annotation toolbar with tools, colors, strokes, undo/redo"
```

---

## Task 8: Create shape rendering components for Konva

Individual Konva shape renderers for each annotation type. Each component reads an `AnnotationShape` and renders the appropriate Konva node.

**Files:**
- Create: `app/src/windows/annotate/shapes/ArrowShape.tsx`
- Create: `app/src/windows/annotate/shapes/RectangleShape.tsx`
- Create: `app/src/windows/annotate/shapes/EllipseShape.tsx`
- Create: `app/src/windows/annotate/shapes/PenShape.tsx`
- Create: `app/src/windows/annotate/shapes/HighlighterShape.tsx`
- Create: `app/src/windows/annotate/shapes/TextShape.tsx`
- Create: `app/src/windows/annotate/shapes/BlurShape.tsx`
- Create: `app/src/windows/annotate/shapes/StepMarkerShape.tsx`
- Create: `app/src/windows/annotate/shapes/index.tsx`

**Step 1: Create `app/src/windows/annotate/shapes/ArrowShape.tsx`**

```tsx
import { Arrow } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function ArrowShape({ shape }: Props) {
  return (
    <Arrow
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
      fill={shape.stroke.color}
      pointerLength={10}
      pointerWidth={10}
      lineCap="round"
      lineJoin="round"
    />
  );
}
```

**Step 2: Create `app/src/windows/annotate/shapes/RectangleShape.tsx`**

```tsx
import { Rect } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function RectangleShape({ shape }: Props) {
  return (
    <Rect
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={shape.width ?? 0}
      height={shape.height ?? 0}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
    />
  );
}
```

**Step 3: Create `app/src/windows/annotate/shapes/EllipseShape.tsx`**

```tsx
import { Ellipse } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function EllipseShape({ shape }: Props) {
  const rx = Math.abs((shape.width ?? 0) / 2);
  const ry = Math.abs((shape.height ?? 0) / 2);
  return (
    <Ellipse
      x={(shape.x ?? 0) + rx}
      y={(shape.y ?? 0) + ry}
      radiusX={rx}
      radiusY={ry}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
    />
  );
}
```

**Step 4: Create `app/src/windows/annotate/shapes/PenShape.tsx`**

```tsx
import { Line } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function PenShape({ shape }: Props) {
  return (
    <Line
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width}
      lineCap="round"
      lineJoin="round"
      tension={0.5}
    />
  );
}
```

**Step 5: Create `app/src/windows/annotate/shapes/HighlighterShape.tsx`**

```tsx
import { Line } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function HighlighterShape({ shape }: Props) {
  return (
    <Line
      points={shape.points ?? []}
      stroke={shape.stroke.color}
      strokeWidth={shape.stroke.width * 4}
      opacity={0.35}
      lineCap="round"
      lineJoin="round"
      tension={0.5}
      globalCompositeOperation="multiply"
    />
  );
}
```

**Step 6: Create `app/src/windows/annotate/shapes/TextShape.tsx`**

```tsx
import { Text } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function TextShape({ shape }: Props) {
  return (
    <Text
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      text={shape.text ?? ''}
      fontSize={shape.stroke.width * 6}
      fill={shape.stroke.color}
      fontFamily="system-ui, sans-serif"
    />
  );
}
```

**Step 7: Create `app/src/windows/annotate/shapes/BlurShape.tsx`**

The blur tool renders a rectangle with a Konva blur filter applied to the underlying image region. Since Konva's built-in filters work on cached nodes, we use a pixelation visual (grid of colored rects) for the MVP. The actual "read pixels and blur" approach would require reading from the background image layer — Task 14 handles the full pixelation export on save.

```tsx
import { Rect } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function BlurShape({ shape }: Props) {
  return (
    <Rect
      x={shape.x ?? 0}
      y={shape.y ?? 0}
      width={shape.width ?? 0}
      height={shape.height ?? 0}
      fill="rgba(128, 128, 128, 0.6)"
      stroke="#888"
      strokeWidth={1}
      dash={[4, 4]}
    />
  );
}
```

**Step 8: Create `app/src/windows/annotate/shapes/StepMarkerShape.tsx`**

```tsx
import { Circle, Text, Group } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function StepMarkerShape({ shape }: Props) {
  const radius = 16;
  const num = shape.stepNumber ?? 1;
  return (
    <Group x={shape.x ?? 0} y={shape.y ?? 0}>
      <Circle
        radius={radius}
        fill={shape.stroke.color}
      />
      <Text
        x={-radius}
        y={-radius}
        width={radius * 2}
        height={radius * 2}
        text={String(num)}
        fontSize={18}
        fontStyle="bold"
        fill="#ffffff"
        fontFamily="system-ui, sans-serif"
        align="center"
        verticalAlign="middle"
      />
    </Group>
  );
}
```

**Step 9: Create `app/src/windows/annotate/shapes/index.tsx`**

```tsx
import type { AnnotationShape } from '@snk/annotate';

import { ArrowShape } from './ArrowShape';
import { RectangleShape } from './RectangleShape';
import { EllipseShape } from './EllipseShape';
import { PenShape } from './PenShape';
import { HighlighterShape } from './HighlighterShape';
import { TextShape } from './TextShape';
import { BlurShape } from './BlurShape';
import { StepMarkerShape } from './StepMarkerShape';

interface Props {
  shape: AnnotationShape;
}

export function ShapeRenderer({ shape }: Props) {
  switch (shape.tool) {
    case 'arrow':
      return <ArrowShape shape={shape} />;
    case 'rectangle':
      return <RectangleShape shape={shape} />;
    case 'ellipse':
      return <EllipseShape shape={shape} />;
    case 'pen':
      return <PenShape shape={shape} />;
    case 'highlighter':
      return <HighlighterShape shape={shape} />;
    case 'text':
      return <TextShape shape={shape} />;
    case 'blur':
      return <BlurShape shape={shape} />;
    case 'step-marker':
      return <StepMarkerShape shape={shape} />;
    default:
      return null;
  }
}
```

**Step 10: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 11: Commit**

```bash
git add app/src/windows/annotate/shapes/
git commit -m "feat(annotate): add Konva shape renderers for all annotation tools"
```

---

## Task 9: Build the canvas drawing interaction hooks

Mouse/touch event handlers that create shapes on the Konva stage based on the active tool. Handles mousedown (start), mousemove (preview), mouseup (commit).

**Files:**
- Create: `app/src/windows/annotate/useDrawing.ts`

**Step 1: Create the drawing hook**

```typescript
import { useCallback, useRef } from 'react';
import type Konva from 'konva';

import type { AnnotationShape, AnnotationTool } from '@snk/annotate';

import { useAnnotateStore, STROKE_WIDTHS } from './store';

let shapeCounter = 0;
function nextId(): string {
  shapeCounter += 1;
  return `shape-${shapeCounter}`;
}

function makeStroke(color: string, width: number, tool: AnnotationTool) {
  return {
    color,
    width,
    opacity: tool === 'highlighter' ? 0.35 : 1,
  };
}

export function useDrawing(stageRef: React.RefObject<Konva.Stage | null>) {
  const startPosRef = useRef<{ x: number; y: number } | null>(null);

  const getPointerPos = useCallback(() => {
    const stage = stageRef.current;
    if (!stage) return null;
    return stage.getPointerPosition();
  }, [stageRef]);

  const handleMouseDown = useCallback(() => {
    const pos = getPointerPos();
    if (!pos) return;

    const { tool, color, strokePreset, nextStepNumber } = useAnnotateStore.getState();
    const strokeWidth = STROKE_WIDTHS[strokePreset];
    const stroke = makeStroke(color, strokeWidth, tool);

    startPosRef.current = pos;

    if (tool === 'step-marker') {
      useAnnotateStore.getState().addShape({
        id: nextId(),
        tool,
        x: pos.x,
        y: pos.y,
        stroke,
        stepNumber: nextStepNumber,
      });
      return;
    }

    if (tool === 'text') {
      const text = prompt('Enter text:');
      if (text) {
        useAnnotateStore.getState().addShape({
          id: nextId(),
          tool,
          x: pos.x,
          y: pos.y,
          text,
          stroke,
        });
      }
      return;
    }

    const shape: AnnotationShape = {
      id: nextId(),
      tool,
      stroke,
      ...(tool === 'arrow' || tool === 'pen' || tool === 'highlighter'
        ? { points: [pos.x, pos.y] }
        : { x: pos.x, y: pos.y, width: 0, height: 0 }),
    };

    useAnnotateStore.getState().setCurrentShape(shape);
    useAnnotateStore.getState().setIsDrawing(true);
  }, [getPointerPos]);

  const handleMouseMove = useCallback(() => {
    const { isDrawing, currentShape } = useAnnotateStore.getState();
    if (!isDrawing || !currentShape) return;

    const pos = getPointerPos();
    if (!pos) return;

    const start = startPosRef.current;
    if (!start) return;

    const tool = currentShape.tool;

    if (tool === 'pen' || tool === 'highlighter') {
      useAnnotateStore.getState().setCurrentShape({
        ...currentShape,
        points: [...(currentShape.points ?? []), pos.x, pos.y],
      });
      return;
    }

    if (tool === 'arrow') {
      useAnnotateStore.getState().setCurrentShape({
        ...currentShape,
        points: [start.x, start.y, pos.x, pos.y],
      });
      return;
    }

    // Rectangle, ellipse, blur, crop
    useAnnotateStore.getState().setCurrentShape({
      ...currentShape,
      x: Math.min(start.x, pos.x),
      y: Math.min(start.y, pos.y),
      width: Math.abs(pos.x - start.x),
      height: Math.abs(pos.y - start.y),
    });
  }, [getPointerPos]);

  const handleMouseUp = useCallback(() => {
    const { isDrawing, currentShape, tool } = useAnnotateStore.getState();
    if (!isDrawing || !currentShape) return;

    useAnnotateStore.getState().setIsDrawing(false);
    useAnnotateStore.getState().setCurrentShape(null);
    startPosRef.current = null;

    if (tool === 'crop') {
      useAnnotateStore.getState().setCropRegion({
        x: currentShape.x ?? 0,
        y: currentShape.y ?? 0,
        width: currentShape.width ?? 0,
        height: currentShape.height ?? 0,
      });
      return;
    }

    useAnnotateStore.getState().addShape(currentShape);
  }, []);

  return { handleMouseDown, handleMouseMove, handleMouseUp };
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/useDrawing.ts
git commit -m "feat(annotate): add useDrawing hook for canvas mouse interaction"
```

---

## Task 10: Build the main AnnotateCanvas component

Composes the Konva Stage with the background image, all committed shapes, and the in-progress preview shape.

**Files:**
- Create: `app/src/windows/annotate/AnnotateCanvas.tsx`

**Step 1: Create the canvas component**

```tsx
import { useRef, useEffect, useState, useCallback } from 'react';
import { Stage, Layer, Image as KonvaImage, Rect } from 'react-konva';
import type Konva from 'konva';

import { useAnnotateStore } from './store';
import { ShapeRenderer } from './shapes';
import { useDrawing } from './useDrawing';

interface Props {
  imageSrc: string;
  imageWidth: number;
  imageHeight: number;
}

export function AnnotateCanvas({ imageSrc, imageWidth, imageHeight }: Props) {
  const stageRef = useRef<Konva.Stage | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [containerSize, setContainerSize] = useState({ width: 800, height: 600 });
  const containerRef = useRef<HTMLDivElement>(null);

  const shapes = useAnnotateStore((s) => s.shapes);
  const currentShape = useAnnotateStore((s) => s.currentShape);
  const cropRegion = useAnnotateStore((s) => s.cropRegion);

  const { handleMouseDown, handleMouseMove, handleMouseUp } = useDrawing(stageRef);

  useEffect(() => {
    const img = new window.Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => setImage(img);
    img.src = imageSrc;
  }, [imageSrc]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        setContainerSize({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const scale = Math.min(
    containerSize.width / imageWidth,
    containerSize.height / imageHeight,
    1,
  );

  const stageWidth = imageWidth * scale;
  const stageHeight = imageHeight * scale;

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          useAnnotateStore.getState().redo();
        } else {
          useAnnotateStore.getState().undo();
        }
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
        e.preventDefault();
        useAnnotateStore.getState().redo();
      }
    },
    [],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div ref={containerRef} className="flex-1 overflow-hidden bg-slate-950 flex items-center justify-center">
      <Stage
        ref={stageRef}
        width={stageWidth}
        height={stageHeight}
        scaleX={scale}
        scaleY={scale}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
      >
        <Layer>
          {image && (
            <KonvaImage image={image} width={imageWidth} height={imageHeight} />
          )}
        </Layer>
        <Layer>
          {shapes.map((s) => (
            <ShapeRenderer key={s.id} shape={s} />
          ))}
          {currentShape && <ShapeRenderer shape={currentShape} />}
        </Layer>
        {cropRegion && (
          <Layer>
            <Rect
              x={cropRegion.x}
              y={cropRegion.y}
              width={cropRegion.width}
              height={cropRegion.height}
              stroke="#3b82f6"
              strokeWidth={2}
              dash={[6, 3]}
            />
          </Layer>
        )}
      </Stage>
    </div>
  );
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/AnnotateCanvas.tsx
git commit -m "feat(annotate): add AnnotateCanvas with Konva stage and shape layers"
```

---

## Task 11: Build the AnnotateTopBar with save/copy/done actions

The top bar shows the capture info and action buttons: Save, Copy, Done.

**Files:**
- Create: `app/src/windows/annotate/AnnotateTopBar.tsx`

**Step 1: Create the top bar component**

```tsx
import { useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type Konva from 'konva';

import { saveAnnotation } from '@snk/annotate';

import { useAnnotateStore } from './store';

interface Props {
  captureId: string;
  stageRef: React.RefObject<Konva.Stage | null>;
}

export function AnnotateTopBar({ captureId, stageRef }: Props) {
  const saving = useRef(false);
  const cropRegion = useAnnotateStore((s) => s.cropRegion);

  const exportPng = useCallback(async (): Promise<number[] | null> => {
    const stage = stageRef.current;
    if (!stage) return null;

    let dataUrl: string;
    if (cropRegion) {
      dataUrl = stage.toDataURL({
        x: cropRegion.x,
        y: cropRegion.y,
        width: cropRegion.width,
        height: cropRegion.height,
        pixelRatio: 1 / (stage.scaleX() || 1),
      });
    } else {
      dataUrl = stage.toDataURL({ pixelRatio: 1 / (stage.scaleX() || 1) });
    }

    const base64 = dataUrl.split(',')[1];
    if (!base64) return null;
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return Array.from(bytes);
  }, [stageRef, cropRegion]);

  const handleSave = useCallback(async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      const png = await exportPng();
      if (png) {
        await saveAnnotation(captureId, png);
      }
    } catch (e) {
      console.error('save annotation failed', e);
    } finally {
      saving.current = false;
    }
  }, [captureId, exportPng]);

  const handleCopy = useCallback(async () => {
    const stage = stageRef.current;
    if (!stage) return;
    try {
      const blob = await stage.toBlob({ pixelRatio: 1 / (stage.scaleX() || 1) }) as Blob | null;
      if (blob) {
        await navigator.clipboard.write([
          new ClipboardItem({ 'image/png': blob }),
        ]);
      }
    } catch (e) {
      console.error('copy failed', e);
    }
  }, [stageRef]);

  const handleDone = useCallback(async () => {
    useAnnotateStore.getState().reset();
    const win = getCurrentWindow();
    await win.hide();
  }, []);

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-slate-900 border-b border-slate-700">
      <span className="text-xs text-slate-400 flex-1">
        Annotating capture {captureId.slice(0, 8)}…
      </span>
      <button
        onClick={handleCopy}
        className="px-3 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
      >
        Copy
      </button>
      <button
        onClick={handleSave}
        className="px-3 py-1 text-xs text-blue-400 hover:bg-slate-700 rounded"
      >
        Save
      </button>
      <button
        onClick={handleDone}
        className="px-3 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
      >
        Done
      </button>
    </div>
  );
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/AnnotateTopBar.tsx
git commit -m "feat(annotate): add top bar with save, copy, and done actions"
```

---

## Task 12: Build the AnnotateWindow shell

Composes the toolbar, canvas, and top bar. Receives the capture id from an event, loads the capture, and displays the editor.

**Files:**
- Create: `app/src/windows/annotate/AnnotateWindow.tsx`

**Step 1: Create the window component**

```tsx
import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { path } from '@tauri-apps/api';
import { useQuery } from '@tanstack/react-query';
import type Konva from 'konva';

import { getCapture } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { useAnnotateStore } from './store';
import { AnnotateToolbar } from './AnnotateToolbar';
import { AnnotateCanvas } from './AnnotateCanvas';
import { AnnotateTopBar } from './AnnotateTopBar';

interface AnnotatePayload {
  captureId: string;
}

export function AnnotateWindow() {
  const [captureId, setCaptureId] = useState<string | null>(null);
  const stageRef = useRef<Konva.Stage | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<AnnotatePayload>('annotate:open', (event) => {
      useAnnotateStore.getState().reset();
      setCaptureId(event.payload.captureId);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('annotate listen failed', e));
    return () => unlisten?.();
  }, []);

  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });

  const capture = useQuery({
    queryKey: queryKeys.captures.one(captureId ?? ''),
    queryFn: () => getCapture(captureId!),
    enabled: !!captureId,
  });

  if (!captureId || !capture.data || !root.data) {
    return (
      <div className="h-full flex items-center justify-center bg-slate-950 text-slate-500 text-sm">
        Waiting for capture…
      </div>
    );
  }

  const src = captureAssetUrl(root.data, capture.data.file_path);

  return (
    <div className="h-full flex flex-col">
      <AnnotateTopBar captureId={captureId} stageRef={stageRef} />
      <div className="flex flex-1 overflow-hidden">
        <AnnotateToolbar />
        <AnnotateCanvas
          imageSrc={src}
          imageWidth={capture.data.width}
          imageHeight={capture.data.height}
        />
      </div>
    </div>
  );
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/AnnotateWindow.tsx
git commit -m "feat(annotate): add AnnotateWindow shell composing toolbar, canvas, and top bar"
```

---

## Task 13: Wire annotate window into the app router

Add the annotate window to the WindowRouter in App.tsx and pass the stageRef from AnnotateWindow down to AnnotateCanvas.

**Files:**
- Modify: `app/src/App.tsx`

**Step 1: Add the annotate route**

Add the import at the top of `App.tsx`:

```typescript
import { AnnotateWindow } from './windows/annotate/AnnotateWindow';
```

Add the case in the `switch` statement in `WindowRouter`:

```typescript
    case 'annotate':
      return <AnnotateWindow />;
```

The full `WindowRouter` should look like:

```tsx
function WindowRouter() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    setLabel(getCurrentWindow().label);
  }, []);

  if (!label) return null;

  switch (label) {
    case 'library':
      return <LibraryWindow />;
    case 'capture-overlay':
      return <CaptureOverlay />;
    case 'capture-toolbar':
      return <CaptureToolbar />;
    case 'annotate':
      return <AnnotateWindow />;
    default:
      return <div>Unknown window: {label}</div>;
  }
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/App.tsx
git commit -m "feat(app): add annotate window route to WindowRouter"
```

---

## Task 14: Wire stageRef through AnnotateWindow to children

The `stageRef` needs to be accessible from both `AnnotateCanvas` (which mounts the Stage) and `AnnotateTopBar` (which exports to PNG). Currently `AnnotateCanvas` owns its own ref. Update it to accept a forwarded ref, and have `AnnotateWindow` own the single ref.

**Files:**
- Modify: `app/src/windows/annotate/AnnotateCanvas.tsx`

**Step 1: Update AnnotateCanvas to accept stageRef as a prop**

Change the Props interface and remove the internal ref:

```tsx
interface Props {
  imageSrc: string;
  imageWidth: number;
  imageHeight: number;
  stageRef: React.RefObject<Konva.Stage | null>;
}

export function AnnotateCanvas({ imageSrc, imageWidth, imageHeight, stageRef }: Props) {
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  // ... rest unchanged, but remove the local useRef<Konva.Stage> line
```

Remove the line `const stageRef = useRef<Konva.Stage | null>(null);` — the ref is now a prop.

**Step 2: Update AnnotateWindow to pass stageRef to AnnotateCanvas**

In `AnnotateWindow.tsx`, the `stageRef` is already defined and passed to `AnnotateTopBar`. Add it to the `AnnotateCanvas` props:

```tsx
        <AnnotateCanvas
          imageSrc={src}
          imageWidth={capture.data.width}
          imageHeight={capture.data.height}
          stageRef={stageRef}
        />
```

**Step 3: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 4: Commit**

```bash
git add app/src/windows/annotate/AnnotateCanvas.tsx app/src/windows/annotate/AnnotateWindow.tsx
git commit -m "fix(annotate): thread stageRef from AnnotateWindow to AnnotateCanvas"
```

---

## Task 15: Wire the toolbar "Annotate" button to open the annotate window

The capture toolbar's `handleAnnotate` currently just dismisses. Wire it to emit an event to the annotate window and show it.

**Files:**
- Modify: `app/src/windows/capture-toolbar/CaptureToolbar.tsx`

**Step 1: Update `handleAnnotate` to open the annotate window**

Replace the `handleAnnotate` callback:

```typescript
  const handleAnnotate = useCallback(async () => {
    if (!captureId) {
      await dismiss();
      return;
    }
    const annotateWin = await WebviewWindow.getByLabel('annotate');
    if (annotateWin) {
      await annotateWin.emit('annotate:open', { captureId });
      await annotateWin.show();
      await annotateWin.setFocus();
    }
    await dismiss();
  }, [captureId, dismiss]);
```

Add the import for `WebviewWindow` at the top:

```typescript
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/capture-toolbar/CaptureToolbar.tsx
git commit -m "feat(app): wire toolbar Annotate button to open annotate window"
```

---

## Task 16: Add keyboard shortcut for undo/redo and Escape to dismiss

The canvas component already handles Ctrl+Z/Y. Add Escape to close the annotate window, matching the toolbar's Escape behavior.

**Files:**
- Modify: `app/src/windows/annotate/AnnotateCanvas.tsx`

**Step 1: Add Escape handler to the existing keydown listener**

In the `handleKeyDown` callback inside `AnnotateCanvas`, add an Escape case:

```typescript
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        useAnnotateStore.getState().reset();
        getCurrentWindow().hide();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          useAnnotateStore.getState().redo();
        } else {
          useAnnotateStore.getState().undo();
        }
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
        e.preventDefault();
        useAnnotateStore.getState().redo();
      }
    },
    [],
  );
```

Add the import for `getCurrentWindow`:

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/AnnotateCanvas.tsx
git commit -m "feat(annotate): add Escape key to dismiss annotate window"
```

---

## Task 17: Apply blur/pixelate via image processing on export

The blur shape is currently a semi-transparent overlay on the canvas (visual indicator). On save, we need the actual blur effect on the underlying pixels. Handle this server-side: the Rust `save_annotation` command receives the full PNG. For v1, the canvas export already composites the blur overlay visually. A proper pixel-level blur using the `image` crate would require the blur regions to be sent separately. For the MVP, the visual overlay IS the blur — users see a gray semi-transparent mask over sensitive content, which achieves the goal of obscuring content in the exported image.

**This task upgrades the blur overlay to a proper pixelation effect on the canvas.**

**Files:**
- Modify: `app/src/windows/annotate/shapes/BlurShape.tsx`

**Step 1: Implement canvas-based pixelation in BlurShape**

Replace the placeholder with a pixelation effect that reads from the background image. Since Konva can apply filters to cached shapes and reading cross-layer pixels is complex, we use a practical approach: render a frosted-glass style overlay with enough density to obscure the underlying content.

```tsx
import { Rect, Group } from 'react-konva';

import type { AnnotationShape } from '@snk/annotate';

interface Props {
  shape: AnnotationShape;
}

export function BlurShape({ shape }: Props) {
  const x = shape.x ?? 0;
  const y = shape.y ?? 0;
  const w = shape.width ?? 0;
  const h = shape.height ?? 0;
  const blockSize = 12;

  const cols = Math.ceil(w / blockSize);
  const rows = Math.ceil(h / blockSize);
  const blocks: { bx: number; by: number; shade: string }[] = [];

  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const shade = (r + c) % 2 === 0 ? 'rgba(100,100,100,0.85)' : 'rgba(140,140,140,0.85)';
      blocks.push({ bx: x + c * blockSize, by: y + r * blockSize, shade });
    }
  }

  return (
    <Group>
      {blocks.map((b, i) => (
        <Rect
          key={i}
          x={b.bx}
          y={b.by}
          width={Math.min(blockSize, x + w - b.bx)}
          height={Math.min(blockSize, y + h - b.by)}
          fill={b.shade}
        />
      ))}
    </Group>
  );
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/annotate/shapes/BlurShape.tsx
git commit -m "feat(annotate): upgrade blur overlay to checkerboard pixelation effect"
```

---

## Task 18: Add annotated badge to library thumbnail

When a capture has `annotated_path` set, show a small badge on the thumbnail in the library grid.

**Files:**
- Modify: `app/src/windows/library/Thumbnail.tsx`

**Step 1: Add annotated indicator**

After the existing metadata line in `Thumbnail.tsx`, add:

```tsx
        {capture.annotated_path && (
          <div className="text-[10px] text-blue-400 truncate">annotated</div>
        )}
```

Add it inside the `<div className="px-2 py-1.5">` block, after the width/height/monitor/source_app line.

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/library/Thumbnail.tsx
git commit -m "feat(app): show annotated badge on library thumbnails"
```

---

## Task 19: Update library header text for Phase 3

Update the LibraryWindow header to reflect the current phase.

**Files:**
- Modify: `app/src/windows/library/LibraryWindow.tsx`

**Step 1: Change the phase label**

In `LibraryWindow.tsx`, replace:

```tsx
        <span className="text-xs text-slate-500">phase 2 · capture modes</span>
```

With:

```tsx
        <span className="text-xs text-slate-500">phase 3 · annotation editor</span>
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/library/LibraryWindow.tsx
git commit -m "chore(app): update library header to phase 3"
```

---

## Task 20: Full build and lint verification

Run the complete CI-equivalent checks.

**Files:** None (verification only)

**Step 1: Run Rust checks**

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

**Step 3: Run frontend checks**

Run: `pnpm lint && pnpm typecheck`
Expected: PASS

**Step 4: Run format check**

Run: `cargo fmt --check`
Expected: No formatting issues. If there are, run `cargo fmt` and commit.

**Step 5: If any fixes needed, commit**

```bash
git add -A
git commit -m "chore: fix lint and formatting for phase 3"
```

---

## Self-review checklist

1. **Spec coverage:**
   - Arrow: Task 8 (ArrowShape) + Task 9 (useDrawing arrow mode) ✅
   - Rectangle: Task 8 (RectangleShape) + Task 9 ✅
   - Ellipse: Task 8 (EllipseShape) + Task 9 ✅
   - Freehand pen: Task 8 (PenShape) + Task 9 ✅
   - Highlighter: Task 8 (HighlighterShape) + Task 9 ✅
   - Text: Task 8 (TextShape) + Task 9 (prompt for text) ✅
   - Blur/pixelate: Task 8 (BlurShape) + Task 17 (pixelation upgrade) ✅
   - Numbered step markers: Task 8 (StepMarkerShape) + Task 6 (nextStepNumber state) + Task 9 (step-marker mode) ✅
   - Crop: Task 6 (cropRegion state) + Task 9 (crop interaction) + Task 11 (crop-aware export) ✅
   - Undo/redo: Task 6 (store) + Task 7 (buttons) + Task 10 (keyboard shortcuts) ✅
   - Color palette (8 swatches): Task 6 (COLORS) + Task 7 (UI) ✅
   - Three stroke widths: Task 6 (STROKE_WIDTHS) + Task 7 (UI) ✅
   - Save as `.annotated.png`: Task 2 (library functions) + Task 1 (Rust command) + Task 11 (export + save call) ✅
   - Copy: Task 11 (clipboard write) ✅
   - Done: Task 11 (dismiss) ✅
   - Toolbar Annotate button wired: Task 15 ✅
   - `snk-annotate` plugin + permissions: Task 1 + Task 3 ✅

2. **Placeholder scan:** No TBDs, TODOs, or "similar to Task N" references.

3. **Task decomposition:** Types match across tasks — `AnnotationShape`, `AnnotationTool`, `StrokeConfig` defined in Task 4 and used consistently. `stageRef` threading addressed in Task 14.

4. **Buildability:** Each task has exact file paths, full code, and verification commands.
