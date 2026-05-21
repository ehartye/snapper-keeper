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
}
