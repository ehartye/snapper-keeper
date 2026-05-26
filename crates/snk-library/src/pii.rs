use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PiiCategory {
    Email,
    Phone,
    CreditCard,
    Ssn,
    Ip,
    ApiKey,
}

impl PiiCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            PiiCategory::Email => "email",
            PiiCategory::Phone => "phone",
            PiiCategory::CreditCard => "credit_card",
            PiiCategory::Ssn => "ssn",
            PiiCategory::Ip => "ip",
            PiiCategory::ApiKey => "api_key",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PiiSpan {
    pub id: i64,
    pub capture_id: String,
    pub category: PiiCategory,
    pub matched_text: String,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_w: f32,
    pub bbox_h: f32,
    pub confidence: f64,
    pub redacted_at: Option<i64>,
    pub dismissed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewPiiSpan<'a> {
    pub capture_id: &'a str,
    pub category: PiiCategory,
    pub matched_text: &'a str,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_w: f32,
    pub bbox_h: f32,
    pub confidence: f64,
}

pub fn insert(db: &Db, span: NewPiiSpan<'_>) -> Result<i64> {
    let created_at = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO pii_spans
                (capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h, confidence, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                span.capture_id,
                span.category.as_str(),
                span.matched_text,
                span.bbox_x,
                span.bbox_y,
                span.bbox_w,
                span.bbox_h,
                span.confidence,
                created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    })
}

pub fn get(db: &Db, span_id: i64) -> Result<Option<PiiSpan>> {
    db.with_conn(|conn| {
        let res = conn.query_row(
            "SELECT id, capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h,
                    confidence, redacted_at, dismissed_at, created_at
             FROM pii_spans WHERE id = ?1",
            [span_id],
            row_to_span,
        );
        match res {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}

pub fn list_for_capture(db: &Db, capture_id: &str) -> Result<Vec<PiiSpan>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h,
                    confidence, redacted_at, dismissed_at, created_at
             FROM pii_spans WHERE capture_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([capture_id], row_to_span)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    })
}

pub fn list_pending_for_capture(db: &Db, capture_id: &str) -> Result<Vec<PiiSpan>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, capture_id, category, matched_text, bbox_x, bbox_y, bbox_w, bbox_h,
                    confidence, redacted_at, dismissed_at, created_at
             FROM pii_spans WHERE capture_id = ?1
                AND redacted_at IS NULL AND dismissed_at IS NULL ORDER BY id",
        )?;
        let rows = stmt.query_map([capture_id], row_to_span)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    })
}

pub fn mark_redacted(db: &Db, span_id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE pii_spans SET redacted_at = ?1 WHERE id = ?2",
            rusqlite::params![now, span_id],
        )?;
        Ok(())
    })
}

pub fn mark_dismissed(db: &Db, span_id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE pii_spans SET dismissed_at = ?1 WHERE id = ?2",
            rusqlite::params![now, span_id],
        )?;
        Ok(())
    })
}

fn row_to_span(row: &rusqlite::Row<'_>) -> rusqlite::Result<PiiSpan> {
    let category_str: String = row.get(2)?;
    let category = match category_str.as_str() {
        "email" => PiiCategory::Email,
        "phone" => PiiCategory::Phone,
        "credit_card" => PiiCategory::CreditCard,
        "ssn" => PiiCategory::Ssn,
        "ip" => PiiCategory::Ip,
        "api_key" => PiiCategory::ApiKey,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown PII category {other}"),
                )),
            ));
        }
    };
    Ok(PiiSpan {
        id: row.get(0)?,
        capture_id: row.get(1)?,
        category,
        matched_text: row.get(3)?,
        bbox_x: row.get(4)?,
        bbox_y: row.get(5)?,
        bbox_w: row.get(6)?,
        bbox_h: row.get(7)?,
        confidence: row.get(8)?,
        redacted_at: row.get(9)?,
        dismissed_at: row.get(10)?,
        created_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::test_support::fresh_db;
    use crate::{Db, NewCapture};

    fn cap(db: &Db) -> String {
        crate::captures::insert(
            db,
            NewCapture {
                file_path: PathBuf::from("test.png"),
                width: 100,
                height: 100,
                source_app: None,
                source_window_title: None,
                monitor: None,
            },
        )
        .unwrap()
        .id
    }

    fn span_for<'a>(c: &'a str, t: &'a str) -> NewPiiSpan<'a> {
        NewPiiSpan {
            capture_id: c,
            category: PiiCategory::Email,
            matched_text: t,
            bbox_x: 0.1,
            bbox_y: 0.1,
            bbox_w: 0.1,
            bbox_h: 0.05,
            confidence: 0.9,
        }
    }

    #[test]
    fn insert_then_get_round_trips() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let id = insert(&db, span_for(&c, "alice@example.com")).unwrap();
        let row = get(&db, id).unwrap().unwrap();
        assert_eq!(row.category, PiiCategory::Email);
        assert_eq!(row.matched_text, "alice@example.com");
        assert!(row.redacted_at.is_none());
        assert!(row.dismissed_at.is_none());
    }

    #[test]
    fn list_pending_excludes_resolved() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let id1 = insert(&db, span_for(&c, "a@x.com")).unwrap();
        let id2 = insert(&db, span_for(&c, "b@x.com")).unwrap();
        let id3 = insert(&db, span_for(&c, "c@x.com")).unwrap();
        mark_redacted(&db, id1).unwrap();
        mark_dismissed(&db, id3).unwrap();
        let pending = list_pending_for_capture(&db, &c).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id2);
    }

    #[test]
    fn list_for_capture_returns_all_states() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let id1 = insert(&db, span_for(&c, "a@x.com")).unwrap();
        mark_redacted(&db, id1).unwrap();
        let all = list_for_capture(&db, &c).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].redacted_at.is_some());
    }

    #[test]
    fn category_round_trip_through_db() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        for cat in [
            PiiCategory::Email,
            PiiCategory::Phone,
            PiiCategory::CreditCard,
            PiiCategory::Ssn,
            PiiCategory::Ip,
            PiiCategory::ApiKey,
        ] {
            let mut s = span_for(&c, "x");
            s.category = cat;
            let id = insert(&db, s).unwrap();
            let r = get(&db, id).unwrap().unwrap();
            assert_eq!(r.category, cat);
        }
    }

    #[test]
    fn cascading_delete_via_captures_fk() {
        let (_t, db) = fresh_db();
        let c = cap(&db);
        let _ = insert(&db, span_for(&c, "a@x.com")).unwrap();
        let _ = insert(&db, span_for(&c, "b@x.com")).unwrap();
        // `captures::delete` doesn't exist; per plan fallback we drop via raw SQL.
        // ON DELETE CASCADE on pii_spans.capture_id should sweep the rows.
        db.with_conn(|conn| {
            conn.execute("DELETE FROM captures WHERE id = ?1", [&c])?;
            Ok(())
        })
        .unwrap();
        let remaining = list_for_capture(&db, &c).unwrap();
        assert!(
            remaining.is_empty(),
            "cascade delete left rows: {remaining:?}"
        );
    }
}
