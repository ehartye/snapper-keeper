# Phase 6: Library Window Polish — Sidebar, Tags, Settings, First-Run Wizard

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Add the library sidebar (smart sections + tag filtering + clipboard history view), full tag CRUD, a settings window, and a first-run wizard to complete the library window experience.

**Architecture:** Tags and settings are pure `snk-library` domain — new Rust modules with Tauri commands, paired TS bindings, and React components. The sidebar controls which query the capture grid uses. The settings window is a separate Tauri window opened via tray menu. The first-run wizard is an overlay in the library window gated on the `firstrun.completed` setting.

**Tech Stack:** Rust (rusqlite, serde_json), TypeScript, React, TanStack Query, Tailwind CSS, Tauri 2 IPC

**Phase 6 scope:**
- Library sidebar with smart sections (All, Today, This Week, Pinned, Trash)
- Tag CRUD (create, update, delete) + tag-to-capture assignment + FTS re-index
- Sidebar tag filter
- Clipboard history view in library main area
- Settings window (capture, clipboard, OCR sections)
- First-run wizard (Windows: hotkey confirmation + library path; macOS permission steps stubbed)
- Tray menu "Settings" item

**Out of scope:** `snk-tray` plugin refactor (Phase 7), `snk-updater` (Phase 7), signing/notarization (Phase 7), sidebar item counts (future polish).

**Pre-flight:**
- Phases 1–5 merged to `main`
- `cargo test --workspace` passes (46 tests)
- `pnpm -r exec tsc --noEmit` clean
- `pnpm --filter @snk/app lint` clean

---

### Task 1: Tags CRUD Rust module

**Files:**
- Create: `crates/snk-library/src/tags.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write the failing tests**

Create `crates/snk-library/src/tags.rs` with tests only:

```rust
use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: i64,
}

pub fn list(_db: &Db) -> Result<Vec<Tag>> {
    todo!()
}

pub fn get(_db: &Db, _id: &str) -> Result<Tag> {
    todo!()
}

pub fn create(_db: &Db, _name: &str, _color: &str) -> Result<Tag> {
    todo!()
}

pub fn update(_db: &Db, _id: &str, _name: &str, _color: &str) -> Result<Tag> {
    todo!()
}

pub fn delete(_db: &Db, _id: &str) -> Result<()> {
    todo!()
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

    #[test]
    fn create_and_list_tags() {
        let db = fresh_db();
        let t1 = create(&db, "bug", "#ff0000").unwrap();
        let t2 = create(&db, "feature", "#00ff00").unwrap();
        assert_eq!(t1.name, "bug");
        assert_eq!(t1.color, "#ff0000");
        assert_eq!(t1.id.len(), 36);

        let tags = list(&db).unwrap();
        let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["bug", "feature"]);
    }

    #[test]
    fn create_duplicate_name_fails() {
        let db = fresh_db();
        create(&db, "dup", "#111111").unwrap();
        let err = create(&db, "dup", "#222222");
        assert!(err.is_err());
    }

    #[test]
    fn get_returns_tag() {
        let db = fresh_db();
        let created = create(&db, "design", "#0000ff").unwrap();
        let fetched = get(&db, &created.id).unwrap();
        assert_eq!(fetched, created);
    }

    #[test]
    fn get_missing_returns_not_found() {
        let db = fresh_db();
        match get(&db, "no-such-id") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn update_changes_name_and_color() {
        let db = fresh_db();
        let t = create(&db, "old", "#aaa").unwrap();
        let updated = update(&db, &t.id, "new", "#bbb").unwrap();
        assert_eq!(updated.name, "new");
        assert_eq!(updated.color, "#bbb");
        assert_eq!(updated.id, t.id);
    }

    #[test]
    fn delete_removes_tag() {
        let db = fresh_db();
        let t = create(&db, "temp", "#ccc").unwrap();
        delete(&db, &t.id).unwrap();
        let tags = list(&db).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let db = fresh_db();
        match delete(&db, "no-such-id") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
}
```

**Step 2: Register the module**

In `crates/snk-library/src/lib.rs`, add after `pub mod search;`:

```rust
pub mod tags;
```

And in the re-exports section, add:

```rust
pub use tags::Tag;
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p snk-library tags::tests -- --nocapture`
Expected: FAIL — all tests panic with `todo!()`

**Step 4: Implement the functions**

Replace the `todo!()` function bodies in `crates/snk-library/src/tags.rs`:

```rust
pub fn list(db: &Db) -> Result<Vec<Tag>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT id, name, color, created_at FROM tags ORDER BY name")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    color: row.get("color")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

pub fn get(db: &Db, id: &str) -> Result<Tag> {
    db.with_conn(|conn| {
        conn.query_row(
            "SELECT id, name, color, created_at FROM tags WHERE id = ?1",
            [id],
            |row| {
                Ok(Tag {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    color: row.get("color")?,
                    created_at: row.get("created_at")?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => crate::LibraryError::NotFound {
                what: format!("tag {id}"),
            },
            other => other.into(),
        })
    })
}

pub fn create(db: &Db, name: &str, color: &str) -> Result<Tag> {
    let id = uuid::Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&id, name, color, created_at],
        )?;
        Ok(())
    })?;
    Ok(Tag {
        id,
        name: name.to_string(),
        color: color.to_string(),
        created_at,
    })
}

pub fn update(db: &Db, id: &str, name: &str, color: &str) -> Result<Tag> {
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
            rusqlite::params![name, color, id],
        )?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("tag {id}"),
            });
        }
        Ok(())
    })?;
    get(db, id)
}

pub fn delete(db: &Db, id: &str) -> Result<()> {
    db.with_conn(|conn| {
        let changed = conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("tag {id}"),
            });
        }
        Ok(())
    })
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p snk-library tags::tests -- --nocapture`
Expected: all 7 pass

**Step 6: Commit**

```bash
git add crates/snk-library/src/tags.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add tags CRUD module — create, list, get, update, delete"
```

---

### Task 2: Tag assignment + FTS re-index

**Files:**
- Modify: `crates/snk-library/src/tags.rs`

**Step 1: Write the failing tests**

Append to `crates/snk-library/src/tags.rs`, inside the existing `impl` block (before `#[cfg(test)]`):

```rust
pub fn assign(db: &Db, capture_id: &str, tag_id: &str) -> Result<()> {
    todo!()
}

pub fn remove(db: &Db, capture_id: &str, tag_id: &str) -> Result<()> {
    todo!()
}

pub fn list_for_capture(db: &Db, capture_id: &str) -> Result<Vec<Tag>> {
    todo!()
}

fn reindex_capture_tags(db: &Db, capture_id: &str) -> Result<()> {
    todo!()
}
```

Add these tests inside the existing `mod tests`:

```rust
    fn insert_test_capture(db: &Db) -> crate::Capture {
        crate::captures::insert(
            db,
            crate::NewCapture {
                file_path: std::path::PathBuf::from("test.png"),
                width: 100,
                height: 100,
                source_app: Some("TestApp".into()),
                source_window_title: Some("Test Window".into()),
                monitor: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn assign_and_list_for_capture() {
        let db = fresh_db();
        let cap = insert_test_capture(&db);
        let t1 = create(&db, "red", "#ff0000").unwrap();
        let t2 = create(&db, "blue", "#0000ff").unwrap();

        assign(&db, &cap.id, &t1.id).unwrap();
        assign(&db, &cap.id, &t2.id).unwrap();

        let tags = list_for_capture(&db, &cap.id).unwrap();
        let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"red"));
        assert!(names.contains(&"blue"));
    }

    #[test]
    fn assign_duplicate_is_idempotent() {
        let db = fresh_db();
        let cap = insert_test_capture(&db);
        let tag = create(&db, "dup", "#aaa").unwrap();
        assign(&db, &cap.id, &tag.id).unwrap();
        assign(&db, &cap.id, &tag.id).unwrap();
        let tags = list_for_capture(&db, &cap.id).unwrap();
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn remove_unassigns_tag() {
        let db = fresh_db();
        let cap = insert_test_capture(&db);
        let tag = create(&db, "gone", "#bbb").unwrap();
        assign(&db, &cap.id, &tag.id).unwrap();
        remove(&db, &cap.id, &tag.id).unwrap();
        let tags = list_for_capture(&db, &cap.id).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn assign_updates_fts_index() {
        let db = fresh_db();
        let cap = insert_test_capture(&db);
        let tag = create(&db, "searchable-tag", "#ccc").unwrap();
        assign(&db, &cap.id, &tag.id).unwrap();

        let results = crate::search::search(&db, "searchable-tag", 10).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            crate::search::SearchResult::Capture { id, .. } => assert_eq!(id, &cap.id),
            _ => panic!("expected Capture result"),
        }
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p snk-library tags::tests -- --nocapture`
Expected: FAIL — new tests panic

**Step 3: Implement the functions**

Replace the `todo!()` bodies:

```rust
pub fn assign(db: &Db, capture_id: &str, tag_id: &str) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO capture_tags (capture_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![capture_id, tag_id],
        )?;
        Ok(())
    })?;
    reindex_capture_tags(db, capture_id)
}

pub fn remove(db: &Db, capture_id: &str, tag_id: &str) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM capture_tags WHERE capture_id = ?1 AND tag_id = ?2",
            rusqlite::params![capture_id, tag_id],
        )?;
        Ok(())
    })?;
    reindex_capture_tags(db, capture_id)
}

pub fn list_for_capture(db: &Db, capture_id: &str) -> Result<Vec<Tag>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color, t.created_at
             FROM tags t
             INNER JOIN capture_tags ct ON ct.tag_id = t.id
             WHERE ct.capture_id = ?1
             ORDER BY t.name",
        )?;
        let rows = stmt
            .query_map([capture_id], |row| {
                Ok(Tag {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    color: row.get("color")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

fn reindex_capture_tags(db: &Db, capture_id: &str) -> Result<()> {
    let capture = crate::captures::get(db, capture_id)?;
    let ocr = crate::ocr::get(db, capture_id)?;
    let tags = list_for_capture(db, capture_id)?;
    let tag_names = if tags.is_empty() {
        None
    } else {
        Some(
            tags.iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        )
    };
    crate::search::index_capture(
        db,
        capture_id,
        capture.source_app.as_deref(),
        capture.source_window_title.as_deref(),
        ocr.as_ref().map(|o| o.text.as_str()),
        tag_names.as_deref(),
    )
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p snk-library tags::tests -- --nocapture`
Expected: all 11 pass (7 from Task 1 + 4 new)

**Step 5: Commit**

```bash
git add crates/snk-library/src/tags.rs
git commit -m "feat(library): add tag assignment with FTS re-index on tag change"
```

---

### Task 3: Settings Rust module

**Files:**
- Create: `crates/snk-library/src/settings.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write the failing tests**

Create `crates/snk-library/src/settings.rs`:

```rust
use serde_json::Value;

use crate::{Db, Result};

pub fn get(db: &Db, key: &str) -> Result<Option<Value>> {
    todo!()
}

pub fn set(db: &Db, key: &str, value: &Value) -> Result<()> {
    todo!()
}

pub fn delete(db: &Db, key: &str) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fresh_db() -> Db {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        std::mem::forget(dir);
        Db::open(&path).unwrap()
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let db = fresh_db();
        assert_eq!(get(&db, "no.such.key").unwrap(), None);
    }

    #[test]
    fn set_and_get_round_trips() {
        let db = fresh_db();
        set(&db, "capture.format", &json!("png")).unwrap();
        let val = get(&db, "capture.format").unwrap();
        assert_eq!(val, Some(json!("png")));
    }

    #[test]
    fn set_overwrites_existing() {
        let db = fresh_db();
        set(&db, "clipboard.history_size", &json!(200)).unwrap();
        set(&db, "clipboard.history_size", &json!(500)).unwrap();
        let val = get(&db, "clipboard.history_size").unwrap();
        assert_eq!(val, Some(json!(500)));
    }

    #[test]
    fn set_handles_complex_json() {
        let db = fresh_db();
        let blocklist = json!(["1Password", "KeePass"]);
        set(&db, "clipboard.app_blocklist", &blocklist).unwrap();
        let val = get(&db, "clipboard.app_blocklist").unwrap();
        assert_eq!(val, Some(blocklist));
    }

    #[test]
    fn delete_removes_key() {
        let db = fresh_db();
        set(&db, "temp.key", &json!(true)).unwrap();
        delete(&db, "temp.key").unwrap();
        assert_eq!(get(&db, "temp.key").unwrap(), None);
    }
}
```

**Step 2: Register the module**

In `crates/snk-library/src/lib.rs`, add after `pub mod search;`:

```rust
pub mod settings;
```

No re-exports needed — settings functions are accessed via `crate::settings::get` etc.

**Step 3: Run tests to verify they fail**

Run: `cargo test -p snk-library settings::tests -- --nocapture`
Expected: FAIL — all tests panic

**Step 4: Implement the functions**

Replace the `todo!()` bodies in `crates/snk-library/src/settings.rs`:

```rust
pub fn get(db: &Db, key: &str) -> Result<Option<Value>> {
    db.with_conn(|conn| {
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(raw) => {
                let val: Value = serde_json::from_str(&raw).map_err(|e| {
                    crate::LibraryError::Database {
                        message: format!("invalid JSON in setting {key}: {e}"),
                        retryable: false,
                    }
                })?;
                Ok(Some(val))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}

pub fn set(db: &Db, key: &str, value: &Value) -> Result<()> {
    let raw = serde_json::to_string(value).map_err(|e| crate::LibraryError::Database {
        message: format!("serialize setting: {e}"),
        retryable: false,
    })?;
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, &raw],
        )?;
        Ok(())
    })
}

pub fn delete(db: &Db, key: &str) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    })
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p snk-library settings::tests -- --nocapture`
Expected: all 5 pass

**Step 6: Commit**

```bash
git add crates/snk-library/src/settings.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add settings module — get, set, delete with JSON values"
```

---

### Task 4: Enhanced ListCapturesQuery

**Files:**
- Modify: `crates/snk-library/src/captures.rs`

**Step 1: Write the failing tests**

Add new fields to `ListCapturesQuery` and new tests. In `crates/snk-library/src/captures.rs`, update the struct:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListCapturesQuery {
    pub limit: Option<u32>,
    pub include_deleted: bool,
    pub since: Option<i64>,
    pub pinned_only: bool,
    pub tag_id: Option<String>,
    pub deleted_only: bool,
}
```

Add these tests inside `mod tests`:

```rust
    #[test]
    fn list_since_filters_by_date() {
        let db = fresh_db();
        let mk = |i: u32| NewCapture {
            file_path: PathBuf::from(format!("{i}.png")),
            width: i,
            height: i,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let _old = insert(&db, mk(1)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let cutoff = chrono::Utc::now().timestamp_millis();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let recent = insert(&db, mk(2)).unwrap();

        let rows = list(
            &db,
            ListCapturesQuery {
                since: Some(cutoff),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, recent.id);
    }

    #[test]
    fn list_pinned_only_filters() {
        let db = fresh_db();
        let mk = |i: u32| NewCapture {
            file_path: PathBuf::from(format!("{i}.png")),
            width: i,
            height: i,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let a = insert(&db, mk(1)).unwrap();
        let _b = insert(&db, mk(2)).unwrap();

        db.with_conn(|conn| {
            conn.execute(
                "UPDATE captures SET pinned = 1 WHERE id = ?1",
                [&a.id],
            )?;
            Ok(())
        })
        .unwrap();

        let rows = list(
            &db,
            ListCapturesQuery {
                pinned_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, a.id);
    }

    #[test]
    fn list_by_tag_id_filters() {
        let db = fresh_db();
        let mk = |i: u32| NewCapture {
            file_path: PathBuf::from(format!("{i}.png")),
            width: i,
            height: i,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let a = insert(&db, mk(1)).unwrap();
        let _b = insert(&db, mk(2)).unwrap();

        let tag = crate::tags::create(&db, "test-tag", "#fff").unwrap();
        crate::tags::assign(&db, &a.id, &tag.id).unwrap();

        let rows = list(
            &db,
            ListCapturesQuery {
                tag_id: Some(tag.id),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, a.id);
    }

    #[test]
    fn list_deleted_only_shows_trashed() {
        let db = fresh_db();
        let mk = |i: u32| NewCapture {
            file_path: PathBuf::from(format!("{i}.png")),
            width: i,
            height: i,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let a = insert(&db, mk(1)).unwrap();
        let b = insert(&db, mk(2)).unwrap();
        soft_delete(&db, &b.id).unwrap();

        let rows = list(
            &db,
            ListCapturesQuery {
                deleted_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, b.id);
        assert!(rows[0].deleted_at.is_some());

        let normal = list(&db, ListCapturesQuery::default()).unwrap();
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].id, a.id);
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p snk-library captures::tests -- --nocapture`
Expected: New tests FAIL because `list()` ignores the new filter fields

**Step 3: Rewrite the list function**

Replace the existing `list` function body with a dynamic query builder:

```rust
pub fn list(db: &Db, q: ListCapturesQuery) -> Result<Vec<Capture>> {
    let limit = q.limit.unwrap_or(200).min(1000) as i64;
    db.with_conn(|conn| {
        let mut clauses: Vec<String> = Vec::new();
        let mut values: Vec<rusqlite::types::Value> = Vec::new();

        if q.deleted_only {
            clauses.push("deleted_at IS NOT NULL".into());
        } else if !q.include_deleted {
            clauses.push("deleted_at IS NULL".into());
        }

        if let Some(since) = q.since {
            values.push(rusqlite::types::Value::Integer(since));
            clauses.push("created_at >= ?".into());
        }

        if q.pinned_only {
            clauses.push("pinned = 1".into());
        }

        if let Some(ref tag_id) = q.tag_id {
            values.push(rusqlite::types::Value::Text(tag_id.clone()));
            clauses.push(
                "id IN (SELECT capture_id FROM capture_tags WHERE tag_id = ?)".into(),
            );
        }

        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };

        values.push(rusqlite::types::Value::Integer(limit));
        let sql = format!(
            "SELECT * FROM captures{} ORDER BY created_at DESC LIMIT ?",
            where_sql
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values), row_to_capture)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p snk-library captures::tests -- --nocapture`
Expected: all tests pass (existing + 4 new)

**Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all pass — existing tests use `ListCapturesQuery::default()` or `ListCapturesQuery { limit: None, include_deleted: true }`, which still works because new fields default to `false`/`None`

**Step 6: Commit**

```bash
git add crates/snk-library/src/captures.rs
git commit -m "feat(library): add since, pinned_only, tag_id, deleted_only filters to ListCapturesQuery"
```

---

### Task 5: Tags + settings Tauri commands + permissions

**Files:**
- Modify: `crates/snk-library/src/commands.rs`
- Modify: `crates/snk-library/src/plugin.rs`
- Modify: `crates/snk-library/build.rs`
- Modify: `crates/snk-library/permissions/default.toml`

**Step 1: Add new commands to commands.rs**

Add these use statements at the top of `crates/snk-library/src/commands.rs`:

```rust
use crate::tags::{self, Tag};
```

Then append the new command functions after `search_library`:

```rust
#[tauri::command]
pub fn list_tags<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
) -> Result<Vec<Tag>> {
    tags::list(&state.db)
}

#[tauri::command]
pub fn create_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    name: String,
    color: String,
) -> Result<Tag> {
    tags::create(&state.db, &name, &color)
}

#[tauri::command]
pub fn update_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
    name: String,
    color: String,
) -> Result<Tag> {
    tags::update(&state.db, &id, &name, &color)
}

#[tauri::command]
pub fn delete_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    tags::delete(&state.db, &id)
}

#[tauri::command]
pub fn assign_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
    tag_id: String,
) -> Result<()> {
    tags::assign(&state.db, &capture_id, &tag_id)
}

#[tauri::command]
pub fn remove_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
    tag_id: String,
) -> Result<()> {
    tags::remove(&state.db, &capture_id, &tag_id)
}

#[tauri::command]
pub fn list_capture_tags<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
) -> Result<Vec<Tag>> {
    tags::list_for_capture(&state.db, &capture_id)
}

#[tauri::command]
pub fn get_setting<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    key: String,
) -> Result<Option<serde_json::Value>> {
    crate::settings::get(&state.db, &key)
}

#[tauri::command]
pub fn set_setting<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    key: String,
    value: serde_json::Value,
) -> Result<()> {
    crate::settings::set(&state.db, &key, &value)
}
```

**Step 2: Register commands in plugin.rs**

In `crates/snk-library/src/plugin.rs`, update the `invoke_handler` to include all new commands:

```rust
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_captures,
            crate::commands::get_capture,
            crate::commands::soft_delete_capture,
            crate::commands::list_clipboard_items,
            crate::commands::get_clipboard_item,
            crate::commands::toggle_clipboard_pin,
            crate::commands::search_library,
            crate::commands::list_tags,
            crate::commands::create_tag,
            crate::commands::update_tag,
            crate::commands::delete_tag,
            crate::commands::assign_tag,
            crate::commands::remove_tag,
            crate::commands::list_capture_tags,
            crate::commands::get_setting,
            crate::commands::set_setting,
        ])
```

**Step 3: Update build.rs**

Replace the COMMANDS array in `crates/snk-library/build.rs`:

```rust
const COMMANDS: &[&str] = &[
    "list_captures",
    "get_capture",
    "soft_delete_capture",
    "list_clipboard_items",
    "get_clipboard_item",
    "toggle_clipboard_pin",
    "search_library",
    "list_tags",
    "create_tag",
    "update_tag",
    "delete_tag",
    "assign_tag",
    "remove_tag",
    "list_capture_tags",
    "get_setting",
    "set_setting",
];
```

**Step 3.5: Update permissions/default.toml**

Replace the permissions list in `crates/snk-library/permissions/default.toml`:

```toml
[default]
description = "Default permissions for snk-library: allows all capture, clipboard, search, tag, and settings operations."
permissions = [
    "allow-list-captures",
    "allow-get-capture",
    "allow-soft-delete-capture",
    "allow-list-clipboard-items",
    "allow-get-clipboard-item",
    "allow-toggle-clipboard-pin",
    "allow-search-library",
    "allow-list-tags",
    "allow-create-tag",
    "allow-update-tag",
    "allow-delete-tag",
    "allow-assign-tag",
    "allow-remove-tag",
    "allow-list-capture-tags",
    "allow-get-setting",
    "allow-set-setting",
]
```

**Step 4: Verify it compiles**

Run: `cargo build -p snk-library`
Expected: success

**Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all pass

**Step 6: Commit**

```bash
git add crates/snk-library/src/commands.rs crates/snk-library/src/plugin.rs crates/snk-library/build.rs crates/snk-library/permissions/default.toml
git commit -m "feat(library): add 9 Tauri commands for tags + settings management"
```

---

### Task 6: Tags + settings TS bindings

**Files:**
- Modify: `packages/snk-library/src/types.ts`
- Modify: `packages/snk-library/src/index.ts`

**Step 1: Add new types**

In `packages/snk-library/src/types.ts`, append after the `SearchResult` type:

```typescript

export interface Tag {
  id: string;
  name: string;
  color: string;
  created_at: number;
}
```

Update the existing `ListCapturesQuery` to include new fields:

```typescript
export interface ListCapturesQuery {
  limit?: number;
  include_deleted?: boolean;
  since?: number;
  pinned_only?: boolean;
  tag_id?: string;
  deleted_only?: boolean;
}
```

**Step 2: Add invoke wrappers**

In `packages/snk-library/src/index.ts`, update the import to include `Tag`:

```typescript
import type { Capture, ListCapturesQuery, SearchResult, Tag } from './types';
```

Append the new functions after `searchLibrary`:

```typescript

export function listTags(): Promise<Tag[]> {
  return invoke<Tag[]>('plugin:snk-library|list_tags');
}

export function createTag(name: string, color: string): Promise<Tag> {
  return invoke<Tag>('plugin:snk-library|create_tag', { name, color });
}

export function updateTag(id: string, name: string, color: string): Promise<Tag> {
  return invoke<Tag>('plugin:snk-library|update_tag', { id, name, color });
}

export function deleteTag(id: string): Promise<void> {
  return invoke<void>('plugin:snk-library|delete_tag', { id });
}

export function assignTag(captureId: string, tagId: string): Promise<void> {
  return invoke<void>('plugin:snk-library|assign_tag', { captureId, tagId });
}

export function removeTag(captureId: string, tagId: string): Promise<void> {
  return invoke<void>('plugin:snk-library|remove_tag', { captureId, tagId });
}

export function listCaptureTags(captureId: string): Promise<Tag[]> {
  return invoke<Tag[]>('plugin:snk-library|list_capture_tags', { captureId });
}

export function getSetting(key: string): Promise<unknown | null> {
  return invoke('plugin:snk-library|get_setting', { key });
}

export function setSetting(key: string, value: unknown): Promise<void> {
  return invoke<void>('plugin:snk-library|set_setting', { key, value });
}
```

**Step 3: Verify TypeScript compiles**

Run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 4: Commit**

```bash
git add packages/snk-library/src/types.ts packages/snk-library/src/index.ts
git commit -m "feat(library): add tags + settings TS bindings and enhanced ListCapturesQuery types"
```

---

### Task 7: Library sidebar component

**Files:**
- Create: `app/src/windows/library/Sidebar.tsx`
- Modify: `app/src/windows/library/CaptureGrid.tsx`
- Modify: `app/src/windows/library/LibraryWindow.tsx`
- Modify: `app/src/lib/queryKeys.ts`

**Depends on:** Tasks 4, 6

**Step 1: Extend query keys**

Replace `app/src/lib/queryKeys.ts`:

```typescript
import type { ListCapturesQuery } from '@snk/library';

export const queryKeys = {
  captures: {
    list: (query?: ListCapturesQuery) =>
      ['captures', 'list', query ?? {}] as const,
    one: (id: string) => ['captures', 'one', id] as const,
  },
  tags: {
    list: () => ['tags', 'list'] as const,
    forCapture: (captureId: string) => ['tags', 'capture', captureId] as const,
  },
  settings: {
    one: (key: string) => ['settings', key] as const,
  },
  clipboard: {
    list: () => ['clipboard', 'list'] as const,
  },
};
```

**Step 2: Make CaptureGrid accept a query prop**

Replace the full `app/src/windows/library/CaptureGrid.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query';
import { path } from '@tauri-apps/api';

import { listCaptures } from '@snk/library';
import type { ListCapturesQuery } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { Thumbnail } from './Thumbnail';

interface Props {
  query?: ListCapturesQuery;
}

export function CaptureGrid({ query }: Props) {
  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });
  const captures = useQuery({
    queryKey: queryKeys.captures.list(query),
    queryFn: () => listCaptures(query),
  });

  if (root.isLoading || captures.isLoading) {
    return <p className="text-slate-500">Loading…</p>;
  }
  if (root.error || captures.error) {
    return (
      <p className="text-red-400">
        Error loading library: {String(root.error ?? captures.error)}
      </p>
    );
  }

  const rows = captures.data ?? [];
  if (rows.length === 0) {
    return (
      <div className="text-slate-500 text-sm">
        No captures yet. Press the hotkey or use the tray menu.
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
      {rows.map((c) => (
        <Thumbnail key={c.id} capture={c} src={captureAssetUrl(root.data!, c.file_path)} />
      ))}
    </div>
  );
}
```

**Step 3: Create the Sidebar component**

Create `app/src/windows/library/Sidebar.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query';

import { listTags } from '@snk/library';
import type { ListCapturesQuery, Tag } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

export type SidebarSelection =
  | { type: 'captures'; label: string; query: ListCapturesQuery }
  | { type: 'clipboard' };

function startOfDay(): number {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function startOfWeek(): number {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() - d.getDay());
  return d.getTime();
}

const SMART_SECTIONS: { label: string; query: ListCapturesQuery }[] = [
  { label: 'All', query: {} },
  { label: 'Today', query: { since: startOfDay() } },
  { label: 'This Week', query: { since: startOfWeek() } },
  { label: 'Pinned', query: { pinned_only: true } },
  { label: 'Trash', query: { deleted_only: true } },
];

interface Props {
  selection: SidebarSelection;
  onSelect: (s: SidebarSelection) => void;
}

function isActive(selection: SidebarSelection, label: string): boolean {
  if (selection.type === 'clipboard') return label === 'Clipboard History';
  return selection.label === label;
}

export function Sidebar({ selection, onSelect }: Props) {
  const tagsQuery = useQuery({
    queryKey: queryKeys.tags.list(),
    queryFn: () => listTags(),
  });

  const tags: Tag[] = tagsQuery.data ?? [];

  return (
    <aside className="w-56 shrink-0 border-r border-slate-800 flex flex-col overflow-y-auto">
      <nav className="p-2 space-y-0.5">
        {SMART_SECTIONS.map((s) => (
          <button
            key={s.label}
            className={`w-full text-left px-3 py-1.5 rounded text-sm ${
              isActive(selection, s.label)
                ? 'bg-slate-700 text-slate-100'
                : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'
            }`}
            onClick={() => onSelect({ type: 'captures', label: s.label, query: s.label === 'Today' ? { since: startOfDay() } : s.label === 'This Week' ? { since: startOfWeek() } : s.query })}
          >
            {s.label}
          </button>
        ))}
      </nav>

      <div className="border-t border-slate-800 mx-2 my-1" />

      <div className="p-2">
        <div className="text-[10px] uppercase tracking-wider text-slate-500 px-3 mb-1">Tags</div>
        {tags.length === 0 ? (
          <div className="text-xs text-slate-600 px-3">No tags yet</div>
        ) : (
          <nav className="space-y-0.5">
            {tags.map((tag) => (
              <button
                key={tag.id}
                className={`w-full text-left px-3 py-1.5 rounded text-sm flex items-center gap-2 ${
                  isActive(selection, tag.name)
                    ? 'bg-slate-700 text-slate-100'
                    : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'
                }`}
                onClick={() =>
                  onSelect({
                    type: 'captures',
                    label: tag.name,
                    query: { tag_id: tag.id },
                  })
                }
              >
                <span
                  className="w-2.5 h-2.5 rounded-full shrink-0"
                  style={{ backgroundColor: tag.color }}
                />
                {tag.name}
              </button>
            ))}
          </nav>
        )}
      </div>

      <div className="border-t border-slate-800 mx-2 my-1" />

      <nav className="p-2">
        <button
          className={`w-full text-left px-3 py-1.5 rounded text-sm ${
            selection.type === 'clipboard'
              ? 'bg-slate-700 text-slate-100'
              : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'
          }`}
          onClick={() => onSelect({ type: 'clipboard' })}
        >
          Clipboard History
        </button>
      </nav>
    </aside>
  );
}
```

**Step 4: Wire sidebar into LibraryWindow**

Replace `app/src/windows/library/LibraryWindow.tsx` — keep all existing event handlers, add sidebar state and layout:

```tsx
import { useEffect, useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalPosition } from '@tauri-apps/api/dpi';

import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
} from '@snk/capture';
import { CLIPBOARD_HISTORY_EVENT, CLIPBOARD_POPUP_SHOW_EVENT, showPopup } from '@snk/clipboard';

import { CaptureGrid } from './CaptureGrid';
import { ClipboardList } from './ClipboardList';
import { SearchBar } from './SearchBar';
import { Sidebar } from './Sidebar';
import type { SidebarSelection } from './Sidebar';

export function LibraryWindow() {
  const queryClient = useQueryClient();
  const [selection, setSelection] = useState<SidebarSelection>({
    type: 'captures',
    label: 'All',
    query: {},
  });

  const refreshCaptures = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: ['captures'] });
  }, [queryClient]);

  const showToolbar = useCallback(async (captureId: string) => {
    const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
    if (toolbar) {
      await toolbar.emit('toolbar:show', { captureId });
      await toolbar.show();
      await toolbar.setFocus();
    }
  }, []);

  const handleFullScreen = useCallback(async () => {
    try {
      const capture = await captureFullScreen();
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e) {
      console.error('capture failed', e);
    }
  }, [refreshCaptures, showToolbar]);

  const handleRegion = useCallback(async () => {
    const overlay = await WebviewWindow.getByLabel('capture-overlay');
    if (overlay) {
      await overlay.show();
      await overlay.setFocus();
    }
  }, []);

  const handleWindow = useCallback(async () => {
    try {
      const { listCapturableWindows, captureWindow } = await import('@snk/capture');
      const windows = await listCapturableWindows();
      const target = windows.find(
        (w) => !w.app_name.includes('snapper-keeper') && w.title.length > 0,
      );
      if (!target) {
        console.warn('no capturable window found');
        return;
      }
      const capture = await captureWindow(target.id);
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e) {
      console.error('window capture failed', e);
    }
  }, [refreshCaptures, showToolbar]);

  const handleTimed = useCallback(async () => {
    setTimeout(async () => {
      try {
        const capture = await captureFullScreen();
        await refreshCaptures();
        await showToolbar(capture.id);
      } catch (e) {
        console.error('timed capture failed', e);
      }
    }, 5000);
  }, [refreshCaptures, showToolbar]);

  const handleClipboardHistory = useCallback(async () => {
    try {
      const pos = await showPopup();
      const popup = await WebviewWindow.getByLabel('clipboard-popup');
      if (popup) {
        await popup.setPosition(new LogicalPosition(pos.x, pos.y));
        await popup.emit(CLIPBOARD_POPUP_SHOW_EVENT, {});
        await popup.show();
        await popup.setFocus();
      }
    } catch (e) {
      console.error('clipboard popup failed', e);
    }
  }, []);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    const setup = async () => {
      unlisteners.push(await listen(CAPTURE_FULL_SCREEN_EVENT, handleFullScreen));
      unlisteners.push(await listen(CAPTURE_REGION_EVENT, handleRegion));
      unlisteners.push(await listen(CAPTURE_WINDOW_EVENT, handleWindow));
      unlisteners.push(await listen(CAPTURE_TIMED_EVENT, handleTimed));
      unlisteners.push(await listen(CLIPBOARD_HISTORY_EVENT, handleClipboardHistory));
    };
    setup().catch((e) => console.error('listen setup failed', e));
    return () => unlisteners.forEach((fn) => fn());
  }, [handleFullScreen, handleRegion, handleWindow, handleTimed, handleClipboardHistory]);

  return (
    <main className="h-full flex">
      <Sidebar selection={selection} onSelect={setSelection} />
      <div className="flex-1 flex flex-col min-w-0">
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
        <section className="flex-1 overflow-auto p-4">
          {selection.type === 'captures' ? (
            <CaptureGrid query={selection.query} />
          ) : (
            <ClipboardList />
          )}
        </section>
      </div>
    </main>
  );
}
```

**Step 5: Verify TypeScript compiles**

Run: `pnpm -r exec tsc --noEmit`
Expected: error — `ClipboardList` not found yet (created in Task 8). Create a stub:

Create `app/src/windows/library/ClipboardList.tsx`:

```tsx
export function ClipboardList() {
  return <div className="text-slate-500 text-sm">Clipboard history — coming in Task 8</div>;
}
```

Re-run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 6: Commit**

```bash
git add app/src/windows/library/Sidebar.tsx app/src/windows/library/CaptureGrid.tsx app/src/windows/library/LibraryWindow.tsx app/src/windows/library/ClipboardList.tsx app/src/lib/queryKeys.ts
git commit -m "feat(ui): add library sidebar with smart sections, tag filter, and clipboard entry"
```

---

### Task 8: Clipboard list component

**Files:**
- Modify: `app/src/windows/library/ClipboardList.tsx`

**Depends on:** Task 7 (stub exists)

**Step 1: Implement the clipboard list**

Replace `app/src/windows/library/ClipboardList.tsx`:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { listClipboardItems, toggleClipboardPin } from '@snk/clipboard';
import type { ClipboardItem } from '@snk/clipboard';

import { queryKeys } from '../../lib/queryKeys';

function formatTimeAgo(ms: number): string {
  const seconds = Math.floor((Date.now() - ms) / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function kindIcon(kind: ClipboardItem['kind']): string {
  switch (kind) {
    case 'text':
      return 'T';
    case 'image':
      return '🖼';
    default:
      return '?';
  }
}

export function ClipboardList() {
  const queryClient = useQueryClient();
  const { data, isLoading, error } = useQuery({
    queryKey: queryKeys.clipboard.list(),
    queryFn: () => listClipboardItems(),
  });

  const handleTogglePin = async (item: ClipboardItem) => {
    await toggleClipboardPin(item.id, !item.pinned);
    await queryClient.invalidateQueries({ queryKey: queryKeys.clipboard.list() });
  };

  if (isLoading) return <p className="text-slate-500">Loading…</p>;
  if (error) return <p className="text-red-400">Error: {String(error)}</p>;

  const items = data ?? [];
  if (items.length === 0) {
    return (
      <div className="text-slate-500 text-sm">
        No clipboard items yet. Copy something to get started.
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {items.map((item) => (
        <div
          key={item.id}
          className="flex items-start gap-3 p-3 rounded-md bg-slate-900 border border-slate-800 hover:border-slate-700"
        >
          <span className="text-lg w-6 text-center shrink-0">{kindIcon(item.kind)}</span>
          <div className="flex-1 min-w-0">
            <div className="text-sm text-slate-200 truncate">
              {item.text_content
                ? item.text_content.slice(0, 120)
                : item.kind === 'image'
                  ? '(image)'
                  : '(empty)'}
            </div>
            <div className="text-[10px] text-slate-500 mt-0.5">
              {item.source_app ?? 'unknown'} · {formatTimeAgo(item.created_at)}
            </div>
          </div>
          <button
            className={`text-xs px-1.5 py-0.5 rounded ${
              item.pinned
                ? 'bg-amber-900 text-amber-300'
                : 'text-slate-500 hover:text-slate-300'
            }`}
            onClick={() => handleTogglePin(item)}
            title={item.pinned ? 'Unpin' : 'Pin'}
          >
            {item.pinned ? 'pinned' : 'pin'}
          </button>
        </div>
      ))}
    </div>
  );
}
```

**Step 2: Verify TypeScript compiles**

Run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 3: Verify lint passes**

Run: `pnpm --filter @snk/app lint`
Expected: clean

**Step 4: Commit**

```bash
git add app/src/windows/library/ClipboardList.tsx
git commit -m "feat(ui): add clipboard history list component for library sidebar view"
```

---

### Task 9: Tag management UI

**Files:**
- Create: `app/src/windows/library/TagDialog.tsx`
- Modify: `app/src/windows/library/Thumbnail.tsx`

**Depends on:** Tasks 6, 7

**Step 1: Create the tag dialog component**

Create `app/src/windows/library/TagDialog.tsx`:

```tsx
import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { createTag, updateTag, deleteTag, listTags } from '@snk/library';
import type { Tag } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

const PRESET_COLORS = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#3b82f6', '#8b5cf6', '#ec4899', '#64748b'];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function TagDialog({ open, onClose }: Props) {
  const queryClient = useQueryClient();
  const { data: tags } = useQuery({
    queryKey: queryKeys.tags.list(),
    queryFn: () => listTags(),
  });
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [color, setColor] = useState(PRESET_COLORS[0]!);

  if (!open) return null;

  const startEdit = (tag: Tag) => {
    setEditingId(tag.id);
    setName(tag.name);
    setColor(tag.color);
  };

  const startCreate = () => {
    setEditingId(null);
    setName('');
    setColor(PRESET_COLORS[0]!);
  };

  const handleSave = async () => {
    if (!name.trim()) return;
    if (editingId) {
      await updateTag(editingId, name.trim(), color);
    } else {
      await createTag(name.trim(), color);
    }
    await queryClient.invalidateQueries({ queryKey: queryKeys.tags.list() });
    setName('');
    setEditingId(null);
  };

  const handleDelete = async (id: string) => {
    await deleteTag(id);
    await queryClient.invalidateQueries({ queryKey: queryKeys.tags.list() });
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-slate-900 border border-slate-700 rounded-lg w-80 p-4">
        <div className="flex justify-between items-center mb-3">
          <h2 className="text-sm font-semibold text-slate-100">Manage Tags</h2>
          <button className="text-slate-400 hover:text-slate-200 text-xs" onClick={onClose}>
            Close
          </button>
        </div>

        <div className="space-y-1 mb-3 max-h-48 overflow-y-auto">
          {(tags ?? []).map((tag) => (
            <div
              key={tag.id}
              className="flex items-center gap-2 px-2 py-1 rounded hover:bg-slate-800 group"
            >
              <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: tag.color }} />
              <span className="text-sm text-slate-200 flex-1">{tag.name}</span>
              <button
                className="text-[10px] text-slate-500 hover:text-slate-300 opacity-0 group-hover:opacity-100"
                onClick={() => startEdit(tag)}
              >
                edit
              </button>
              <button
                className="text-[10px] text-red-500 hover:text-red-300 opacity-0 group-hover:opacity-100"
                onClick={() => handleDelete(tag.id)}
              >
                delete
              </button>
            </div>
          ))}
        </div>

        <div className="border-t border-slate-800 pt-3">
          <div className="text-[10px] text-slate-500 mb-1">{editingId ? 'Edit tag' : 'New tag'}</div>
          <input
            className="w-full bg-slate-800 text-slate-100 text-sm px-2 py-1 rounded border border-slate-700 mb-2"
            placeholder="Tag name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSave();
            }}
          />
          <div className="flex gap-1 mb-2">
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                className={`w-5 h-5 rounded-full border-2 ${
                  color === c ? 'border-white' : 'border-transparent'
                }`}
                style={{ backgroundColor: c }}
                onClick={() => setColor(c)}
              />
            ))}
          </div>
          <div className="flex gap-2">
            <button
              className="bg-slate-700 hover:bg-slate-600 text-slate-100 text-xs px-3 py-1 rounded flex-1"
              onClick={handleSave}
            >
              {editingId ? 'Update' : 'Create'}
            </button>
            {editingId && (
              <button
                className="text-xs text-slate-400 hover:text-slate-200 px-2"
                onClick={startCreate}
              >
                Cancel
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
```

**Step 2: Add tag assignment to Thumbnail**

Update `app/src/windows/library/Thumbnail.tsx` to add a right-click context menu for tag assignment:

```tsx
import { useState, useRef, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { Capture } from '@snk/library';
import { listTags, listCaptureTags, assignTag, removeTag } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

interface Props {
  capture: Capture;
  src: string;
}

export function Thumbnail({ capture, src }: Props) {
  const [loaded, setLoaded] = useState(false);
  const [menuPos, setMenuPos] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const allTags = useQuery({
    queryKey: queryKeys.tags.list(),
    queryFn: () => listTags(),
    enabled: menuPos !== null,
  });

  const captureTags = useQuery({
    queryKey: queryKeys.tags.forCapture(capture.id),
    queryFn: () => listCaptureTags(capture.id),
    enabled: menuPos !== null,
  });

  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuPos(null);
      }
    };
    if (menuPos) document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [menuPos]);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setMenuPos({ x: e.clientX, y: e.clientY });
  };

  const handleToggleTag = async (tagId: string, assigned: boolean) => {
    if (assigned) {
      await removeTag(capture.id, tagId);
    } else {
      await assignTag(capture.id, tagId);
    }
    await queryClient.invalidateQueries({ queryKey: queryKeys.tags.forCapture(capture.id) });
    await queryClient.invalidateQueries({ queryKey: ['captures'] });
  };

  const assignedIds = new Set((captureTags.data ?? []).map((t) => t.id));

  return (
    <>
      <div
        className="bg-slate-900 border border-slate-800 rounded-md overflow-hidden"
        onContextMenu={handleContextMenu}
      >
        <div className="relative aspect-video bg-slate-950">
          <img
            src={src}
            alt={`Capture ${capture.id}`}
            onLoad={() => setLoaded(true)}
            className={`w-full h-full object-cover transition-opacity ${
              loaded ? 'opacity-100' : 'opacity-0'
            }`}
          />
        </div>
        <div className="px-2 py-1.5">
          <div className="text-xs text-slate-200 truncate">
            {new Date(capture.created_at).toLocaleTimeString()}
          </div>
          <div className="text-[10px] text-slate-500 truncate">
            {capture.width}x{capture.height}
            {capture.monitor ? ` · ${capture.monitor}` : ''}
            {capture.source_app ? ` · ${capture.source_app}` : ''}
          </div>
          {capture.annotated_path && (
            <div className="text-[10px] text-blue-400 truncate">annotated</div>
          )}
        </div>
      </div>

      {menuPos && (
        <div
          ref={menuRef}
          className="fixed bg-slate-800 border border-slate-700 rounded-md shadow-lg py-1 z-50 min-w-[140px]"
          style={{ left: menuPos.x, top: menuPos.y }}
        >
          <div className="text-[10px] text-slate-500 px-3 py-1">Tags</div>
          {(allTags.data ?? []).length === 0 ? (
            <div className="text-xs text-slate-500 px-3 py-1">No tags created</div>
          ) : (
            (allTags.data ?? []).map((tag) => (
              <button
                key={tag.id}
                className="w-full text-left px-3 py-1 text-sm text-slate-200 hover:bg-slate-700 flex items-center gap-2"
                onClick={() => handleToggleTag(tag.id, assignedIds.has(tag.id))}
              >
                <span
                  className="w-2.5 h-2.5 rounded-full shrink-0"
                  style={{ backgroundColor: tag.color }}
                />
                <span className="flex-1">{tag.name}</span>
                {assignedIds.has(tag.id) && <span className="text-green-400 text-xs">✓</span>}
              </button>
            ))
          )}
        </div>
      )}
    </>
  );
}
```

**Step 3: Add "Manage Tags" button to Sidebar**

In `app/src/windows/library/Sidebar.tsx`, add the dialog trigger. Import `TagDialog` and add state to the `Sidebar` component:

Add this import at the top:

```tsx
import { TagDialog } from './TagDialog';
```

Add state inside the `Sidebar` component, after the `tagsQuery`:

```tsx
  const [tagDialogOpen, setTagDialogOpen] = useState(false);
```

Add the `useState` import (update the import line at the top of the file from):

```tsx
import { useQuery } from '@tanstack/react-query';
```

to:

```tsx
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
```

After the Tags heading div, before the tags list, add:

```tsx
        <button
          className="text-[10px] text-slate-500 hover:text-slate-300 px-3 mb-1"
          onClick={() => setTagDialogOpen(true)}
        >
          Manage tags
        </button>
```

At the end of the component, before the closing `</aside>`, add:

```tsx
      <TagDialog open={tagDialogOpen} onClose={() => setTagDialogOpen(false)} />
```

**Step 4: Verify TypeScript compiles**

Run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 5: Verify lint passes**

Run: `pnpm --filter @snk/app lint`
Expected: clean

**Step 6: Commit**

```bash
git add app/src/windows/library/TagDialog.tsx app/src/windows/library/Thumbnail.tsx app/src/windows/library/Sidebar.tsx
git commit -m "feat(ui): add tag management dialog and capture context menu for tag assignment"
```

---

### Task 10: Settings window

**Files:**
- Create: `app/src/windows/settings/SettingsWindow.tsx`
- Modify: `app/src-tauri/tauri.conf.json`
- Modify: `app/src/App.tsx`
- Modify: `app/src-tauri/src/main.rs`

**Depends on:** Tasks 5, 6

**Step 1: Add settings window to tauri.conf.json**

In `app/src-tauri/tauri.conf.json`, add to the `app.windows` array after the `clipboard-popup` entry:

```json
      {
        "label": "settings",
        "title": "Settings",
        "width": 600,
        "height": 500,
        "minWidth": 400,
        "minHeight": 400,
        "resizable": true,
        "visible": false,
        "decorations": true,
        "skipTaskbar": false
      }
```

**Step 2: Add settings route to App.tsx**

In `app/src/App.tsx`, add the import:

```tsx
import { SettingsWindow } from './windows/settings/SettingsWindow';
```

Add the case in the `switch (label)`:

```tsx
    case 'settings':
      return <SettingsWindow />;
```

**Step 3: Add tray Settings menu item**

In `app/src-tauri/src/main.rs`, add a Settings menu item. After the `open_lib` line:

```rust
            let settings =
                MenuItem::with_id(app, "tray:settings", "Settings…", true, None::<&str>)?;
```

Update the `Menu::with_items` call to include `&settings` between `&open_lib` and `&quit`:

```rust
            let menu = Menu::with_items(
                app,
                &[
                    &capture_region,
                    &capture_window,
                    &capture_screen,
                    &capture_timed,
                    &clipboard_hist,
                    &sep,
                    &open_lib,
                    &settings,
                    &quit,
                ],
            )?;
```

Add the handler in the `on_menu_event` match:

```rust
                    "tray:settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
```

**Step 4: Create the SettingsWindow component**

Create `app/src/windows/settings/SettingsWindow.tsx`:

```tsx
import { useState, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { getSetting, setSetting } from '@snk/library';

import { queryKeys } from '../../lib/queryKeys';

interface SettingRowProps {
  label: string;
  description?: string;
  children: React.ReactNode;
}

function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-2">
      <div>
        <div className="text-sm text-slate-200">{label}</div>
        {description && <div className="text-[10px] text-slate-500">{description}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      className={`w-9 h-5 rounded-full relative transition-colors ${value ? 'bg-blue-600' : 'bg-slate-600'}`}
      onClick={() => onChange(!value)}
    >
      <span
        className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${value ? 'translate-x-4' : 'translate-x-0.5'}`}
      />
    </button>
  );
}

function useSetting<T>(key: string, defaultValue: T): [T, (v: T) => void, boolean] {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: queryKeys.settings.one(key),
    queryFn: () => getSetting(key),
  });

  const value = data !== null && data !== undefined ? (data as T) : defaultValue;

  const update = (v: T) => {
    setSetting(key, v).then(() => {
      queryClient.invalidateQueries({ queryKey: queryKeys.settings.one(key) });
    });
  };

  return [value, update, isLoading];
}

export function SettingsWindow() {
  const [captureFormat, setCaptureFormat] = useSetting('capture.format', 'png');
  const [autoCopy, setAutoCopy] = useSetting('capture.auto_copy', true);
  const [jpgQuality, setJpgQuality] = useSetting('capture.jpg_quality', 90);
  const [historySize, setHistorySize] = useSetting('clipboard.history_size', 200);
  const [trackImages, setTrackImages] = useSetting('clipboard.track_images', true);
  const [trackFiles, setTrackFiles] = useSetting('clipboard.track_files', true);
  const [ocrEnabled, setOcrEnabled] = useSetting('ocr.enabled', true);

  return (
    <main className="h-full flex flex-col bg-slate-950 text-slate-100">
      <header className="px-4 py-3 border-b border-slate-800">
        <h1 className="text-sm font-semibold">Settings</h1>
      </header>
      <div className="flex-1 overflow-auto p-4 space-y-6">
        <section>
          <h2 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Capture</h2>
          <div className="bg-slate-900 rounded-lg border border-slate-800 px-3 divide-y divide-slate-800">
            <SettingRow label="Format">
              <select
                className="bg-slate-800 text-sm text-slate-200 px-2 py-1 rounded border border-slate-700"
                value={captureFormat as string}
                onChange={(e) => setCaptureFormat(e.target.value)}
              >
                <option value="png">PNG</option>
                <option value="jpg">JPG</option>
                <option value="webp">WebP</option>
              </select>
            </SettingRow>
            <SettingRow label="Auto-copy to clipboard" description="Copy capture to clipboard immediately after capture">
              <Toggle value={autoCopy as boolean} onChange={setAutoCopy} />
            </SettingRow>
            {captureFormat === 'jpg' && (
              <SettingRow label="JPG quality" description="1–100">
                <input
                  type="number"
                  className="bg-slate-800 text-sm text-slate-200 w-16 px-2 py-1 rounded border border-slate-700"
                  min={1}
                  max={100}
                  value={jpgQuality as number}
                  onChange={(e) => setJpgQuality(Number(e.target.value))}
                />
              </SettingRow>
            )}
          </div>
        </section>

        <section>
          <h2 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Clipboard</h2>
          <div className="bg-slate-900 rounded-lg border border-slate-800 px-3 divide-y divide-slate-800">
            <SettingRow label="History size" description="Maximum number of clipboard items to keep">
              <input
                type="number"
                className="bg-slate-800 text-sm text-slate-200 w-20 px-2 py-1 rounded border border-slate-700"
                min={10}
                max={1000}
                value={historySize as number}
                onChange={(e) => setHistorySize(Number(e.target.value))}
              />
            </SettingRow>
            <SettingRow label="Track images" description="Store copied images in clipboard history">
              <Toggle value={trackImages as boolean} onChange={setTrackImages} />
            </SettingRow>
            <SettingRow label="Track files" description="Store copied file references in clipboard history">
              <Toggle value={trackFiles as boolean} onChange={setTrackFiles} />
            </SettingRow>
          </div>
        </section>

        <section>
          <h2 className="text-xs uppercase tracking-wider text-slate-500 mb-2">OCR</h2>
          <div className="bg-slate-900 rounded-lg border border-slate-800 px-3 divide-y divide-slate-800">
            <SettingRow label="Enable OCR" description="Automatically extract text from captures using Tesseract">
              <Toggle value={ocrEnabled as boolean} onChange={setOcrEnabled} />
            </SettingRow>
          </div>
        </section>
      </div>
    </main>
  );
}
```

**Step 5: Verify TypeScript compiles**

Run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 6: Verify lint passes**

Run: `pnpm --filter @snk/app lint`
Expected: clean

**Step 7: Verify Rust compiles**

Run: `cargo build -p snapper-keeper-app`
Expected: success

**Step 8: Commit**

```bash
git add app/src/windows/settings/SettingsWindow.tsx app/src/App.tsx app/src-tauri/tauri.conf.json app/src-tauri/src/main.rs
git commit -m "feat(app): add settings window with capture, clipboard, and OCR configuration"
```

---

### Task 11: First-run wizard

**Files:**
- Create: `app/src/windows/library/FirstRunWizard.tsx`
- Modify: `app/src/windows/library/LibraryWindow.tsx`

**Depends on:** Tasks 6 (settings bindings)

**Step 1: Create the wizard component**

Create `app/src/windows/library/FirstRunWizard.tsx`:

```tsx
import { useState } from 'react';

import { setSetting } from '@snk/library';

type Step = 'welcome' | 'hotkeys' | 'library' | 'done';

const DEFAULT_HOTKEYS = [
  { action: 'Capture region', chord: 'Ctrl+Shift+4' },
  { action: 'Capture window', chord: 'Ctrl+Shift+5' },
  { action: 'Capture screen', chord: 'Ctrl+Shift+3' },
  { action: 'Timed capture', chord: 'Ctrl+Shift+6' },
  { action: 'Clipboard history', chord: 'Ctrl+Shift+V' },
  { action: 'Open library', chord: 'Ctrl+Shift+L' },
];

interface Props {
  onComplete: () => void;
}

export function FirstRunWizard({ onComplete }: Props) {
  const [step, setStep] = useState<Step>('welcome');

  const finish = async () => {
    await setSetting('firstrun.completed', true);
    onComplete();
  };

  return (
    <div className="fixed inset-0 bg-slate-950 flex items-center justify-center z-50">
      <div className="max-w-md w-full mx-4">
        {step === 'welcome' && (
          <div className="text-center space-y-4">
            <h1 className="text-xl font-semibold text-slate-100">Welcome to snapper-keeper</h1>
            <p className="text-sm text-slate-400">
              Screen capture with OCR search, plus clipboard history with instant paste.
              Let&apos;s get you set up.
            </p>
            <button
              className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
              onClick={() => setStep('hotkeys')}
            >
              Get started
            </button>
          </div>
        )}

        {step === 'hotkeys' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-slate-100">Keyboard shortcuts</h2>
            <p className="text-sm text-slate-400">
              These are the default hotkeys. You can change them anytime in Settings.
            </p>
            <div className="bg-slate-900 rounded-lg border border-slate-800 divide-y divide-slate-800">
              {DEFAULT_HOTKEYS.map((hk) => (
                <div key={hk.action} className="flex justify-between px-4 py-2">
                  <span className="text-sm text-slate-200">{hk.action}</span>
                  <kbd className="text-xs bg-slate-800 text-slate-300 px-2 py-0.5 rounded border border-slate-700">
                    {hk.chord}
                  </kbd>
                </div>
              ))}
            </div>
            <div className="flex justify-between">
              <button
                className="text-sm text-slate-400 hover:text-slate-200"
                onClick={() => setStep('welcome')}
              >
                Back
              </button>
              <button
                className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
                onClick={() => setStep('library')}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {step === 'library' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-slate-100">Library location</h2>
            <p className="text-sm text-slate-400">
              Your captures, clipboard history, and settings are stored locally.
              No cloud, no servers, no telemetry.
            </p>
            <div className="bg-slate-900 rounded-lg border border-slate-800 px-4 py-3">
              <div className="text-[10px] text-slate-500">Storage location</div>
              <div className="text-sm text-slate-200 font-mono">%APPDATA%/snapper-keeper/</div>
            </div>
            <div className="flex justify-between">
              <button
                className="text-sm text-slate-400 hover:text-slate-200"
                onClick={() => setStep('hotkeys')}
              >
                Back
              </button>
              <button
                className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
                onClick={() => setStep('done')}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {step === 'done' && (
          <div className="text-center space-y-4">
            <h2 className="text-lg font-semibold text-slate-100">All set!</h2>
            <p className="text-sm text-slate-400">
              You&apos;re ready to go. Try pressing Ctrl+Shift+4 to capture a region,
              or Ctrl+Shift+V to open clipboard history.
            </p>
            <button
              className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-6 py-2 rounded"
              onClick={finish}
            >
              Start using snapper-keeper
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
```

**Step 2: Gate the library window on first-run**

In `app/src/windows/library/LibraryWindow.tsx`, add the first-run check. Add these imports at the top:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';
```

(Replace the existing `import { useQueryClient } from '@tanstack/react-query';`)

Add the import:

```tsx
import { getSetting } from '@snk/library';
```

And:

```tsx
import { FirstRunWizard } from './FirstRunWizard';
```

Inside the `LibraryWindow` component, after the `queryClient` declaration:

```tsx
  const firstRun = useQuery({
    queryKey: queryKeys.settings.one('firstrun.completed'),
    queryFn: () => getSetting('firstrun.completed'),
  });

  const [wizardDismissed, setWizardDismissed] = useState(false);
  const showWizard = !wizardDismissed && firstRun.data !== true;
```

(Add `queryKeys` import from `../../lib/queryKeys` if not already imported.)

In the return JSX, wrap the existing content:

```tsx
  return (
    <>
      {showWizard && !firstRun.isLoading && (
        <FirstRunWizard
          onComplete={() => {
            setWizardDismissed(true);
            queryClient.invalidateQueries({ queryKey: queryKeys.settings.one('firstrun.completed') });
          }}
        />
      )}
      <main className="h-full flex">
        {/* ... existing sidebar + main content ... */}
      </main>
    </>
  );
```

**Step 3: Verify TypeScript compiles**

Run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 4: Verify lint passes**

Run: `pnpm --filter @snk/app lint`
Expected: clean

**Step 5: Commit**

```bash
git add app/src/windows/library/FirstRunWizard.tsx app/src/windows/library/LibraryWindow.tsx
git commit -m "feat(ui): add first-run wizard with hotkey confirmation and library path overview"
```

---

### Task 12: Full integration verification

**Files:** None (verification only)

**Step 1: Run all Rust tests**

Run: `cargo test --workspace`
Expected: all pass

**Step 2: TypeScript type-check**

Run: `pnpm -r exec tsc --noEmit`
Expected: clean

**Step 3: ESLint**

Run: `pnpm --filter @snk/app lint`
Expected: no errors, 0 warnings

**Step 4: Build the app**

Run: `cargo build -p snapper-keeper-app`
Expected: success

**Step 5: Fix any issues found**

If any check fails, fix the issue and re-run all checks.

**Step 6: Commit fixes (if any)**

```bash
git add <fixed-files>
git commit -m "fix: integration fixes for Phase 6"
```
