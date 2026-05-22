use serde_json::Value;

use crate::{Db, Result};

pub fn get(db: &Db, key: &str) -> Result<Option<Value>> {
    db.with_conn(|conn| {
        let result = conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        });
        match result {
            Ok(raw) => {
                let val: Value =
                    serde_json::from_str(&raw).map_err(|e| crate::LibraryError::Database {
                        message: format!("invalid JSON in setting {key}: {e}"),
                        retryable: false,
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

    #[test]
    fn delete_unknown_key_is_a_noop() {
        let db = fresh_db();
        // Should not error even when the row doesn't exist.
        delete(&db, "never.set").unwrap();
    }

    #[test]
    fn get_returns_none_for_unset_key() {
        let db = fresh_db();
        assert_eq!(get(&db, "no.such").unwrap(), None);
    }

    #[test]
    fn get_corrupt_json_returns_database_error() {
        let db = fresh_db();
        // Bypass set() to plant a literal that isn't valid JSON.
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params!["broken", "not-json{"],
            )?;
            Ok(())
        })
        .unwrap();
        match get(&db, "broken") {
            Err(crate::LibraryError::Database { message, .. }) => {
                assert!(message.contains("invalid JSON"));
            }
            other => panic!("expected Database error, got {other:?}"),
        }
    }

    #[test]
    fn set_round_trips_complex_values() {
        let db = fresh_db();
        let v = json!({"a": 1, "b": [2, 3], "c": {"d": true}});
        set(&db, "nested", &v).unwrap();
        assert_eq!(get(&db, "nested").unwrap(), Some(v));
    }
}
