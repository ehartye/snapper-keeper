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

pub fn upsert(
    db: &Db,
    capture_id: &str,
    text: &str,
    language: &str,
    confidence: f64,
) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::test_support::fresh_db;

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
        let (_tmp, db) = fresh_db();
        let cap_id = insert_capture(&db);
        upsert(&db, &cap_id, "hello world", "eng", 0.95).unwrap();
        let row = get(&db, &cap_id).unwrap().unwrap();
        assert_eq!(row.text, "hello world");
        assert_eq!(row.language, "eng");
        assert!((row.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn upsert_replaces_existing_ocr_text() {
        let (_tmp, db) = fresh_db();
        let cap_id = insert_capture(&db);
        upsert(&db, &cap_id, "first", "eng", 0.8).unwrap();
        upsert(&db, &cap_id, "second", "eng", 0.9).unwrap();
        let row = get(&db, &cap_id).unwrap().unwrap();
        assert_eq!(row.text, "second");
        assert!((row.confidence - 0.9).abs() < 0.001);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let (_tmp, db) = fresh_db();
        let result = get(&db, "no-such-id").unwrap();
        assert!(result.is_none());
    }
}
