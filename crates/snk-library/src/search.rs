use serde::{Deserialize, Serialize};

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SearchResult {
    Capture {
        id: String,
        rank: f64,
        snippet: String,
    },
    Clipboard {
        id: String,
        rank: f64,
        snippet: String,
    },
}

pub fn index_capture(
    db: &Db,
    capture_id: &str,
    source_app: Option<&str>,
    window_title: Option<&str>,
    ocr_text: Option<&str>,
    tag_names: Option<&str>,
) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM captures_fts WHERE capture_id = ?1",
            [capture_id],
        )?;
        conn.execute(
            "INSERT INTO captures_fts (capture_id, source_app, window_title, ocr_text, tag_names)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                capture_id,
                source_app.unwrap_or(""),
                window_title.unwrap_or(""),
                ocr_text.unwrap_or(""),
                tag_names.unwrap_or(""),
            ],
        )?;
        Ok(())
    })
}

pub fn index_clipboard(
    db: &Db,
    clipboard_id: &str,
    text_content: Option<&str>,
    source_app: Option<&str>,
    window_title: Option<&str>,
) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM clipboard_fts WHERE clipboard_id = ?1",
            [clipboard_id],
        )?;
        conn.execute(
            "INSERT INTO clipboard_fts (clipboard_id, text_content, source_app, window_title)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                clipboard_id,
                text_content.unwrap_or(""),
                source_app.unwrap_or(""),
                window_title.unwrap_or(""),
            ],
        )?;
        Ok(())
    })
}

pub fn search(db: &Db, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = sanitize_fts_query(query);
    if fts_query.is_empty() {
        return Ok(Vec::new());
    }
    db.with_conn(|conn| {
        let mut results = Vec::new();

        let mut stmt = conn.prepare(
            "SELECT capture_id, rank, snippet(captures_fts, 3, '<mark>', '</mark>', '...', 32)
             FROM captures_fts
             WHERE captures_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let capture_rows = stmt.query_map(rusqlite::params![&fts_query, limit], |row| {
            Ok(SearchResult::Capture {
                id: row.get(0)?,
                rank: row.get(1)?,
                snippet: row.get(2)?,
            })
        })?;
        for row in capture_rows {
            results.push(row?);
        }

        let mut stmt = conn.prepare(
            "SELECT clipboard_id, rank, snippet(clipboard_fts, 1, '<mark>', '</mark>', '...', 32)
             FROM clipboard_fts
             WHERE clipboard_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let clipboard_rows = stmt.query_map(rusqlite::params![&fts_query, limit], |row| {
            Ok(SearchResult::Clipboard {
                id: row.get(0)?,
                rank: row.get(1)?,
                snippet: row.get(2)?,
            })
        })?;
        for row in clipboard_rows {
            results.push(row?);
        }

        // FTS5 rank is negative — lower (more negative) = better match.
        results.sort_by(|a, b| {
            let ra = match a {
                SearchResult::Capture { rank, .. } | SearchResult::Clipboard { rank, .. } => *rank,
            };
            let rb = match b {
                SearchResult::Capture { rank, .. } | SearchResult::Clipboard { rank, .. } => *rank,
            };
            ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit as usize);
        Ok(results)
    })
}

pub fn remove_capture_index(db: &Db, capture_id: &str) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM captures_fts WHERE capture_id = ?1",
            [capture_id],
        )?;
        Ok(())
    })
}

fn sanitize_fts_query(input: &str) -> String {
    let tokens: Vec<String> = input
        .split_whitespace()
        .map(|token| {
            let cleaned: String = token
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if cleaned.is_empty() {
                String::new()
            } else {
                format!("\"{cleaned}\"*")
            }
        })
        .filter(|s| !s.is_empty())
        .collect();
    tokens.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::test_support::fresh_db;

    fn insert_capture(db: &Db, app: &str, title: &str) -> String {
        crate::captures::insert(
            db,
            crate::NewCapture {
                file_path: PathBuf::from("test.png"),
                width: 100,
                height: 100,
                source_app: Some(app.into()),
                source_window_title: Some(title.into()),
                monitor: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn index_and_search_capture_by_ocr_text() {
        let (_tmp, db) = fresh_db();
        let id = insert_capture(&db, "Firefox", "GitHub");
        index_capture(
            &db,
            &id,
            Some("Firefox"),
            Some("GitHub"),
            Some("hello world rust"),
            None,
        )
        .unwrap();

        let results = search(&db, "rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::Capture { id: rid, .. } => assert_eq!(rid, &id),
            _ => panic!("expected Capture result"),
        }
    }

    #[test]
    fn index_and_search_clipboard_by_text_content() {
        let (_tmp, db) = fresh_db();
        let item = crate::clipboard::insert(
            &db,
            crate::NewClipboardItem {
                kind: crate::ClipboardItemKind::Text,
                text_content: Some("important meeting notes".into()),
                file_path: None,
                content_hash: "abc".into(),
                source_app: Some("Slack".into()),
                source_window_title: Some("General".into()),
            },
        )
        .unwrap();
        index_clipboard(
            &db,
            &item.id,
            Some("important meeting notes"),
            Some("Slack"),
            Some("General"),
        )
        .unwrap();

        let results = search(&db, "meeting", 10).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::Clipboard { id: rid, .. } => assert_eq!(rid, &item.id),
            _ => panic!("expected Clipboard result"),
        }
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let (_tmp, db) = fresh_db();
        let results = search(&db, "nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_mixed_results_ranked() {
        let (_tmp, db) = fresh_db();
        let cap_id = insert_capture(&db, "VS Code", "main.rs");
        index_capture(
            &db,
            &cap_id,
            Some("VS Code"),
            Some("main.rs"),
            Some("fn main rust"),
            None,
        )
        .unwrap();

        let clip = crate::clipboard::insert(
            &db,
            crate::NewClipboardItem {
                kind: crate::ClipboardItemKind::Text,
                text_content: Some("rust programming language".into()),
                file_path: None,
                content_hash: "xyz".into(),
                source_app: Some("Firefox".into()),
                source_window_title: Some("docs.rs".into()),
            },
        )
        .unwrap();
        index_clipboard(
            &db,
            &clip.id,
            Some("rust programming language"),
            Some("Firefox"),
            Some("docs.rs"),
        )
        .unwrap();

        let results = search(&db, "rust", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn update_capture_index_replaces_old_entry() {
        let (_tmp, db) = fresh_db();
        let id = insert_capture(&db, "App", "Win");
        index_capture(&db, &id, Some("App"), Some("Win"), Some("old text"), None).unwrap();
        index_capture(&db, &id, Some("App"), Some("Win"), Some("new text"), None).unwrap();

        let results = search(&db, "old", 10).unwrap();
        assert!(results.is_empty());
        let results = search(&db, "new", 10).unwrap();
        assert_eq!(results.len(), 1);
    }
}
