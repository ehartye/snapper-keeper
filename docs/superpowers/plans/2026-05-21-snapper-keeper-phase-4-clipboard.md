# snapper-keeper — Phase 4: Clipboard Plugin & Popup

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Add clipboard history management with a caret-anchored popup, dedup-aware persistence, sensitive-content filtering, and auto-paste into the previously-focused app via synthetic input.

**Architecture:** A new `snk-clipboard` Rust plugin crate owns the clipboard watcher (polling `arboard` on a background thread), dedup hashing (SHA-256), sensitive-content detection (OS clipboard flags), paste synthesis (`SendInput` on Windows, `CGEventPost` on macOS), and caret/cursor position resolution. A new DB migration (`V002__clipboard_items.sql`) adds the `clipboard_items` table. A paired TS package (`packages/snk-clipboard/`) exports typed bindings. A new `clipboard-popup` Tauri window renders the popup UI (380px wide, type-to-filter, arrow-key nav, number-key shortcuts). The `Ctrl/Cmd+Shift+V` hotkey is registered in `snk-hotkeys` to trigger the popup. All persistence flows through `snk-library` — `snk-clipboard` calls library functions, never touches SQLite directly.

**Tech Stack:** Same workspace. New: `arboard` crate (clipboard read/write), `sha2` crate (already in workspace for content hashing), `windows` crate (Windows API for `GetGUIThreadInfo`, `GetCaretPos`, `SendInput`), `core-graphics` + `core-foundation` (macOS caret + paste synthesis). New TS deps: none.

**Phase 4 scope (in):**
- Clipboard watcher polling arboard on a background thread (500ms interval)
- Content dedup via SHA-256 (text normalized, image raw bytes, file canonicalized path)
- Sensitive-content detection (macOS ConcealedType/TransientType, Windows ExcludeClipboardContentFromMonitoring)
- DB migration V002 for `clipboard_items` table
- Library CRUD: insert, list, get, find-by-hash, bump-timestamp, evict-unpinned, toggle-pin
- Clipboard popup window (380px, caret-anchored with cursor fallback)
- Type-to-filter, arrow-key nav, number-key shortcuts (1-9), Enter to paste
- Auto-paste: set clipboard → focus previous window → synthesize Ctrl/Cmd+V
- Caret position resolution (Win: GetGUIThreadInfo, Mac: AXUIElement, fallback: cursor)
- `snk-clipboard` skips its own writes (self-origin tag)
- Hotkey `Ctrl/Cmd+Shift+V` registered in snk-hotkeys
- Tray menu entry for clipboard history
- Eviction: oldest unpinned items beyond history_size (default 200)

**Out of scope (later phases):**
- Per-app blocklist UI (settings phase 6 — hardcode common password managers for v1)
- Clipboard FTS5 search (phase 5 with OCR)
- File-kind clipboard items (v1 handles text + image only; file-path clipboard is deferred)
- Clipboard section in library sidebar (phase 6)

---

## Pre-flight

You are building on `main` which has phases 1-3 complete. Create a worktree on a `feature/phase-4-clipboard` branch.

**Verify before starting:**

```bash
rustc --version        # 1.78+
node --version         # 20+
pnpm --version         # 9+
cargo test             # all green
pnpm lint && pnpm typecheck  # all green
```

---

## Task 1: Add V002 migration for clipboard_items table

**Files:**
- Create: `crates/snk-library/migrations/V002__clipboard_items.sql`
- Modify: `crates/snk-library/src/migrate.rs`

**Step 1: Create `crates/snk-library/migrations/V002__clipboard_items.sql`**

```sql
CREATE TABLE clipboard_items (
    id                  TEXT PRIMARY KEY,          -- uuid v7
    kind                TEXT NOT NULL,             -- 'text' or 'image'
    text_content        TEXT,                      -- inline for text <= 8KB
    file_path           TEXT,                      -- relative path for images / large text
    content_hash        TEXT NOT NULL,             -- sha256 for dedup
    source_app          TEXT,
    source_window_title TEXT,
    created_at          INTEGER NOT NULL,          -- unix ms
    pinned              INTEGER NOT NULL DEFAULT 0,
    sensitive           INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_clipboard_items_created_at ON clipboard_items(created_at DESC);
CREATE INDEX idx_clipboard_items_hash ON clipboard_items(content_hash);
```

**Step 2: Register V002 in `crates/snk-library/src/migrate.rs`**

Add the V002 constant and include it in the migrations vec:

```rust
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V001), M::up(V002)])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 2,
            recoverable: e.to_string().contains("Backup"),
        })?;
    Ok(())
}
```

**Step 3: Write a test for V002**

Add a new test in `migrate.rs`:

```rust
    #[test]
    fn v002_creates_clipboard_items_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("apply migrations");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='clipboard_items'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "clipboard_items table should exist");
    }
```

**Step 4: Run tests**

Run: `cargo test -p snk-library`
Expected: All tests pass including new V002 test. Existing V001 tests remain green.

**Step 5: Commit**

```bash
git add crates/snk-library/migrations/V002__clipboard_items.sql crates/snk-library/src/migrate.rs
git commit -m "feat(library): add V002 migration for clipboard_items table"
```

---

## Task 2: Add clipboard_items CRUD to snk-library

Library functions for clipboard item persistence: insert, get, list, find-by-hash, bump-timestamp, evict-unpinned, toggle-pin.

**Files:**
- Create: `crates/snk-library/src/clipboard.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write failing tests**

Create `crates/snk-library/src/clipboard.rs` with the test module first:

```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardItem {
    pub id: String,
    pub kind: String,
    pub text_content: Option<String>,
    pub file_path: Option<String>,
    pub content_hash: String,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
    pub created_at: i64,
    pub pinned: bool,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewClipboardItem {
    pub kind: String,
    pub text_content: Option<String>,
    pub file_path: Option<PathBuf>,
    pub content_hash: String,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListClipboardQuery {
    pub limit: Option<u32>,
    pub filter: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Db {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        std::mem::forget(dir);
        Db::open(&path).unwrap()
    }

    fn sample_item(hash: &str) -> NewClipboardItem {
        NewClipboardItem {
            kind: "text".into(),
            text_content: Some("hello world".into()),
            file_path: None,
            content_hash: hash.into(),
            source_app: Some("Firefox".into()),
            source_window_title: Some("GitHub".into()),
        }
    }

    #[test]
    fn insert_and_get() {
        let db = fresh_db();
        let item = insert(&db, sample_item("abc123")).unwrap();
        assert_eq!(item.kind, "text");
        assert_eq!(item.content_hash, "abc123");
        assert!(!item.pinned);

        let fetched = get(&db, &item.id).unwrap();
        assert_eq!(fetched, item);
    }

    #[test]
    fn find_by_hash_returns_existing() {
        let db = fresh_db();
        let item = insert(&db, sample_item("hash1")).unwrap();
        let found = find_by_hash(&db, "hash1").unwrap();
        assert_eq!(found.unwrap().id, item.id);
    }

    #[test]
    fn find_by_hash_returns_none_for_missing() {
        let db = fresh_db();
        let found = find_by_hash(&db, "no-such-hash").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn bump_timestamp_updates_created_at() {
        let db = fresh_db();
        let item = insert(&db, sample_item("bump1")).unwrap();
        let old_ts = item.created_at;
        std::thread::sleep(std::time::Duration::from_millis(2));
        bump_timestamp(&db, &item.id).unwrap();
        let updated = get(&db, &item.id).unwrap();
        assert!(updated.created_at > old_ts);
    }

    #[test]
    fn list_returns_newest_first() {
        let db = fresh_db();
        let a = insert(&db, sample_item("a")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = insert(&db, sample_item("b")).unwrap();

        let items = list(&db, ListClipboardQuery::default()).unwrap();
        assert_eq!(items[0].id, b.id);
        assert_eq!(items[1].id, a.id);
    }

    #[test]
    fn list_filters_by_text_content() {
        let db = fresh_db();
        let mut item1 = sample_item("f1");
        item1.text_content = Some("rust programming".into());
        insert(&db, item1).unwrap();

        let mut item2 = sample_item("f2");
        item2.text_content = Some("python scripting".into());
        insert(&db, item2).unwrap();

        let items = list(
            &db,
            ListClipboardQuery {
                limit: None,
                filter: Some("rust".into()),
            },
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content_hash, "f1");
    }

    #[test]
    fn toggle_pin() {
        let db = fresh_db();
        let item = insert(&db, sample_item("pin1")).unwrap();
        assert!(!item.pinned);

        set_pinned(&db, &item.id, true).unwrap();
        let updated = get(&db, &item.id).unwrap();
        assert!(updated.pinned);

        set_pinned(&db, &item.id, false).unwrap();
        let updated = get(&db, &item.id).unwrap();
        assert!(!updated.pinned);
    }

    #[test]
    fn evict_unpinned_removes_oldest_beyond_limit() {
        let db = fresh_db();
        let a = insert(&db, sample_item("ev1")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = insert(&db, sample_item("ev2")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let c = insert(&db, sample_item("ev3")).unwrap();

        // Pin the oldest
        set_pinned(&db, &a.id, true).unwrap();

        // Evict to keep only 2 unpinned — should remove b (oldest unpinned)
        evict_unpinned(&db, 1).unwrap();

        let items = list(&db, ListClipboardQuery { limit: None, filter: None }).unwrap();
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&a.id.as_str())); // pinned, kept
        assert!(ids.contains(&c.id.as_str())); // newest unpinned, kept
        assert!(!ids.contains(&b.id.as_str())); // oldest unpinned, evicted
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p snk-library`
Expected: FAIL — functions not implemented yet.

**Step 3: Implement the functions**

Add the implementations above the `#[cfg(test)]` block in `clipboard.rs`:

```rust
fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardItem> {
    Ok(ClipboardItem {
        id: row.get("id")?,
        kind: row.get("kind")?,
        text_content: row.get("text_content")?,
        file_path: row.get("file_path")?,
        content_hash: row.get("content_hash")?,
        source_app: row.get("source_app")?,
        source_window_title: row.get("source_window_title")?,
        created_at: row.get("created_at")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        sensitive: row.get::<_, i64>("sensitive")? != 0,
    })
}

pub fn insert(db: &Db, new: NewClipboardItem) -> Result<ClipboardItem> {
    let id = Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().timestamp_millis();
    let file_path = new
        .file_path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string());

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO clipboard_items
                (id, kind, text_content, file_path, content_hash, source_app, source_window_title, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &id,
                &new.kind,
                &new.text_content,
                &file_path,
                &new.content_hash,
                &new.source_app,
                &new.source_window_title,
                created_at,
            ],
        )?;
        Ok(())
    })?;

    Ok(ClipboardItem {
        id,
        kind: new.kind,
        text_content: new.text_content,
        file_path,
        content_hash: new.content_hash,
        source_app: new.source_app,
        source_window_title: new.source_window_title,
        created_at,
        pinned: false,
        sensitive: false,
    })
}

pub fn get(db: &Db, id: &str) -> Result<ClipboardItem> {
    db.with_conn(|conn| {
        conn.query_row(
            "SELECT * FROM clipboard_items WHERE id = ?1",
            [id],
            row_to_item,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => crate::LibraryError::NotFound {
                what: format!("clipboard item {id}"),
            },
            other => other.into(),
        })
    })
}

pub fn find_by_hash(db: &Db, hash: &str) -> Result<Option<ClipboardItem>> {
    db.with_conn(|conn| {
        let result = conn.query_row(
            "SELECT * FROM clipboard_items WHERE content_hash = ?1 ORDER BY created_at DESC LIMIT 1",
            [hash],
            row_to_item,
        );
        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}

pub fn bump_timestamp(db: &Db, id: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("clipboard item {id}"),
            });
        }
        Ok(())
    })
}

pub fn list(db: &Db, q: ListClipboardQuery) -> Result<Vec<ClipboardItem>> {
    let limit = q.limit.unwrap_or(50).min(200);
    db.with_conn(|conn| {
        if let Some(ref filter) = q.filter {
            let pattern = format!("%{filter}%");
            let mut stmt = conn.prepare(
                "SELECT * FROM clipboard_items
                 WHERE (text_content LIKE ?1 OR source_app LIKE ?1)
                 ORDER BY pinned DESC, created_at DESC LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(rusqlite::params![pattern, limit], row_to_item)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        } else {
            let mut stmt = conn.prepare(
                "SELECT * FROM clipboard_items ORDER BY pinned DESC, created_at DESC LIMIT ?1",
            )?;
            let rows = stmt
                .query_map([limit], row_to_item)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        }
    })
}

pub fn set_pinned(db: &Db, id: &str, pinned: bool) -> Result<()> {
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE clipboard_items SET pinned = ?1 WHERE id = ?2",
            rusqlite::params![pinned as i64, id],
        )?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("clipboard item {id}"),
            });
        }
        Ok(())
    })
}

pub fn evict_unpinned(db: &Db, max_unpinned: u32) -> Result<u64> {
    db.with_conn(|conn| {
        let deleted = conn.execute(
            "DELETE FROM clipboard_items WHERE id IN (
                SELECT id FROM clipboard_items
                WHERE pinned = 0
                ORDER BY created_at DESC
                LIMIT -1 OFFSET ?1
            )",
            [max_unpinned],
        )?;
        Ok(deleted as u64)
    })
}
```

**Step 4: Register the module in `crates/snk-library/src/lib.rs`**

Add `pub mod clipboard;` and export the types:

```rust
pub mod captures;
pub mod clipboard;
pub mod commands;
pub mod db;
pub mod error;
pub mod files;
pub mod migrate;
pub mod plugin;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use clipboard::{ClipboardItem, ListClipboardQuery, NewClipboardItem};
pub use db::Db;
pub use error::{LibraryError, Result};
pub use plugin::{init, LibraryState};
```

**Step 5: Run tests**

Run: `cargo test -p snk-library`
Expected: All tests pass (existing + 8 new clipboard tests).

**Step 6: Commit**

```bash
git add crates/snk-library/src/clipboard.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add clipboard_items CRUD with dedup and eviction"
```

---

## Task 3: Add clipboard library commands (Tauri IPC)

Expose the clipboard library functions as Tauri commands so the frontend and `snk-clipboard` plugin can call them.

**Files:**
- Modify: `crates/snk-library/src/commands.rs`
- Modify: `crates/snk-library/src/plugin.rs`
- Modify: `crates/snk-library/build.rs`
- Modify: `crates/snk-library/permissions/default.toml`

**Step 1: Add commands in `crates/snk-library/src/commands.rs`**

Append after the existing `soft_delete_capture` command:

```rust
use crate::clipboard::{self, ClipboardItem, ListClipboardQuery};

#[tauri::command]
pub fn list_clipboard_items<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    query: Option<ListClipboardQuery>,
) -> Result<Vec<ClipboardItem>> {
    clipboard::list(&state.db, query.unwrap_or_default())
}

#[tauri::command]
pub fn get_clipboard_item<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<ClipboardItem> {
    clipboard::get(&state.db, &id)
}

#[tauri::command]
pub fn toggle_clipboard_pin<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
    pinned: bool,
) -> Result<()> {
    clipboard::set_pinned(&state.db, &id, pinned)
}
```

**Step 2: Register commands in `crates/snk-library/src/plugin.rs`**

Update the `invoke_handler`:

```rust
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-library")
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_captures,
            crate::commands::get_capture,
            crate::commands::soft_delete_capture,
            crate::commands::list_clipboard_items,
            crate::commands::get_clipboard_item,
            crate::commands::toggle_clipboard_pin,
        ])
        .setup(|app, _api| {
            let root = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("resolve app data dir: {e}"))?;
            let db_path = root.join("snapper-keeper.db");
            let db = Db::open(&db_path).map_err(|e| format!("open db: {e}"))?;
            app.manage(LibraryState {
                db: Arc::new(db),
                root,
            });
            Ok(())
        })
        .build()
}
```

**Step 3: Update `crates/snk-library/build.rs`**

```rust
const COMMANDS: &[&str] = &[
    "list_captures",
    "get_capture",
    "soft_delete_capture",
    "list_clipboard_items",
    "get_clipboard_item",
    "toggle_clipboard_pin",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 4: Update `crates/snk-library/permissions/default.toml`**

```toml
[default]
description = "Default permissions for snk-library: allows all capture and clipboard operations."
permissions = [
    "allow-list-captures",
    "allow-get-capture",
    "allow-soft-delete-capture",
    "allow-list-clipboard-items",
    "allow-get-clipboard-item",
    "allow-toggle-clipboard-pin",
]
```

**Step 5: Verify build**

Run: `cargo check -p snk-library`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/snk-library/src/commands.rs crates/snk-library/src/plugin.rs crates/snk-library/build.rs crates/snk-library/permissions/default.toml
git commit -m "feat(library): expose clipboard CRUD as Tauri commands"
```

---

## Task 4: Scaffold the `snk-clipboard` Rust plugin crate

**Files:**
- Create: `crates/snk-clipboard/Cargo.toml`
- Create: `crates/snk-clipboard/build.rs`
- Create: `crates/snk-clipboard/permissions/default.toml`
- Create: `crates/snk-clipboard/src/lib.rs`
- Create: `crates/snk-clipboard/src/error.rs`
- Create: `crates/snk-clipboard/src/plugin.rs`
- Create: `crates/snk-clipboard/src/commands.rs`
- Create: `crates/snk-clipboard/src/watcher.rs`
- Create: `crates/snk-clipboard/src/hasher.rs`
- Create: `crates/snk-clipboard/src/paste.rs`
- Create: `crates/snk-clipboard/src/caret.rs`
- Modify: `Cargo.toml` (workspace root — add member + arboard dep)

**Step 1: Add `arboard` to workspace deps in root `Cargo.toml`**

Add to `[workspace.dependencies]`:

```toml
arboard = "3"
sha2 = "0.10"
```

**Step 2: Create `crates/snk-clipboard/Cargo.toml`**

```toml
[package]
name = "snk-clipboard"
version = "0.0.1"
links = "snk-clipboard"
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
arboard.workspace = true
sha2.workspace = true
tokio.workspace = true
uuid.workspace = true
chrono.workspace = true

[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_Foundation",
] }

[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.24"
core-foundation = "0.10"
```

**Step 3: Add workspace member in root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/snk-library",
    "crates/snk-hotkeys",
    "crates/snk-capture",
    "crates/snk-annotate",
    "crates/snk-clipboard",
    "app/src-tauri",
]
```

**Step 4: Create `crates/snk-clipboard/build.rs`**

```rust
const COMMANDS: &[&str] = &["paste_item", "show_popup"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 5: Create `crates/snk-clipboard/permissions/default.toml`**

```toml
[default]
description = "Default permissions for snk-clipboard: allows paste and popup."
permissions = ["allow-paste-item", "allow-show-popup"]
```

**Step 6: Create `crates/snk-clipboard/src/error.rs`**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ClipboardError {
    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),

    #[error("clipboard access failed: {message}")]
    Access { message: String },

    #[error("paste failed: {reason}")]
    PasteFailed { reason: String },

    #[error("not found: {what}")]
    NotFound { what: String },
}

impl From<snk_library::LibraryError> for ClipboardError {
    fn from(e: snk_library::LibraryError) -> Self {
        ClipboardError::Library(e)
    }
}

impl From<arboard::Error> for ClipboardError {
    fn from(e: arboard::Error) -> Self {
        ClipboardError::Access {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ClipboardError>;
```

**Step 7: Create `crates/snk-clipboard/src/hasher.rs`**

```rust
use sha2::{Digest, Sha256};

pub fn hash_text(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut h = Sha256::new();
    h.update(normalized.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn hash_image_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_text_normalizes_whitespace() {
        let a = hash_text("  hello   world  ");
        let b = hash_text("hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_text_different_content_different_hash() {
        let a = hash_text("hello");
        let b = hash_text("world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_image_bytes_deterministic() {
        let bytes = b"fake png data";
        let a = hash_image_bytes(bytes);
        let b = hash_image_bytes(bytes);
        assert_eq!(a, b);
    }
}
```

**Step 8: Create `crates/snk-clipboard/src/caret.rs`**

```rust
#[derive(Debug, Clone, Copy)]
pub struct CaretPosition {
    pub x: i32,
    pub y: i32,
}

pub fn get_caret_position() -> Option<CaretPosition> {
    #[cfg(target_os = "windows")]
    {
        get_caret_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_caret_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

pub fn get_cursor_position() -> Option<CaretPosition> {
    #[cfg(target_os = "windows")]
    {
        get_cursor_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_cursor_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

pub fn resolve_popup_position() -> CaretPosition {
    get_caret_position()
        .or_else(get_cursor_position)
        .unwrap_or(CaretPosition { x: 100, y: 100 })
}

#[cfg(target_os = "windows")]
fn get_caret_windows() -> Option<CaretPosition> {
    use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};
    use windows::Win32::Foundation::POINT;

    unsafe {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(0, &mut info).is_ok() {
            let pt = POINT {
                x: info.rcCaret.left,
                y: info.rcCaret.bottom,
            };
            if pt.x != 0 || pt.y != 0 {
                return Some(CaretPosition { x: pt.x, y: pt.y });
            }
        }
        None
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_windows() -> Option<CaretPosition> {
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use windows::Win32::Foundation::POINT;

    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            return Some(CaretPosition { x: pt.x, y: pt.y });
        }
        None
    }
}

#[cfg(target_os = "macos")]
fn get_caret_macos() -> Option<CaretPosition> {
    // AXUIElement caret detection requires Accessibility permission.
    // Deferred to a future refinement — use cursor fallback for v1.
    None
}

#[cfg(target_os = "macos")]
fn get_cursor_macos() -> Option<CaretPosition> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let loc = event.location();
    Some(CaretPosition {
        x: loc.x as i32,
        y: loc.y as i32,
    })
}
```

**Step 9: Create `crates/snk-clipboard/src/paste.rs`**

```rust
use crate::Result;

pub fn synthesize_paste() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        synthesize_paste_windows()
    }
    #[cfg(target_os = "macos")]
    {
        synthesize_paste_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(crate::ClipboardError::PasteFailed {
            reason: "unsupported platform".into(),
        })
    }
}

#[cfg(target_os = "windows")]
fn synthesize_paste_windows() -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        VIRTUAL_KEY,
    };

    const VK_CONTROL: VIRTUAL_KEY = VIRTUAL_KEY(0x11);
    const VK_V: VIRTUAL_KEY = VIRTUAL_KEY(0x56);

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != 4 {
            return Err(crate::ClipboardError::PasteFailed {
                reason: format!("SendInput returned {sent}, expected 4"),
            });
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn synthesize_paste_macos() -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| crate::ClipboardError::PasteFailed {
            reason: "failed to create CGEventSource".into(),
        })?;

    // 'v' key is keycode 9
    let key_v: CGKeyCode = 9;

    let key_down = CGEvent::new_keyboard_event(source.clone(), key_v, true)
        .map_err(|_| crate::ClipboardError::PasteFailed {
            reason: "failed to create key-down event".into(),
        })?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);

    let key_up = CGEvent::new_keyboard_event(source, key_v, false)
        .map_err(|_| crate::ClipboardError::PasteFailed {
            reason: "failed to create key-up event".into(),
        })?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);

    key_down.post(core_graphics::event::CGEventTapLocation::HID);
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}
```

**Step 10: Create `crates/snk-clipboard/src/watcher.rs`**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arboard::Clipboard;
use tracing::{debug, error, warn};

use snk_library::clipboard::{self, NewClipboardItem};
use snk_library::{Db, files};

use crate::hasher;

static SKIP_NEXT: AtomicBool = AtomicBool::new(false);

pub fn mark_skip_next() {
    SKIP_NEXT.store(true, Ordering::SeqCst);
}

pub fn start_watcher(db: Arc<Db>, library_root: std::path::PathBuf) {
    std::thread::spawn(move || {
        let mut clip = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "failed to open clipboard for watching");
                return;
            }
        };
        let mut last_hash: Option<String> = None;

        loop {
            std::thread::sleep(Duration::from_millis(500));

            if SKIP_NEXT.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            {
                debug!("skipping own clipboard write");
                continue;
            }

            if let Ok(text) = clip.get_text() {
                if !text.is_empty() {
                    let hash = hasher::hash_text(&text);
                    if last_hash.as_deref() == Some(&hash) {
                        continue;
                    }
                    last_hash = Some(hash.clone());

                    match clipboard::find_by_hash(&db, &hash) {
                        Ok(Some(existing)) => {
                            let _ = clipboard::bump_timestamp(&db, &existing.id);
                        }
                        Ok(None) => {
                            let new_item = NewClipboardItem {
                                kind: "text".into(),
                                text_content: Some(text),
                                file_path: None,
                                content_hash: hash,
                                source_app: None,
                                source_window_title: None,
                            };
                            match clipboard::insert(&db, new_item) {
                                Ok(_) => {
                                    let _ = clipboard::evict_unpinned(&db, 200);
                                }
                                Err(e) => warn!(error = ?e, "clipboard insert failed"),
                            }
                        }
                        Err(e) => warn!(error = ?e, "clipboard hash lookup failed"),
                    }
                    continue;
                }
            }

            if let Ok(img) = clip.get_image() {
                let bytes = img.bytes.to_vec();
                if !bytes.is_empty() {
                    let hash = hasher::hash_image_bytes(&bytes);
                    if last_hash.as_deref() == Some(&hash) {
                        continue;
                    }
                    last_hash = Some(hash.clone());

                    match clipboard::find_by_hash(&db, &hash) {
                        Ok(Some(existing)) => {
                            let _ = clipboard::bump_timestamp(&db, &existing.id);
                        }
                        Ok(None) => {
                            let id = uuid::Uuid::now_v7();
                            let relative = files::clipboard_image_relative_path(&id);
                            if let Ok(_) = files::write_atomic(&library_root, &relative, &bytes) {
                                let new_item = NewClipboardItem {
                                    kind: "image".into(),
                                    text_content: None,
                                    file_path: Some(relative),
                                    content_hash: hash,
                                    source_app: None,
                                    source_window_title: None,
                                };
                                match clipboard::insert(&db, new_item) {
                                    Ok(_) => {
                                        let _ = clipboard::evict_unpinned(&db, 200);
                                    }
                                    Err(e) => warn!(error = ?e, "clipboard image insert failed"),
                                }
                            }
                        }
                        Err(e) => warn!(error = ?e, "clipboard image hash lookup failed"),
                    }
                }
            }
        }
    });
}
```

**Step 11: Create `crates/snk-clipboard/src/commands.rs`**

```rust
use std::sync::Arc;

use arboard::Clipboard;
use tauri::{Runtime, State};
use tracing::info;

use snk_library::clipboard;
use snk_library::plugin::LibraryState;

use crate::caret;
use crate::paste;
use crate::watcher;
use crate::Result;

#[tauri::command]
pub fn paste_item<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    let item = clipboard::get(&state.db, &id)?;

    let mut clip = Clipboard::new()?;

    watcher::mark_skip_next();

    if let Some(ref text) = item.text_content {
        clip.set_text(text).map_err(arboard::Error::from)?;
    }

    std::thread::sleep(std::time::Duration::from_millis(50));

    paste::synthesize_paste()?;

    clipboard::bump_timestamp(&state.db, &id)?;

    info!(id = %id, "pasted clipboard item");
    Ok(())
}

#[tauri::command]
pub fn show_popup<R: Runtime>(
    _app: tauri::AppHandle<R>,
) -> Result<crate::caret::CaretPosition> {
    let pos = caret::resolve_popup_position();
    Ok(pos)
}
```

**Step 12: Create `crates/snk-clipboard/src/plugin.rs`**

```rust
use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-clipboard")
        .invoke_handler(tauri::generate_handler![
            crate::commands::paste_item,
            crate::commands::show_popup,
        ])
        .setup(|app, _api| {
            let state: tauri::State<'_, snk_library::plugin::LibraryState> =
                app.state();
            let db = Arc::clone(&state.db);
            let root = state.root.clone();
            crate::watcher::start_watcher(db, root);
            Ok(())
        })
        .build()
}
```

**Step 13: Create `crates/snk-clipboard/src/lib.rs`**

```rust
pub mod caret;
pub mod commands;
pub mod error;
pub mod hasher;
pub mod paste;
pub mod plugin;
pub mod watcher;

pub use error::{ClipboardError, Result};
pub use plugin::init;
```

**Step 14: Add `clipboard_image_relative_path` to snk-library files.rs**

Add this function in `crates/snk-library/src/files.rs`:

```rust
pub fn clipboard_image_relative_path(id: &Uuid) -> PathBuf {
    let now = Utc::now();
    PathBuf::from("clipboard")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{id}.png"))
}
```

**Step 15: Add `Serialize` derive to `CaretPosition`**

The `show_popup` command returns `CaretPosition` which needs `Serialize`:

Already included in the `caret.rs` above — add `use serde::Serialize;` at the top and `#[derive(Debug, Clone, Copy, Serialize)]` on the struct.

**Step 16: Verify compilation**

Run: `cargo check -p snk-clipboard`
Expected: PASS (or minor fixes needed — address any type mismatches)

**Step 17: Run tests**

Run: `cargo test -p snk-clipboard`
Expected: hasher tests pass.

**Step 18: Commit**

```bash
git add crates/snk-clipboard/ Cargo.toml crates/snk-library/src/files.rs
git commit -m "feat(clipboard): scaffold snk-clipboard plugin with watcher, hasher, paste, caret"
```

---

## Task 5: Register snk-clipboard plugin in the Tauri app

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/src/main.rs`
- Modify: `app/src-tauri/tauri.conf.json`
- Modify: `app/src-tauri/capabilities/default.json`

**Step 1: Add dependency in `app/src-tauri/Cargo.toml`**

```toml
snk-clipboard = { path = "../../crates/snk-clipboard" }
```

**Step 2: Register plugin in `app/src-tauri/src/main.rs`**

Add after `snk_annotate::init()`:

```rust
        .plugin(snk_clipboard::init())
```

**Step 3: Add clipboard-popup window to `app/src-tauri/tauri.conf.json`**

Add a window entry:

```json
      {
        "label": "clipboard-popup",
        "title": "",
        "width": 380,
        "height": 480,
        "resizable": false,
        "alwaysOnTop": true,
        "decorations": false,
        "transparent": true,
        "visible": false,
        "skipTaskbar": true
      }
```

**Step 4: Update capabilities**

In `app/src-tauri/capabilities/default.json`, add `"clipboard-popup"` to windows and `"snk-clipboard:default"` to permissions:

```json
{
  "identifier": "default",
  "windows": ["library", "capture-overlay", "capture-toolbar", "annotate", "clipboard-popup"],
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
    "snk-annotate:default",
    "snk-clipboard:default"
  ]
}
```

**Step 5: Add clipboard history hotkey and tray menu entry**

In `crates/snk-hotkeys/src/lib.rs`, add the `ClipboardHistory` variant:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HotkeyAction {
    CaptureFullScreen,
    CaptureRegion,
    CaptureWindow,
    CaptureTimedFullScreen,
    ClipboardHistory,
}

impl HotkeyAction {
    pub fn event_name(self) -> &'static str {
        match self {
            HotkeyAction::CaptureFullScreen => "hotkey:capture-full-screen",
            HotkeyAction::CaptureRegion => "hotkey:capture-region",
            HotkeyAction::CaptureWindow => "hotkey:capture-window",
            HotkeyAction::CaptureTimedFullScreen => "hotkey:capture-timed",
            HotkeyAction::ClipboardHistory => "hotkey:clipboard-history",
        }
    }

    pub fn default_chord(self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            HotkeyAction::CaptureFullScreen => "Cmd+Shift+3",
            HotkeyAction::CaptureRegion => "Cmd+Shift+4",
            HotkeyAction::CaptureWindow => "Cmd+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "Cmd+Shift+6",
            HotkeyAction::ClipboardHistory => "Cmd+Shift+V",
        }
        #[cfg(not(target_os = "macos"))]
        match self {
            HotkeyAction::CaptureFullScreen => "CmdOrCtrl+Shift+3",
            HotkeyAction::CaptureRegion => "CmdOrCtrl+Shift+4",
            HotkeyAction::CaptureWindow => "CmdOrCtrl+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "CmdOrCtrl+Shift+6",
            HotkeyAction::ClipboardHistory => "CmdOrCtrl+Shift+V",
        }
    }
}
```

Add `HotkeyAction::ClipboardHistory` to the `actions` array in `register_defaults`.

In `app/src-tauri/src/main.rs`, add the clipboard history tray menu item:

After `capture_timed`, add:

```rust
            let clipboard_hist = MenuItem::with_id(
                app,
                "tray:clipboard-history",
                "Clipboard history\tCtrl+Shift+V",
                true,
                None::<&str>,
            )?;
```

Add `&clipboard_hist` between `&capture_timed` and `&sep` in the menu construction. Add a handler in `on_menu_event`:

```rust
                    "tray:clipboard-history" => {
                        let _ = app.emit("hotkey:clipboard-history", ());
                    }
```

**Step 6: Verify build**

Run: `cargo check -p snapper-keeper`
Expected: PASS

**Step 7: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/src/main.rs app/src-tauri/tauri.conf.json app/src-tauri/capabilities/default.json crates/snk-hotkeys/src/lib.rs
git commit -m "feat(app): register snk-clipboard plugin, hotkey, and tray entry"
```

---

## Task 6: Scaffold the `@snk/clipboard` TypeScript package

**Files:**
- Create: `packages/snk-clipboard/package.json`
- Create: `packages/snk-clipboard/tsconfig.json`
- Create: `packages/snk-clipboard/src/types.ts`
- Create: `packages/snk-clipboard/src/index.ts`
- Modify: `app/package.json`

**Step 1: Create `packages/snk-clipboard/package.json`**

```json
{
  "name": "@snk/clipboard",
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

**Step 2: Create `packages/snk-clipboard/tsconfig.json`**

```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

**Step 3: Create `packages/snk-clipboard/src/types.ts`**

```typescript
export interface ClipboardItem {
  id: string;
  kind: string;
  text_content: string | null;
  file_path: string | null;
  content_hash: string;
  source_app: string | null;
  source_window_title: string | null;
  created_at: number;
  pinned: boolean;
  sensitive: boolean;
}

export interface ListClipboardQuery {
  limit?: number;
  filter?: string;
}

export interface CaretPosition {
  x: number;
  y: number;
}
```

**Step 4: Create `packages/snk-clipboard/src/index.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

import type { ClipboardItem, ListClipboardQuery, CaretPosition } from './types';

export * from './types';

export const CLIPBOARD_HISTORY_EVENT = 'hotkey:clipboard-history';

export function listClipboardItems(query?: ListClipboardQuery): Promise<ClipboardItem[]> {
  return invoke<ClipboardItem[]>('plugin:snk-library|list_clipboard_items', { query });
}

export function getClipboardItem(id: string): Promise<ClipboardItem> {
  return invoke<ClipboardItem>('plugin:snk-library|get_clipboard_item', { id });
}

export function toggleClipboardPin(id: string, pinned: boolean): Promise<void> {
  return invoke<void>('plugin:snk-library|toggle_clipboard_pin', { id, pinned });
}

export function pasteItem(id: string): Promise<void> {
  return invoke<void>('plugin:snk-clipboard|paste_item', { id });
}

export function showPopup(): Promise<CaretPosition> {
  return invoke<CaretPosition>('plugin:snk-clipboard|show_popup');
}
```

**Step 5: Add dependency in `app/package.json`**

Add to `dependencies`:

```json
    "@snk/clipboard": "workspace:*",
```

**Step 6: Install deps and verify**

Run: `pnpm install && pnpm typecheck`
Expected: PASS

**Step 7: Commit**

```bash
git add packages/snk-clipboard/ app/package.json pnpm-lock.yaml
git commit -m "feat(clipboard): scaffold @snk/clipboard TS package with bindings"
```

---

## Task 7: Build the clipboard popup Zustand store

**Files:**
- Create: `app/src/windows/clipboard-popup/store.ts`

**Step 1: Create the store**

```typescript
import { create } from 'zustand';

import type { ClipboardItem } from '@snk/clipboard';

interface ClipboardPopupState {
  items: ClipboardItem[];
  filter: string;
  selectedIndex: number;

  setItems: (items: ClipboardItem[]) => void;
  setFilter: (filter: string) => void;
  setSelectedIndex: (index: number) => void;
  moveSelection: (delta: number) => void;
  reset: () => void;
}

const initialState = {
  items: [] as ClipboardItem[],
  filter: '',
  selectedIndex: 0,
};

export const useClipboardPopupStore = create<ClipboardPopupState>((set, get) => ({
  ...initialState,

  setItems: (items) => set({ items, selectedIndex: 0 }),
  setFilter: (filter) => set({ filter, selectedIndex: 0 }),
  setSelectedIndex: (index) => set({ selectedIndex: index }),

  moveSelection: (delta) => {
    const { items, selectedIndex } = get();
    if (items.length === 0) return;
    const next = Math.max(0, Math.min(items.length - 1, selectedIndex + delta));
    set({ selectedIndex: next });
  },

  reset: () => set(initialState),
}));
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/clipboard-popup/store.ts
git commit -m "feat(clipboard): add Zustand store for clipboard popup state"
```

---

## Task 8: Build the ClipboardPopupItem component

**Files:**
- Create: `app/src/windows/clipboard-popup/ClipboardPopupItem.tsx`

**Step 1: Create the component**

```tsx
import type { ClipboardItem } from '@snk/clipboard';

interface Props {
  item: ClipboardItem;
  index: number;
  isSelected: boolean;
  onSelect: (id: string) => void;
}

function timeAgo(ms: number): string {
  const sec = Math.floor((Date.now() - ms) / 1000);
  if (sec < 60) return 'just now';
  if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
  if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
  return `${Math.floor(sec / 86400)}d ago`;
}

export function ClipboardPopupItem({ item, index, isSelected, onSelect }: Props) {
  const preview =
    item.kind === 'text'
      ? (item.text_content ?? '').slice(0, 120)
      : '[image]';

  return (
    <button
      onClick={() => onSelect(item.id)}
      className={`w-full text-left px-3 py-2 flex items-start gap-2 ${
        isSelected ? 'bg-blue-600/30' : 'hover:bg-slate-800'
      }`}
    >
      <span className="text-[10px] text-slate-500 w-4 shrink-0 text-right pt-0.5">
        {index < 9 ? index + 1 : ''}
      </span>
      <span className="text-xs text-slate-500 w-4 shrink-0 pt-0.5">
        {item.kind === 'text' ? 'T' : 'I'}
      </span>
      <div className="flex-1 min-w-0">
        <div className="text-xs text-slate-200 truncate">{preview}</div>
        <div className="text-[10px] text-slate-500 truncate">
          {item.source_app ?? 'unknown'} · {timeAgo(item.created_at)}
          {item.pinned ? ' · pinned' : ''}
        </div>
      </div>
    </button>
  );
}
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/windows/clipboard-popup/ClipboardPopupItem.tsx
git commit -m "feat(clipboard): add ClipboardPopupItem component"
```

---

## Task 9: Build the ClipboardPopup window component

**Files:**
- Create: `app/src/windows/clipboard-popup/ClipboardPopup.tsx`

**Step 1: Create the component**

```tsx
import { useEffect, useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

import {
  listClipboardItems,
  pasteItem,
  toggleClipboardPin,
  CLIPBOARD_HISTORY_EVENT,
} from '@snk/clipboard';

import { useClipboardPopupStore } from './store';
import { ClipboardPopupItem } from './ClipboardPopupItem';

export function ClipboardPopup() {
  const items = useClipboardPopupStore((s) => s.items);
  const filter = useClipboardPopupStore((s) => s.filter);
  const selectedIndex = useClipboardPopupStore((s) => s.selectedIndex);
  const setItems = useClipboardPopupStore((s) => s.setItems);
  const setFilter = useClipboardPopupStore((s) => s.setFilter);
  const moveSelection = useClipboardPopupStore((s) => s.moveSelection);
  const reset = useClipboardPopupStore((s) => s.reset);
  const inputRef = useRef<HTMLInputElement>(null);

  const loadItems = useCallback(
    async (filterText?: string) => {
      try {
        const result = await listClipboardItems({
          limit: 50,
          filter: filterText || undefined,
        });
        setItems(result);
      } catch (e) {
        console.error('load clipboard items failed', e);
      }
    },
    [setItems],
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen(CLIPBOARD_HISTORY_EVENT, async () => {
      reset();
      await loadItems();
      const win = getCurrentWindow();
      await win.show();
      await win.setFocus();
      inputRef.current?.focus();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('clipboard popup listen failed', e));
    return () => unlisten?.();
  }, [loadItems, reset]);

  const dismiss = useCallback(async () => {
    reset();
    const win = getCurrentWindow();
    await win.hide();
  }, [reset]);

  const handlePaste = useCallback(
    async (id: string) => {
      try {
        await dismiss();
        await pasteItem(id);
      } catch (e) {
        console.error('paste failed', e);
      }
    },
    [dismiss],
  );

  const handleFilterChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = e.target.value;
      setFilter(val);
      loadItems(val);
    },
    [setFilter, loadItems],
  );

  const handleKeyDown = useCallback(
    async (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        await dismiss();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        moveSelection(1);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        moveSelection(-1);
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        const item = items[selectedIndex];
        if (item) {
          await handlePaste(item.id);
        }
        return;
      }
      // Number keys 1-9 for quick select
      const num = parseInt(e.key, 10);
      if (num >= 1 && num <= 9 && items[num - 1]) {
        e.preventDefault();
        await handlePaste(items[num - 1]!.id);
        return;
      }
      // Ctrl/Cmd+P to pin
      if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
        e.preventDefault();
        const item = items[selectedIndex];
        if (item) {
          await toggleClipboardPin(item.id, !item.pinned);
          await loadItems(filter);
        }
      }
    },
    [items, selectedIndex, dismiss, handlePaste, moveSelection, loadItems, filter],
  );

  return (
    <div
      className="flex flex-col h-full bg-slate-900/95 border border-slate-700 rounded-lg shadow-2xl"
      onKeyDown={handleKeyDown}
    >
      <div className="px-3 pt-3 pb-2">
        <input
          ref={inputRef}
          type="text"
          value={filter}
          onChange={handleFilterChange}
          placeholder="Type to filter..."
          className="w-full bg-slate-800 text-xs text-slate-200 px-3 py-1.5 rounded border border-slate-600 outline-none focus:border-blue-500"
        />
      </div>
      <div className="flex-1 overflow-y-auto">
        {items.length === 0 ? (
          <div className="px-3 py-4 text-xs text-slate-500 text-center">
            No clipboard items
          </div>
        ) : (
          items.map((item, i) => (
            <ClipboardPopupItem
              key={item.id}
              item={item}
              index={i}
              isSelected={i === selectedIndex}
              onSelect={handlePaste}
            />
          ))
        )}
      </div>
      <div className="px-3 py-1.5 border-t border-slate-700 text-[10px] text-slate-500 flex gap-3">
        <span>↑↓ nav</span>
        <span>Enter paste</span>
        <span>1-9 jump</span>
        <span>Ctrl+P pin</span>
        <span>Esc close</span>
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
git add app/src/windows/clipboard-popup/ClipboardPopup.tsx
git commit -m "feat(clipboard): add ClipboardPopup window with filter, nav, and paste"
```

---

## Task 10: Wire clipboard popup into the app router

**Files:**
- Modify: `app/src/App.tsx`

**Step 1: Add the import and route**

Add import:

```typescript
import { ClipboardPopup } from './windows/clipboard-popup/ClipboardPopup';
```

Add case in WindowRouter switch:

```typescript
    case 'clipboard-popup':
      return <ClipboardPopup />;
```

**Step 2: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 3: Commit**

```bash
git add app/src/App.tsx
git commit -m "feat(app): add clipboard-popup route to WindowRouter"
```

---

## Task 11: Wire hotkey event to show clipboard popup in LibraryWindow

**Files:**
- Modify: `app/src/windows/library/LibraryWindow.tsx`

**Step 1: Add clipboard history event listener**

Import the event constant:

```typescript
import { CLIPBOARD_HISTORY_EVENT } from '@snk/clipboard';
```

Add `showPopup` import from `@snk/clipboard`:

```typescript
import { CLIPBOARD_HISTORY_EVENT, showPopup } from '@snk/clipboard';
```

Add the handler and listener. After the existing `handleTimed` callback:

```typescript
  const handleClipboardHistory = useCallback(async () => {
    try {
      const pos = await showPopup();
      const popup = await WebviewWindow.getByLabel('clipboard-popup');
      if (popup) {
        await popup.setPosition(new LogicalPosition(pos.x, pos.y));
        await popup.emit(CLIPBOARD_HISTORY_EVENT, {});
        await popup.show();
        await popup.setFocus();
      }
    } catch (e) {
      console.error('clipboard popup failed', e);
    }
  }, []);
```

Add `LogicalPosition` import:

```typescript
import { LogicalPosition } from '@tauri-apps/api/dpi';
```

Register the listener in the `useEffect` setup function:

```typescript
      unlisteners.push(await listen(CLIPBOARD_HISTORY_EVENT, handleClipboardHistory));
```

Add `handleClipboardHistory` to the useEffect dependency array.

**Step 2: Update the phase label**

```tsx
        <span className="text-xs text-slate-500">phase 4 · clipboard</span>
```

**Step 3: Verify typecheck**

Run: `pnpm typecheck`
Expected: PASS

**Step 4: Commit**

```bash
git add app/src/windows/library/LibraryWindow.tsx
git commit -m "feat(app): wire clipboard history hotkey to show popup window"
```

---

## Task 12: Full build and lint verification

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
git commit -m "chore: fix lint and formatting for phase 4"
```

---

## Self-review checklist

1. **Spec coverage:**
   - Clipboard watcher (polling arboard): Task 4 (watcher.rs) ✅
   - Dedup via SHA-256 (text normalized, image raw): Task 4 (hasher.rs + watcher.rs) ✅
   - DB migration V002 clipboard_items: Task 1 ✅
   - Library CRUD (insert, get, list, find-by-hash, bump, evict, pin): Task 2 ✅
   - Library Tauri commands: Task 3 ✅
   - `snk-clipboard` plugin crate: Task 4 ✅
   - Caret position (Win: GetGUIThreadInfo, Mac: cursor fallback): Task 4 (caret.rs) ✅
   - Paste synthesis (Win: SendInput, Mac: CGEventPost): Task 4 (paste.rs) ✅
   - Self-skip on own writes: Task 4 (watcher.rs mark_skip_next) ✅
   - Clipboard popup window (380px, caret-anchored): Task 5 (window config) + Task 9 (UI) ✅
   - Type-to-filter: Task 9 (filter input) ✅
   - Arrow-key nav: Task 9 (ArrowUp/Down handlers) ✅
   - Number-key shortcuts (1-9): Task 9 ✅
   - Enter to paste: Task 9 ✅
   - Ctrl+P to pin: Task 9 ✅
   - Esc to close: Task 9 ✅
   - Hotkey Ctrl/Cmd+Shift+V: Task 5 (snk-hotkeys) ✅
   - Tray menu entry: Task 5 ✅
   - Eviction of unpinned beyond history_size: Task 2 (evict_unpinned) + Task 4 (called in watcher) ✅
   - TS bindings: Task 6 ✅
   - Router wiring: Task 10 ✅
   - LibraryWindow integration: Task 11 ✅

2. **Placeholder scan:** No TBDs, TODOs, or "similar to Task N" references.

3. **Task decomposition:** Types match — `ClipboardItem`, `ListClipboardQuery`, `CaretPosition` consistent across Tasks 2/3/4/6. Function names match between library functions and commands.

4. **Buildability:** Each task has exact file paths, full code, and verification commands.

**Deferred from spec (intentionally scoped out):**
- Sensitive-content detection (macOS pasteboard flags, Windows clipboard formats) — deferred to a refinement; the watcher structure supports adding the check in the polling loop
- File-kind clipboard items — v1 handles text + image only
- Per-app blocklist — hardcode deferred to settings phase
- Clipboard FTS5 — phase 5 with OCR
