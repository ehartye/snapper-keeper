use std::fmt;
use std::path::PathBuf;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Db, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardItemKind {
    Text,
    Image,
}

impl ClipboardItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ClipboardItemKind::Text => "text",
            ClipboardItemKind::Image => "image",
        }
    }
}

impl fmt::Display for ClipboardItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ToSql for ClipboardItemKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for ClipboardItemKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "text" => Ok(ClipboardItemKind::Text),
            "image" => Ok(ClipboardItemKind::Image),
            other => Err(FromSqlError::Other(
                format!("unknown clipboard_item kind: {other}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardItem {
    pub id: String,
    pub kind: ClipboardItemKind,
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
    pub kind: ClipboardItemKind,
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

    crate::search::index_clipboard(
        db,
        &id,
        new.text_content.as_deref(),
        new.source_app.as_deref(),
        new.source_window_title.as_deref(),
    )?;

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
            kind: ClipboardItemKind::Text,
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
        assert_eq!(item.kind, ClipboardItemKind::Text);
        assert_eq!(item.content_hash, "abc123");
        assert!(!item.pinned);

        let fetched = get(&db, &item.id).unwrap();
        assert_eq!(fetched, item);
    }

    #[test]
    fn kind_round_trips_through_sqlite() {
        let db = fresh_db();
        let mut img = sample_item("img-hash");
        img.kind = ClipboardItemKind::Image;
        let inserted = insert(&db, img).unwrap();
        assert_eq!(inserted.kind, ClipboardItemKind::Image);
        let fetched = get(&db, &inserted.id).unwrap();
        assert_eq!(fetched.kind, ClipboardItemKind::Image);
    }

    #[test]
    fn kind_serializes_as_lowercase_string() {
        let json = serde_json::to_string(&ClipboardItemKind::Text).unwrap();
        assert_eq!(json, "\"text\"");
        let json = serde_json::to_string(&ClipboardItemKind::Image).unwrap();
        assert_eq!(json, "\"image\"");
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

        // Evict to keep only 1 unpinned — should remove b (oldest unpinned)
        evict_unpinned(&db, 1).unwrap();

        let items = list(
            &db,
            ListClipboardQuery {
                limit: None,
                filter: None,
            },
        )
        .unwrap();
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&a.id.as_str())); // pinned, kept
        assert!(ids.contains(&c.id.as_str())); // newest unpinned, kept
        assert!(!ids.contains(&b.id.as_str())); // oldest unpinned, evicted
    }
}
