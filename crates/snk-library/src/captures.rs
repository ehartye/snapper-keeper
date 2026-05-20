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
            width: 1, height: 1,
            source_app: None, source_window_title: None, monitor: None,
        };
        let c = insert(&db, new).unwrap();
        db.with_conn(|conn| {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM captures WHERE id = ?1", [&c.id], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    }
}
