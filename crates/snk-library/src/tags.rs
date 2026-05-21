use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: i64,
}

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
        assert_eq!(t2.name, "feature");
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
}
