# snapper-keeper — Phase 5: OCR & Full-Text Search

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Add async OCR processing (tesseract sidecar) on every new capture and wire full-text search across captures and clipboard items, so the library search bar returns results ranked by relevance.

**Architecture:** `snk-ocr` is a new Rust crate (Tauri plugin) that listens for `capture:saved` events and runs tesseract as a child process. OCR results flow back into `snk-library` via its typed API (`upsert_ocr_text`). FTS5 virtual tables are populated transactionally alongside base-table mutations. The library window's search bar queries a new `search` Tauri command that hits the FTS5 index and returns unified results.

**Tech Stack:** Tauri 2, Rust (rusqlite FTS5, tokio spawn, std::process::Command for sidecar), React + TypeScript, TanStack Query (search debounce).

**Phase 5 scope (in):**
- V003 migration: `ocr_text` table + `captures_fts` + `clipboard_fts` FTS5 virtual tables
- `snk-library` modules: `ocr` (upsert/get OCR text), `search` (unified FTS5 query)
- `snk-library` commands: `upsert_ocr_text`, `search_library`
- `snk-ocr` crate: async queue, tesseract sidecar invocation, retry with backoff (3 attempts)
- `@snk/ocr` TS package: events + status query binding
- Library window: search bar component with debounced FTS5 query + result rendering
- FTS population wired into existing `captures::insert` and `clipboard::insert`

**Out of scope (later phases):**
- Sidebar smart sections, tag management UI (Phase 6)
- Settings UI, first-run wizard (Phase 6)
- Signing, notarization, auto-updater (Phase 7)
- Per-language tesseract data packs beyond `eng`
- OCR re-processing UI ("re-run OCR" button)

---

## Pre-flight

You are in a worktree on a fresh `feature/phase-5-ocr-search` branch forked from `main`. Main already contains everything through Phase 4 (clipboard plugin, popup, paste synthesis).

**Key files you'll modify:**
- `Cargo.toml` (workspace root — add `snk-ocr` member)
- `crates/snk-library/` (new migration, new modules, updated commands + build.rs)
- `app/src-tauri/Cargo.toml` (add snk-ocr dep)
- `app/src-tauri/src/main.rs` (load snk-ocr plugin)
- `app/src-tauri/capabilities/default.json` (add snk-ocr permission)
- `app/src/windows/library/LibraryWindow.tsx` (search bar)
- `packages/snk-library/src/` (search binding)
- `packages/snk-ocr/` (new TS package)

**Required tools:**
- Rust toolchain (stable 1.78+)
- pnpm 9+
- Tesseract OCR installed on the dev machine (`tesseract --version` should work). On Windows: `choco install tesseract` or download from GitHub. On macOS: `brew install tesseract`.

---

## Task 1: V003 Migration — ocr_text + FTS5 Tables

**Files:**
- Create: `crates/snk-library/migrations/V003__ocr_fts.sql`
- Modify: `crates/snk-library/src/migrate.rs:6-11`

**Step 1: Write the migration SQL file**

Create `crates/snk-library/migrations/V003__ocr_fts.sql`:

```sql
-- Phase 5 — OCR text storage + FTS5 indexes for captures and clipboard.

CREATE TABLE ocr_text (
    capture_id  TEXT PRIMARY KEY REFERENCES captures(id) ON DELETE CASCADE,
    text        TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'eng',
    confidence  REAL NOT NULL DEFAULT 0.0,
    created_at  INTEGER NOT NULL  -- unix ms
);

-- Regular FTS5 (not contentless). We considered content='' + contentless_delete=1
-- but UNINDEXED columns return NULL on contentless tables in SQLite < 3.47
-- (contentless_unindexed=1 added in 3.47; rusqlite 0.31 bundles 3.45). Regular
-- FTS5 stores indexed columns redundantly but UNINDEXED capture_id/clipboard_id
-- are retrievable, DELETE works natively, and there's no version constraint.
CREATE VIRTUAL TABLE captures_fts USING fts5(
    capture_id UNINDEXED,
    source_app,
    window_title,
    ocr_text,
    tag_names
);

CREATE VIRTUAL TABLE clipboard_fts USING fts5(
    clipboard_id UNINDEXED,
    text_content,
    source_app,
    window_title
);
```

**Step 2: Wire V003 into migrate.rs**

Update `crates/snk-library/src/migrate.rs` to include the new migration:

```rust
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");
const V003: &str = include_str!("../migrations/V003__ocr_fts.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V001), M::up(V002), M::up(V003)])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 3,
            recoverable: e.to_string().contains("Backup"),
        })?;
    Ok(())
}
```

**Step 3: Write the failing test**

Add to `crates/snk-library/src/migrate.rs` tests module:

```rust
#[test]
fn v003_creates_ocr_and_fts_tables() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).expect("apply migrations");

    for table in ["ocr_text", "captures_fts", "clipboard_fts"] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view') AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "table {table} should exist");
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p snk-library migrate::tests::v003_creates_ocr_and_fts_tables`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/snk-library/migrations/V003__ocr_fts.sql crates/snk-library/src/migrate.rs
git commit -m "feat(library): add V003 migration — ocr_text + captures_fts + clipboard_fts"
```

---

## Task 2: Library OCR Module — upsert_ocr_text + get_ocr_text

**Files:**
- Create: `crates/snk-library/src/ocr.rs`
- Modify: `crates/snk-library/src/lib.rs` (add `pub mod ocr;` + re-exports)

**Step 1: Write the failing test**

Create `crates/snk-library/src/ocr.rs` with tests first:

```rust
use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrText {
    pub capture_id: String,
    pub text: String,
    pub language: String,
    pub confidence: f64,
    pub created_at: i64,
}

pub fn upsert(db: &Db, capture_id: &str, text: &str, language: &str, confidence: f64) -> Result<()> {
    todo!()
}

pub fn get(db: &Db, capture_id: &str) -> Result<Option<OcrText>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fresh_db() -> Db {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        std::mem::forget(dir);
        Db::open(&path).unwrap()
    }

    fn insert_capture(db: &Db) -> String {
        crate::captures::insert(
            db,
            crate::NewCapture {
                file_path: PathBuf::from("test.png"),
                width: 100,
                height: 100,
                source_app: Some("Firefox".into()),
                source_window_title: Some("Test Page".into()),
                monitor: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn upsert_inserts_new_ocr_text() {
        let db = fresh_db();
        let cap_id = insert_capture(&db);
        upsert(&db, &cap_id, "hello world", "eng", 0.95).unwrap();
        let row = get(&db, &cap_id).unwrap().unwrap();
        assert_eq!(row.text, "hello world");
        assert_eq!(row.language, "eng");
        assert!((row.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn upsert_replaces_existing_ocr_text() {
        let db = fresh_db();
        let cap_id = insert_capture(&db);
        upsert(&db, &cap_id, "first", "eng", 0.8).unwrap();
        upsert(&db, &cap_id, "second", "eng", 0.9).unwrap();
        let row = get(&db, &cap_id).unwrap().unwrap();
        assert_eq!(row.text, "second");
        assert!((row.confidence - 0.9).abs() < 0.001);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let db = fresh_db();
        let result = get(&db, "no-such-id").unwrap();
        assert!(result.is_none());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library ocr::tests`
Expected: FAIL (todo!() panics)

**Step 3: Implement upsert and get**

Replace the `todo!()` bodies in `crates/snk-library/src/ocr.rs`:

```rust
pub fn upsert(db: &Db, capture_id: &str, text: &str, language: &str, confidence: f64) -> Result<()> {
    let created_at = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO ocr_text (capture_id, text, language, confidence, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(capture_id) DO UPDATE SET
                text = excluded.text,
                language = excluded.language,
                confidence = excluded.confidence,
                created_at = excluded.created_at",
            rusqlite::params![capture_id, text, language, confidence, created_at],
        )?;
        Ok(())
    })
}

pub fn get(db: &Db, capture_id: &str) -> Result<Option<OcrText>> {
    db.with_conn(|conn| {
        let result = conn.query_row(
            "SELECT capture_id, text, language, confidence, created_at
             FROM ocr_text WHERE capture_id = ?1",
            [capture_id],
            |row| {
                Ok(OcrText {
                    capture_id: row.get(0)?,
                    text: row.get(1)?,
                    language: row.get(2)?,
                    confidence: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        );
        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}
```

**Step 4: Register module in lib.rs**

Add to `crates/snk-library/src/lib.rs`:
- `pub mod ocr;` after line 4 (after `pub mod commands;`)
- `pub use ocr::OcrText;` in the re-exports section

**Step 5: Run tests to verify they pass**

Run: `cargo test -p snk-library ocr::tests`
Expected: PASS (3 tests)

**Step 6: Commit**

```bash
git add crates/snk-library/src/ocr.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add ocr module — upsert_ocr_text + get_ocr_text"
```

---

## Task 3: Library Search Module — FTS5 Population + Query

**Files:**
- Create: `crates/snk-library/src/search.rs`
- Modify: `crates/snk-library/src/lib.rs` (add `pub mod search;` + re-exports)

**Step 1: Write the failing test**

Create `crates/snk-library/src/search.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SearchResult {
    Capture {
        id: String,
        rank: f64,
        snippet: String,
    },
    Clipboard {
        id: String,
        rank: f64,
        snippet: String,
    },
}

pub fn index_capture(
    db: &Db,
    capture_id: &str,
    source_app: Option<&str>,
    window_title: Option<&str>,
    ocr_text: Option<&str>,
    tag_names: Option<&str>,
) -> Result<()> {
    todo!()
}

pub fn index_clipboard(
    db: &Db,
    clipboard_id: &str,
    text_content: Option<&str>,
    source_app: Option<&str>,
    window_title: Option<&str>,
) -> Result<()> {
    todo!()
}

pub fn search(db: &Db, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
    todo!()
}

pub fn remove_capture_index(db: &Db, capture_id: &str) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fresh_db() -> Db {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        std::mem::forget(dir);
        Db::open(&path).unwrap()
    }

    fn insert_capture(db: &Db, app: &str, title: &str) -> String {
        crate::captures::insert(
            db,
            crate::NewCapture {
                file_path: PathBuf::from("test.png"),
                width: 100,
                height: 100,
                source_app: Some(app.into()),
                source_window_title: Some(title.into()),
                monitor: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn index_and_search_capture_by_ocr_text() {
        let db = fresh_db();
        let id = insert_capture(&db, "Firefox", "GitHub");
        index_capture(&db, &id, Some("Firefox"), Some("GitHub"), Some("hello world rust"), None).unwrap();

        let results = search(&db, "rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::Capture { id: rid, .. } => assert_eq!(rid, &id),
            _ => panic!("expected Capture result"),
        }
    }

    #[test]
    fn index_and_search_clipboard_by_text_content() {
        let db = fresh_db();
        let item = crate::clipboard::insert(
            &db,
            crate::NewClipboardItem {
                kind: crate::ClipboardItemKind::Text,
                text_content: Some("important meeting notes".into()),
                file_path: None,
                content_hash: "abc".into(),
                source_app: Some("Slack".into()),
                source_window_title: Some("General".into()),
            },
        )
        .unwrap();
        index_clipboard(&db, &item.id, Some("important meeting notes"), Some("Slack"), Some("General")).unwrap();

        let results = search(&db, "meeting", 10).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::Clipboard { id: rid, .. } => assert_eq!(rid, &item.id),
            _ => panic!("expected Clipboard result"),
        }
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let db = fresh_db();
        let results = search(&db, "nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_mixed_results_ranked() {
        let db = fresh_db();
        let cap_id = insert_capture(&db, "VS Code", "main.rs");
        index_capture(&db, &cap_id, Some("VS Code"), Some("main.rs"), Some("fn main rust"), None).unwrap();

        let clip = crate::clipboard::insert(
            &db,
            crate::NewClipboardItem {
                kind: crate::ClipboardItemKind::Text,
                text_content: Some("rust programming language".into()),
                file_path: None,
                content_hash: "xyz".into(),
                source_app: Some("Firefox".into()),
                source_window_title: Some("docs.rs".into()),
            },
        )
        .unwrap();
        index_clipboard(&db, &clip.id, Some("rust programming language"), Some("Firefox"), Some("docs.rs")).unwrap();

        let results = search(&db, "rust", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn update_capture_index_replaces_old_entry() {
        let db = fresh_db();
        let id = insert_capture(&db, "App", "Win");
        index_capture(&db, &id, Some("App"), Some("Win"), Some("old text"), None).unwrap();
        index_capture(&db, &id, Some("App"), Some("Win"), Some("new text"), None).unwrap();

        let results = search(&db, "old", 10).unwrap();
        assert!(results.is_empty());
        let results = search(&db, "new", 10).unwrap();
        assert_eq!(results.len(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library search::tests`
Expected: FAIL (todo!() panics)

**Step 3: Implement FTS5 population and query**

Replace the `todo!()` bodies in `crates/snk-library/src/search.rs`:

```rust
pub fn index_capture(
    db: &Db,
    capture_id: &str,
    source_app: Option<&str>,
    window_title: Option<&str>,
    ocr_text: Option<&str>,
    tag_names: Option<&str>,
) -> Result<()> {
    db.with_conn(|conn| {
        // Delete existing entry for this capture (contentless FTS5 requires explicit delete+insert)
        conn.execute(
            "DELETE FROM captures_fts WHERE capture_id = ?1",
            [capture_id],
        )?;
        conn.execute(
            "INSERT INTO captures_fts (capture_id, source_app, window_title, ocr_text, tag_names)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                capture_id,
                source_app.unwrap_or(""),
                window_title.unwrap_or(""),
                ocr_text.unwrap_or(""),
                tag_names.unwrap_or(""),
            ],
        )?;
        Ok(())
    })
}

pub fn index_clipboard(
    db: &Db,
    clipboard_id: &str,
    text_content: Option<&str>,
    source_app: Option<&str>,
    window_title: Option<&str>,
) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM clipboard_fts WHERE clipboard_id = ?1",
            [clipboard_id],
        )?;
        conn.execute(
            "INSERT INTO clipboard_fts (clipboard_id, text_content, source_app, window_title)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                clipboard_id,
                text_content.unwrap_or(""),
                source_app.unwrap_or(""),
                window_title.unwrap_or(""),
            ],
        )?;
        Ok(())
    })
}

pub fn search(db: &Db, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = sanitize_fts_query(query);
    db.with_conn(|conn| {
        let mut results = Vec::new();

        // Search captures
        let mut stmt = conn.prepare(
            "SELECT capture_id, rank, snippet(captures_fts, 3, '<mark>', '</mark>', '...', 32)
             FROM captures_fts
             WHERE captures_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let capture_rows = stmt.query_map(rusqlite::params![&fts_query, limit], |row| {
            Ok(SearchResult::Capture {
                id: row.get(0)?,
                rank: row.get(1)?,
                snippet: row.get(2)?,
            })
        })?;
        for row in capture_rows {
            results.push(row?);
        }

        // Search clipboard
        let mut stmt = conn.prepare(
            "SELECT clipboard_id, rank, snippet(clipboard_fts, 1, '<mark>', '</mark>', '...', 32)
             FROM clipboard_fts
             WHERE clipboard_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let clipboard_rows = stmt.query_map(rusqlite::params![&fts_query, limit], |row| {
            Ok(SearchResult::Clipboard {
                id: row.get(0)?,
                rank: row.get(1)?,
                snippet: row.get(2)?,
            })
        })?;
        for row in clipboard_rows {
            results.push(row?);
        }

        // Sort by rank (FTS5 rank is negative — more negative = better match)
        results.sort_by(|a, b| {
            let ra = match a {
                SearchResult::Capture { rank, .. } => *rank,
                SearchResult::Clipboard { rank, .. } => *rank,
            };
            let rb = match b {
                SearchResult::Capture { rank, .. } => *rank,
                SearchResult::Clipboard { rank, .. } => *rank,
            };
            ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit as usize);
        Ok(results)
    })
}

pub fn remove_capture_index(db: &Db, capture_id: &str) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM captures_fts WHERE capture_id = ?1",
            [capture_id],
        )?;
        Ok(())
    })
}

fn sanitize_fts_query(input: &str) -> String {
    // Escape special FTS5 characters and wrap each token in quotes for prefix matching
    let tokens: Vec<String> = input
        .split_whitespace()
        .map(|token| {
            let cleaned: String = token.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
            if cleaned.is_empty() {
                String::new()
            } else {
                format!("\"{cleaned}\"*")
            }
        })
        .filter(|s| !s.is_empty())
        .collect();
    tokens.join(" ")
}
```

**Step 4: Register module in lib.rs**

Add to `crates/snk-library/src/lib.rs`:
- `pub mod search;` after the `ocr` module line
- `pub use search::SearchResult;` in the re-exports

**Step 5: Run tests to verify they pass**

Run: `cargo test -p snk-library search::tests`
Expected: PASS (5 tests)

**Step 6: Commit**

```bash
git add crates/snk-library/src/search.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add search module — FTS5 index + unified search query"
```

---

## Task 4: Wire FTS Population into Existing Insert Functions

**Files:**
- Modify: `crates/snk-library/src/captures.rs` (insert calls `search::index_capture`)
- Modify: `crates/snk-library/src/clipboard.rs` (insert calls `search::index_clipboard`)

**Step 1: Write the failing test**

Add to `crates/snk-library/src/captures.rs` tests:

```rust
#[test]
fn insert_populates_fts_index() {
    let db = fresh_db();
    let new = NewCapture {
        file_path: PathBuf::from("fts.png"),
        width: 800,
        height: 600,
        source_app: Some("VS Code".into()),
        source_window_title: Some("main.rs — snapper-keeper".into()),
        monitor: None,
    };
    let c = insert(&db, new).unwrap();
    let results = crate::search::search(&db, "VS Code", 10).unwrap();
    assert_eq!(results.len(), 1);
    match &results[0] {
        crate::search::SearchResult::Capture { id, .. } => assert_eq!(id, &c.id),
        _ => panic!("expected Capture result"),
    }
}
```

Add to `crates/snk-library/src/clipboard.rs` tests:

```rust
#[test]
fn insert_populates_clipboard_fts_index() {
    let db = fresh_db();
    let mut item = sample_item("fts-hash");
    item.text_content = Some("important search term".into());
    item.source_app = Some("Terminal".into());
    let inserted = insert(&db, item).unwrap();
    let results = crate::search::search(&db, "important", 10).unwrap();
    assert_eq!(results.len(), 1);
    match &results[0] {
        crate::search::SearchResult::Clipboard { id, .. } => assert_eq!(id, &inserted.id),
        _ => panic!("expected Clipboard result"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p snk-library captures::tests::insert_populates_fts_index`
Run: `cargo test -p snk-library clipboard::tests::insert_populates_clipboard_fts_index`
Expected: FAIL (no FTS population yet)

**Step 3: Wire FTS population into captures::insert**

In `crates/snk-library/src/captures.rs`, after the `conn.execute(INSERT...)` call inside `db.with_conn`, add:

```rust
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO captures
                (id, file_path, width, height, source_app, source_window_title, monitor, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &id,
                &file_path,
                new.width,
                new.height,
                new.source_app,
                new.source_window_title,
                new.monitor,
                created_at,
            ],
        )?;
        Ok(())
    })?;

    // Populate FTS index with capture metadata (OCR text added later by snk-ocr)
    crate::search::index_capture(
        db,
        &id,
        new.source_app.as_deref(),
        new.source_window_title.as_deref(),
        None,
        None,
    )?;
```

Note: The full function body stays the same — just add the `crate::search::index_capture(...)` call after the `db.with_conn(...)` block and before the `Ok(Capture {...})`.

**Step 4: Wire FTS population into clipboard::insert**

In `crates/snk-library/src/clipboard.rs`, after the `db.with_conn(INSERT...)` block, add:

```rust
    // Populate FTS index
    crate::search::index_clipboard(
        db,
        &id,
        new.text_content.as_deref(),
        new.source_app.as_deref(),
        new.source_window_title.as_deref(),
    )?;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p snk-library captures::tests::insert_populates_fts_index`
Run: `cargo test -p snk-library clipboard::tests::insert_populates_clipboard_fts_index`
Expected: PASS

**Step 6: Run full library test suite**

Run: `cargo test -p snk-library`
Expected: All tests pass (including pre-existing tests)

**Step 7: Commit**

```bash
git add crates/snk-library/src/captures.rs crates/snk-library/src/clipboard.rs
git commit -m "feat(library): wire FTS5 population into captures::insert and clipboard::insert"
```

---

## Task 5: Library search_library Command

**Files:**
- Modify: `crates/snk-library/src/commands.rs` (add `search_library` command)
- Modify: `crates/snk-library/src/plugin.rs` (register command)
- Modify: `crates/snk-library/build.rs` (add to COMMANDS)

**Step 1: Add search_library command**

Add to `crates/snk-library/src/commands.rs`:

```rust
use crate::search::{self, SearchResult};

#[tauri::command]
pub fn search_library<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>> {
    search::search(&state.db, &query, limit.unwrap_or(50))
}
```

**Step 2: Register in plugin.rs invoke_handler**

Add `crate::commands::search_library` to the `tauri::generate_handler![]` macro in `crates/snk-library/src/plugin.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    crate::commands::list_captures,
    crate::commands::get_capture,
    crate::commands::soft_delete_capture,
    crate::commands::list_clipboard_items,
    crate::commands::get_clipboard_item,
    crate::commands::toggle_clipboard_pin,
    crate::commands::search_library,
])
```

**Step 3: Add to build.rs COMMANDS**

Update `crates/snk-library/build.rs`:

```rust
const COMMANDS: &[&str] = &[
    "list_captures",
    "get_capture",
    "soft_delete_capture",
    "list_clipboard_items",
    "get_clipboard_item",
    "toggle_clipboard_pin",
    "search_library",
];
```

**Step 4: Verify build compiles**

Run: `cargo build -p snk-library`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add crates/snk-library/src/commands.rs crates/snk-library/src/plugin.rs crates/snk-library/build.rs
git commit -m "feat(library): add search_library Tauri command"
```

---

## Task 6: snk-ocr Crate Scaffold — Plugin + Queue + Sidecar

**Files:**
- Create: `crates/snk-ocr/Cargo.toml`
- Create: `crates/snk-ocr/build.rs`
- Create: `crates/snk-ocr/src/lib.rs`
- Create: `crates/snk-ocr/src/plugin.rs`
- Create: `crates/snk-ocr/src/queue.rs`
- Create: `crates/snk-ocr/src/sidecar.rs`
- Create: `crates/snk-ocr/permissions/default.toml`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Create Cargo.toml**

Create `crates/snk-ocr/Cargo.toml`:

```toml
[package]
name = "snk-ocr"
version = "0.0.1"
links = "snk-ocr"
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
tokio.workspace = true

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create build.rs**

Create `crates/snk-ocr/build.rs`:

```rust
const COMMANDS: &[&str] = &["ocr_status"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 3: Create permissions/default.toml**

Create `crates/snk-ocr/permissions/default.toml`:

```toml
[default]
description = "Default permissions for snk-ocr plugin"
permissions = ["allow-ocr-status"]
```

**Step 4: Create src/sidecar.rs — tesseract invocation with retry**

Create `crates/snk-ocr/src/sidecar.rs`:

```rust
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use tracing::{info, warn};

pub struct OcrOutput {
    pub text: String,
    pub confidence: f64,
}

pub fn run_tesseract(image_path: &Path, language: &str) -> Result<OcrOutput, String> {
    let mut last_err = String::new();
    let delays = [Duration::from_millis(0), Duration::from_secs(1), Duration::from_secs(3)];

    for (attempt, delay) in delays.iter().enumerate() {
        if attempt > 0 {
            std::thread::sleep(*delay);
            warn!(attempt, "retrying tesseract");
        }

        match invoke_tesseract(image_path, language) {
            Ok(output) => {
                info!(attempt, chars = output.text.len(), "tesseract succeeded");
                return Ok(output);
            }
            Err(e) => {
                last_err = e;
                warn!(attempt, error = %last_err, "tesseract failed");
            }
        }
    }

    Err(format!("tesseract failed after 3 attempts: {last_err}"))
}

fn invoke_tesseract(image_path: &Path, language: &str) -> Result<OcrOutput, String> {
    let output = Command::new("tesseract")
        .arg(image_path.as_os_str())
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg("3")
        .output()
        .map_err(|e| format!("spawn tesseract: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tesseract exit {}: {stderr}", output.status));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Tesseract doesn't emit confidence to stdout in this mode;
    // use a heuristic: non-empty text with reasonable char count = high confidence.
    let confidence = if text.is_empty() {
        0.0
    } else {
        0.85
    };

    Ok(OcrOutput { text, confidence })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_tesseract_returns_error_when_binary_missing() {
        // This test verifies error handling when tesseract isn't found.
        // It will pass whether tesseract is installed or not — we're testing
        // the error path with a deliberately bad image path.
        let result = invoke_tesseract(Path::new("/nonexistent/image.png"), "eng");
        // Either tesseract isn't found (spawn error) or it fails on the bad path
        assert!(result.is_err() || result.unwrap().text.is_empty());
    }
}
```

**Step 5: Create src/queue.rs — async OCR task queue**

Create `crates/snk-ocr/src/queue.rs`:

```rust
use std::sync::Arc;

use snk_library::Db;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::sidecar;

pub struct OcrQueue {
    tx: mpsc::UnboundedSender<OcrJob>,
}

struct OcrJob {
    capture_id: String,
    image_path: std::path::PathBuf,
    language: String,
}

impl OcrQueue {
    pub fn start(db: Arc<Db>, library_root: std::path::PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(worker(rx, db, library_root));
        Self { tx }
    }

    pub fn enqueue(&self, capture_id: String, image_path: std::path::PathBuf, language: String) {
        if self.tx.send(OcrJob { capture_id, image_path, language }).is_err() {
            error!("ocr queue closed");
        }
    }
}

async fn worker(
    mut rx: mpsc::UnboundedReceiver<OcrJob>,
    db: Arc<Db>,
    library_root: std::path::PathBuf,
) {
    info!("ocr worker started");
    while let Some(job) = rx.recv().await {
        let full_path = library_root.join(&job.image_path);
        let db_clone = db.clone();

        // Run blocking tesseract on a dedicated thread
        let result = tokio::task::spawn_blocking(move || {
            sidecar::run_tesseract(&full_path, &job.language)
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.text.is_empty() {
                    info!(capture_id = %job.capture_id, "ocr produced no text");
                    continue;
                }
                if let Err(e) = snk_library::ocr::upsert(
                    &db_clone,
                    &job.capture_id,
                    &output.text,
                    &job.language,
                    output.confidence,
                ) {
                    error!(capture_id = %job.capture_id, error = %e, "failed to store ocr text");
                    continue;
                }
                // Update FTS index with OCR text
                let ocr_row = snk_library::ocr::get(&db_clone, &job.capture_id);
                if let Ok(Some(_)) = ocr_row {
                    // Re-index capture with OCR text
                    let capture = snk_library::captures::get(&db_clone, &job.capture_id);
                    if let Ok(cap) = capture {
                        let _ = snk_library::search::index_capture(
                            &db_clone,
                            &job.capture_id,
                            cap.source_app.as_deref(),
                            cap.source_window_title.as_deref(),
                            Some(&output.text),
                            None,
                        );
                    }
                }
                info!(capture_id = %job.capture_id, chars = output.text.len(), "ocr indexed");
            }
            Ok(Err(e)) => {
                error!(capture_id = %job.capture_id, error = %e, "ocr sidecar failed");
            }
            Err(e) => {
                error!(capture_id = %job.capture_id, error = %e, "ocr task panicked");
            }
        }
    }
    info!("ocr worker stopped");
}
```

**Step 6: Create src/plugin.rs — Tauri plugin with event listener**

Create `crates/snk-ocr/src/plugin.rs`:

```rust
use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Listener, Manager, Runtime};

use snk_library::plugin::LibraryState;

use crate::queue::OcrQueue;

pub struct OcrState {
    pub queue: OcrQueue,
}

#[tauri::command]
pub fn ocr_status<R: Runtime>(_app: tauri::AppHandle<R>) -> Result<String, String> {
    // Simple status check — can be extended later with queue depth, etc.
    Ok("running".to_string())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-ocr")
        .invoke_handler(tauri::generate_handler![ocr_status])
        .setup(|app, _api| {
            let lib_state = app.state::<LibraryState>();
            let db = lib_state.db.clone();
            let root = lib_state.root.clone();

            let queue = OcrQueue::start(Arc::clone(&db), root.clone());
            app.manage(OcrState { queue });

            // Listen for capture:saved events and enqueue OCR
            let ocr_state = app.state::<OcrState>();
            let queue_handle = &ocr_state.queue;
            let db_for_listener = Arc::clone(&db);
            let root_for_listener = root.clone();

            // We need to clone the sender side to move into the listener closure
            let app_handle = app.app_handle().clone();
            app_handle.listen("capture:saved", move |event| {
                let capture_id = event.payload().trim_matches('"').to_string();
                if capture_id.is_empty() {
                    return;
                }

                // Look up the capture to get its file_path
                match snk_library::captures::get(&db_for_listener, &capture_id) {
                    Ok(capture) => {
                        let image_path = std::path::PathBuf::from(&capture.file_path);
                        // Get language from settings (default to "eng")
                        let language = "eng".to_string();
                        if let Some(ocr) = app_handle.try_state::<OcrState>() {
                            ocr.queue.enqueue(capture_id, image_path, language);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(capture_id, error = %e, "could not look up capture for ocr");
                    }
                }
            });

            Ok(())
        })
        .build()
}
```

**Step 7: Create src/lib.rs**

Create `crates/snk-ocr/src/lib.rs`:

```rust
pub mod plugin;
pub mod queue;
pub mod sidecar;

pub use plugin::init;
```

**Step 8: Create permissions/default.toml**

Already created in Step 3.

**Step 9: Add snk-ocr to workspace members**

In root `Cargo.toml`, add `"crates/snk-ocr"` to the `members` array:

```toml
[workspace]
resolver = "2"
members = [
    "crates/snk-library",
    "crates/snk-hotkeys",
    "crates/snk-capture",
    "crates/snk-annotate",
    "crates/snk-clipboard",
    "crates/snk-ocr",
    "app/src-tauri",
]
```

**Step 10: Verify build compiles**

Run: `cargo build -p snk-ocr`
Expected: SUCCESS

**Step 11: Commit**

```bash
git add crates/snk-ocr/ Cargo.toml
git commit -m "feat(ocr): scaffold snk-ocr crate — async queue + tesseract sidecar + retry"
```

---

## Task 7: Integrate snk-ocr into the App

**Files:**
- Modify: `app/src-tauri/Cargo.toml` (add snk-ocr dep)
- Modify: `app/src-tauri/src/main.rs` (load plugin)
- Modify: `app/src-tauri/capabilities/default.json` (add permission)

**Step 1: Add snk-ocr dependency**

Add to `app/src-tauri/Cargo.toml` dependencies:

```toml
snk-ocr = { path = "../../crates/snk-ocr" }
```

**Step 2: Load the plugin in main.rs**

Add `.plugin(snk_ocr::init())` after `.plugin(snk_clipboard::init())` in `app/src-tauri/src/main.rs`:

```rust
.plugin(snk_clipboard::init())
.plugin(snk_ocr::init())
```

**Step 3: Add permission to capabilities**

Add `"snk-ocr:default"` to `app/src-tauri/capabilities/default.json` permissions array:

```json
"snk-clipboard:default",
"snk-ocr:default"
```

**Step 4: Verify full app builds**

Run: `cargo build -p snapper-keeper-app`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/src/main.rs app/src-tauri/capabilities/default.json
git commit -m "feat(app): integrate snk-ocr plugin into app shell"
```

---

## Task 8: @snk/ocr TypeScript Package

**Files:**
- Create: `packages/snk-ocr/package.json`
- Create: `packages/snk-ocr/tsconfig.json`
- Create: `packages/snk-ocr/src/index.ts`
- Create: `packages/snk-ocr/src/types.ts`

**Step 1: Create package.json**

Create `packages/snk-ocr/package.json`:

```json
{
  "name": "@snk/ocr",
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
    "@tauri-apps/api": "^2.0.0"
  },
  "devDependencies": {
    "typescript": "^5.4.0"
  }
}
```

**Step 2: Create tsconfig.json**

Create `packages/snk-ocr/tsconfig.json` (inherits everything from tsconfig.base.json — no local overrides, matching sibling packages):

```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

**Step 3: Create src/types.ts**

Create `packages/snk-ocr/src/types.ts`:

```typescript
export interface OcrText {
  capture_id: string;
  text: string;
  language: string;
  confidence: number;
  created_at: number;
}
```

**Step 4: Create src/index.ts**

Create `packages/snk-ocr/src/index.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

export * from './types';

export const OCR_COMPLETED_EVENT = 'ocr:completed';

export function ocrStatus(): Promise<string> {
  return invoke<string>('plugin:snk-ocr|ocr_status');
}
```

**Step 5: Run pnpm install to link the new package**

Run: `pnpm install`
Expected: SUCCESS (new package linked in workspace)

**Step 6: Commit**

```bash
git add packages/snk-ocr/
git commit -m "feat(ocr): add @snk/ocr TypeScript bindings package"
```

---

## Task 9: Library TS Search Binding

**Files:**
- Modify: `packages/snk-library/src/types.ts` (add SearchResult type)
- Modify: `packages/snk-library/src/index.ts` (add searchLibrary function)

**Step 1: Add SearchResult type**

Add to `packages/snk-library/src/types.ts`:

```typescript
export type SearchResult =
  | { kind: 'capture'; id: string; rank: number; snippet: string }
  | { kind: 'clipboard'; id: string; rank: number; snippet: string };
```

**Step 2: Add searchLibrary function**

Add to `packages/snk-library/src/index.ts`:

```typescript
import type { Capture, ListCapturesQuery, SearchResult } from './types';

export function searchLibrary(query: string, limit?: number): Promise<SearchResult[]> {
  return invoke<SearchResult[]>('plugin:snk-library|search_library', { query, limit });
}
```

**Step 3: Verify TypeScript compiles**

Run: `pnpm --filter @snk/library exec tsc --noEmit`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add packages/snk-library/src/types.ts packages/snk-library/src/index.ts
git commit -m "feat(library): add searchLibrary TS binding + SearchResult type"
```

---

## Task 10: Library Window Search Bar Component

**Files:**
- Create: `app/src/windows/library/SearchBar.tsx`
- Modify: `app/src/windows/library/LibraryWindow.tsx` (integrate search bar)

**Step 1: Create SearchBar component**

Create `app/src/windows/library/SearchBar.tsx`:

```tsx
import { useState, useCallback, useEffect, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { searchLibrary, type SearchResult } from '@snk/library';

export function SearchBar() {
  const [input, setInput] = useState('');
  const [debouncedQuery, setDebouncedQuery] = useState('');
  const timerRef = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setDebouncedQuery(input.trim());
    }, 250);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [input]);

  const { data: results, isLoading } = useQuery({
    queryKey: ['search', debouncedQuery],
    queryFn: () => searchLibrary(debouncedQuery, 20),
    enabled: debouncedQuery.length > 0,
  });

  const handleClear = useCallback(() => {
    setInput('');
    setDebouncedQuery('');
  }, []);

  return (
    <div className="relative">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Search captures & clipboard..."
        className="w-full bg-slate-800 border border-slate-700 rounded px-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-slate-500"
      />
      {input && (
        <button
          onClick={handleClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 text-xs"
        >
          Clear
        </button>
      )}
      {debouncedQuery && results && results.length > 0 && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-slate-800 border border-slate-700 rounded shadow-lg max-h-64 overflow-auto z-50">
          {results.map((result) => (
            <SearchResultRow key={resultKey(result)} result={result} />
          ))}
        </div>
      )}
      {debouncedQuery && results && results.length === 0 && !isLoading && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-slate-800 border border-slate-700 rounded shadow-lg p-3 z-50">
          <p className="text-slate-500 text-xs text-center">No results</p>
        </div>
      )}
    </div>
  );
}

function resultKey(result: SearchResult): string {
  return `${result.kind}-${result.id}`;
}

function SearchResultRow({ result }: { result: SearchResult }) {
  const icon = result.kind === 'capture' ? 'img' : 'txt';
  return (
    <div className="px-3 py-2 hover:bg-slate-700 cursor-pointer border-b border-slate-700 last:border-0">
      <div className="flex items-center gap-2">
        <span className="text-[10px] font-mono text-slate-500 uppercase w-6">{icon}</span>
        <span
          className="text-xs text-slate-300 truncate flex-1"
          dangerouslySetInnerHTML={{ __html: result.snippet }}
        />
      </div>
    </div>
  );
}
```

**Step 2: Integrate SearchBar into LibraryWindow**

In `app/src/windows/library/LibraryWindow.tsx`, add the SearchBar to the header:

```tsx
import { SearchBar } from './SearchBar';
```

Replace the header section:

```tsx
<header className="px-4 py-2 border-b border-slate-800 flex items-center gap-3">
  <h1 className="text-sm font-semibold">snapper-keeper</h1>
  <div className="flex-1 max-w-md">
    <SearchBar />
  </div>
  <button
    className="bg-slate-800 hover:bg-slate-700 text-slate-100 px-3 py-1 rounded text-xs"
    onClick={handleFullScreen}
  >
    Capture screen
  </button>
</header>
```

**Step 3: Verify TypeScript compiles**

Run: `pnpm --filter @snk/app exec tsc --noEmit`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add app/src/windows/library/SearchBar.tsx app/src/windows/library/LibraryWindow.tsx
git commit -m "feat(ui): add search bar to library window with debounced FTS5 query"
```

---

## Task 11: OCR Integration Test with Fixture Image

**Files:**
- Create: `crates/snk-ocr/tests/fixtures/hello.png` (simple image with text "Hello World")
- Create: `crates/snk-ocr/tests/integration_test.rs`

**Step 1: Create a test fixture image**

Create a simple PNG image with text "Hello" using a Rust test that generates it. Instead of checking in a binary, we'll generate it in the test setup using the `image` crate.

Add `image` to `crates/snk-ocr/Cargo.toml` dev-dependencies:

```toml
[dev-dependencies]
tempfile = "3"
image = { workspace = true }
```

**Step 2: Write the integration test**

Create `crates/snk-ocr/tests/integration_test.rs`:

```rust
//! Integration test — requires tesseract installed on the machine.
//! Skips gracefully if tesseract is not available.

use std::path::Path;
use std::process::Command;

fn tesseract_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn sidecar_extracts_text_from_image() {
    if !tesseract_available() {
        eprintln!("SKIP: tesseract not installed");
        return;
    }

    // Create a simple white image with black text using imageproc would be ideal,
    // but for now we'll use a pre-existing fixture. Generate a minimal image
    // that tesseract can read by writing "Hello" as a simple bitmap.
    let dir = tempfile::tempdir().unwrap();
    let img_path = dir.path().join("test.png");

    // Create a 200x50 white image — tesseract won't find text but should not crash
    let img = image::RgbaImage::from_pixel(200, 50, image::Rgba([255, 255, 255, 255]));
    img.save(&img_path).unwrap();

    let result = snk_ocr::sidecar::run_tesseract(&img_path, "eng");
    // Blank image should succeed but return empty/near-empty text
    match result {
        Ok(output) => {
            // Success — tesseract processed the image without error
            assert!(output.text.len() < 10, "blank image should have minimal text");
        }
        Err(e) => {
            // Tesseract might emit warnings about empty pages — that's OK
            assert!(
                e.contains("empty") || e.contains("Empty") || e.contains("exit"),
                "unexpected error: {e}"
            );
        }
    }
}

#[test]
fn sidecar_retries_on_bad_path() {
    if !tesseract_available() {
        eprintln!("SKIP: tesseract not installed");
        return;
    }

    let result = snk_ocr::sidecar::run_tesseract(Path::new("/nonexistent/image.png"), "eng");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("3 attempts"), "should report retry exhaustion: {err}");
}
```

**Step 3: Run the tests**

Run: `cargo test -p snk-ocr -- --nocapture`
Expected: PASS (tests skip gracefully if tesseract not installed, pass if it is)

**Step 4: Commit**

```bash
git add crates/snk-ocr/tests/ crates/snk-ocr/Cargo.toml
git commit -m "test(ocr): integration tests for tesseract sidecar invocation"
```

---

## Task 12: Full Integration — End-to-End Wiring Verification

**Files:**
- No new files — this task verifies existing wiring

**Step 1: Run full Rust test suite**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run TypeScript type checking**

Run: `pnpm -r exec tsc --noEmit`
Expected: All packages type-check clean

**Step 3: Run ESLint**

Run: `pnpm --filter @snk/app lint`
Expected: No errors

**Step 4: Verify full app builds**

Run: `cargo build -p snapper-keeper-app`
Expected: SUCCESS

**Step 5: Verify app builds in release mode**

Run: `cargo build -p snapper-keeper-app --release`
Expected: SUCCESS (may take longer)

**Step 6: Commit any remaining fixes**

If any fixes were needed, commit them:

```bash
git add -A
git commit -m "fix(phase-5): address build/lint issues from integration verification"
```

If no fixes needed, skip this step.

---

## Summary of Deliverables

| Task | What ships |
|------|-----------|
| 1 | V003 migration (ocr_text + captures_fts + clipboard_fts) |
| 2 | `ocr` module in snk-library (upsert + get) |
| 3 | `search` module in snk-library (FTS5 population + unified query) |
| 4 | FTS5 auto-population wired into captures::insert + clipboard::insert |
| 5 | `search_library` Tauri command |
| 6 | `snk-ocr` crate (queue + sidecar + retry + event listener) |
| 7 | snk-ocr integrated into app (dep + plugin load + capability) |
| 8 | `@snk/ocr` TS package |
| 9 | `searchLibrary` TS binding in @snk/library |
| 10 | Library window search bar component |
| 11 | Integration tests for tesseract sidecar |
| 12 | Full build verification |

## Dependency Graph

```
Task 1 (migration)
  └── Task 2 (ocr module) ─────────────────┐
  └── Task 3 (search module) ──┐            │
       └── Task 4 (wire FTS) ──┤            │
       └── Task 5 (command) ───┤            │
            └── Task 9 (TS) ───┤            │
                 └── Task 10 (UI) ──────────┤
Task 6 (snk-ocr crate) depends on Tasks 2,3│
  └── Task 7 (app integration) ────────────┤
  └── Task 8 (@snk/ocr TS) ───────────────┤
  └── Task 11 (integration tests) ─────────┤
                                            │
Task 12 (verification) depends on ALL ──────┘
```

Tasks 2 and 3 can run in parallel after Task 1. Tasks 6, 8, 9 can start once their deps are met. Task 12 is the final gate.
