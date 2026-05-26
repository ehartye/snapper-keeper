use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub bbox: BBox,
    pub confidence: f64,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrText {
    pub capture_id: String,
    pub text: String,
    pub language: String,
    pub confidence: f64,
    pub created_at: i64,
    pub engine: String,
    pub words: Option<Vec<OcrWord>>,
}

/// Legacy upsert — does not populate words_json or engine. Kept so existing
/// call-sites (and tests) don't break during the transition. New OCR pipeline
/// code MUST call `upsert_full` instead.
pub fn upsert(
    db: &Db,
    capture_id: &str,
    text: &str,
    language: &str,
    confidence: f64,
) -> Result<()> {
    upsert_full(db, capture_id, text, language, confidence, &[], "")
}

pub fn upsert_full(
    db: &Db,
    capture_id: &str,
    text: &str,
    language: &str,
    confidence: f64,
    words: &[OcrWord],
    engine: &str,
) -> Result<()> {
    let created_at = chrono::Utc::now().timestamp_millis();
    let words_json = if words.is_empty() {
        None
    } else {
        Some(serde_json::to_string(words).map_err(|e| crate::LibraryError::Persist {
            detail: format!("serialize words_json: {e}"),
        })?)
    };
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO ocr_text (capture_id, text, language, confidence, created_at, words_json, engine)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(capture_id) DO UPDATE SET
                text = excluded.text,
                language = excluded.language,
                confidence = excluded.confidence,
                created_at = excluded.created_at,
                words_json = excluded.words_json,
                engine = excluded.engine",
            rusqlite::params![capture_id, text, language, confidence, created_at, words_json, engine],
        )?;
        Ok(())
    })
}

pub fn get(db: &Db, capture_id: &str) -> Result<Option<OcrText>> {
    db.with_conn(|conn| {
        let result = conn.query_row(
            "SELECT capture_id, text, language, confidence, created_at, words_json, engine
             FROM ocr_text WHERE capture_id = ?1",
            [capture_id],
            |row| {
                let words_json: Option<String> = row.get(5)?;
                let words = words_json
                    .map(|s| serde_json::from_str::<Vec<OcrWord>>(&s))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                Ok(OcrText {
                    capture_id: row.get(0)?,
                    text: row.get(1)?,
                    language: row.get(2)?,
                    confidence: row.get(3)?,
                    created_at: row.get(4)?,
                    engine: row.get(6)?,
                    words,
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

    #[test]
    fn upsert_persists_words_json_and_engine() {
        let (_tmp, db) = fresh_db();
        let cap_id = insert_capture(&db);
        let words = vec![
            OcrWord {
                text: "hello".into(),
                bbox: BBox {
                    x: 0.1,
                    y: 0.05,
                    w: 0.08,
                    h: 0.04,
                },
                confidence: 0.97,
                line: 0,
            },
            OcrWord {
                text: "world".into(),
                bbox: BBox {
                    x: 0.19,
                    y: 0.05,
                    w: 0.08,
                    h: 0.04,
                },
                confidence: 0.95,
                line: 0,
            },
        ];
        upsert_full(
            &db,
            &cap_id,
            "hello world",
            "eng",
            0.95,
            &words,
            "Vision (test)",
        )
        .unwrap();

        let row = get(&db, &cap_id).unwrap().unwrap();
        assert_eq!(row.text, "hello world");
        assert_eq!(row.engine, "Vision (test)");
        let parsed = row.words.expect("words populated");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].text, "hello");
        assert!((parsed[0].confidence - 0.97).abs() < 1e-6);
    }

    #[test]
    fn legacy_upsert_leaves_words_null() {
        let (_tmp, db) = fresh_db();
        let cap_id = insert_capture(&db);
        upsert(&db, &cap_id, "legacy text", "eng", 0.8).unwrap();
        let row = get(&db, &cap_id).unwrap().unwrap();
        assert!(row.words.is_none(), "legacy upsert must leave words_json NULL");
        assert_eq!(row.engine, "");
    }
}
