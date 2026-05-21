use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capture {
    pub id: String,
    pub file_path: String,
    pub annotated_path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
    pub monitor: Option<String>,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCapture {
    pub file_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
    pub monitor: Option<String>,
}

pub fn insert(db: &Db, new: NewCapture) -> Result<Capture> {
    let id = Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().timestamp_millis();
    let file_path = new
        .file_path
        .to_str()
        .ok_or_else(|| crate::LibraryError::Io {
            path: new.file_path.display().to_string(),
            reason: "non-utf8 path".into(),
        })?
        .to_string();

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

    Ok(Capture {
        id,
        file_path,
        annotated_path: None,
        width: new.width,
        height: new.height,
        source_app: new.source_app,
        source_window_title: new.source_window_title,
        monitor: new.monitor,
        created_at,
        deleted_at: None,
        pinned: false,
    })
}

fn row_to_capture(row: &rusqlite::Row<'_>) -> rusqlite::Result<Capture> {
    Ok(Capture {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        annotated_path: row.get("annotated_path")?,
        width: row.get::<_, i64>("width")? as u32,
        height: row.get::<_, i64>("height")? as u32,
        source_app: row.get("source_app")?,
        source_window_title: row.get("source_window_title")?,
        monitor: row.get("monitor")?,
        created_at: row.get("created_at")?,
        deleted_at: row.get("deleted_at")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
    })
}

pub fn annotated_relative_path(original: &str) -> PathBuf {
    let p = PathBuf::from(original);
    match p.extension().and_then(|e| e.to_str()) {
        Some(ext) => p.with_extension(format!("annotated.{ext}")),
        None => PathBuf::from(format!("{original}.annotated")),
    }
}

pub fn get(db: &Db, id: &str) -> Result<Capture> {
    db.with_conn(|conn| {
        let row = conn
            .query_row(
                "SELECT * FROM captures WHERE id = ?1 AND deleted_at IS NULL",
                [id],
                row_to_capture,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => crate::LibraryError::NotFound {
                    what: format!("capture {id}"),
                },
                other => other.into(),
            })?;
        Ok(row)
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListCapturesQuery {
    pub limit: Option<u32>,
    pub include_deleted: bool,
}

pub fn list(db: &Db, q: ListCapturesQuery) -> Result<Vec<Capture>> {
    let limit = q.limit.unwrap_or(200).min(1000);
    db.with_conn(|conn| {
        let sql = if q.include_deleted {
            "SELECT * FROM captures ORDER BY created_at DESC LIMIT ?1"
        } else {
            "SELECT * FROM captures WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map([limit], row_to_capture)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

pub fn soft_delete(db: &Db, id: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE captures SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![now, id],
        )?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("capture {id}"),
            });
        }
        Ok(())
    })
}

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
    fn insert_returns_capture_with_uuid_v7() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("captures/2026/05/x.png"),
            width: 1920,
            height: 1080,
            source_app: Some("Firefox".into()),
            source_window_title: Some("github".into()),
            monitor: Some("Monitor 0".into()),
        };
        let c = insert(&db, new).unwrap();
        assert_eq!(c.width, 1920);
        assert_eq!(c.height, 1080);
        assert_eq!(c.source_app.as_deref(), Some("Firefox"));
        // UUIDv7 strings are 36 chars
        assert_eq!(c.id.len(), 36);
        assert!(!c.pinned);
        assert!(c.deleted_at.is_none());
    }

    #[test]
    fn insert_creates_a_persisted_row() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("x.png"),
            width: 1,
            height: 1,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let c = insert(&db, new).unwrap();
        db.with_conn(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM captures WHERE id = ?1",
                    [&c.id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn get_returns_inserted() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("a.png"),
            width: 10,
            height: 10,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let inserted = insert(&db, new).unwrap();
        let fetched = get(&db, &inserted.id).unwrap();
        assert_eq!(fetched, inserted);
    }

    #[test]
    fn get_returns_not_found_for_missing_id() {
        let db = fresh_db();
        match get(&db, "no-such-id") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn list_returns_newest_first_and_excludes_deleted() {
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
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = insert(&db, mk(2)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let c = insert(&db, mk(3)).unwrap();

        // Soft-delete `b`
        db.with_conn(|conn| {
            conn.execute(
                "UPDATE captures SET deleted_at=?1 WHERE id=?2",
                rusqlite::params![1_i64, &b.id],
            )?;
            Ok(())
        })
        .unwrap();

        let rows = list(&db, ListCapturesQuery::default()).unwrap();
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec![c.id.as_str(), a.id.as_str()]);
    }

    #[test]
    fn soft_delete_sets_deleted_at() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("del.png"),
            width: 1,
            height: 1,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let c = insert(&db, new).unwrap();
        assert!(c.deleted_at.is_none());

        soft_delete(&db, &c.id).unwrap();

        let rows = list(
            &db,
            ListCapturesQuery {
                limit: None,
                include_deleted: true,
            },
        )
        .unwrap();
        let found = rows.iter().find(|r| r.id == c.id).unwrap();
        assert!(found.deleted_at.is_some());
    }

    #[test]
    fn soft_delete_nonexistent_returns_not_found() {
        let db = fresh_db();
        match soft_delete(&db, "no-such-id") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

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
}
