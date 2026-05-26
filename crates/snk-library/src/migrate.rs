use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");
const V003: &str = include_str!("../migrations/V003__ocr_fts.sql");
const V004: &str = include_str!("../migrations/V004__annotation_state.sql");
const V005: &str = include_str!("../migrations/V005__drop_clipboard_sensitive.sql");
const V006: &str = include_str!("../migrations/V006__phase10_ocr_bounds_and_pii.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(V001),
        M::up(V002),
        M::up(V003),
        M::up(V004),
        M::up(V005),
        M::up(V006),
    ])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 6,
            recoverable: e.to_string().contains("Backup"),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v001_applies_cleanly() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("apply v001");

        // Tables exist
        for table in [
            "captures",
            "tags",
            "capture_tags",
            "settings",
            "hotkey_bindings",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {table} should exist");
        }
    }

    #[test]
    fn v001_is_idempotent_via_migrations_tracking() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();
    }

    #[test]
    fn v002_creates_clipboard_items_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("apply migrations");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='clipboard_items'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "clipboard_items table should exist");
    }

    #[test]
    fn migration_count_matches_latest_applied_version() {
        // Catches the failure mode where someone adds a new `Vxxx__*.sql`
        // file without also adding a matching `M::up` entry in
        // `migrations()` (or vice versa). The count of .sql files in the
        // migrations/ directory should equal the schema version reported
        // after `to_latest()`.
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("migrations apply");

        let v = migrations()
            .current_version(&conn)
            .expect("query schema version");

        let migration_files = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
            .expect("read migrations dir")
            .filter_map(std::result::Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .count();

        assert_eq!(
            format!("{v:?}"),
            format!("Inside({migration_files})"),
            "applied schema version doesn't match the count of .sql files in migrations/. \
             If you added a file, add a matching `M::up` entry in migrations(); \
             if you removed one, drop the corresponding entry."
        );
    }

    #[test]
    fn v003_creates_ocr_and_fts_tables() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("apply migrations");

        for table in ["ocr_text", "captures_fts", "clipboard_fts"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view') AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {table} should exist");
        }
    }

    #[test]
    fn v005_drops_sensitive_column_from_clipboard_items() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("migrations apply");

        let column_names: Vec<String> = conn
            .prepare("PRAGMA table_info(clipboard_items)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();

        assert!(
            !column_names.iter().any(|c| c == "sensitive"),
            "sensitive column should be dropped by V005; got columns {column_names:?}"
        );
    }

    #[test]
    fn v006_adds_words_json_and_engine_to_ocr_text() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("apply migrations");

        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(ocr_text)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(
            cols.contains(&"words_json".into()),
            "words_json column missing; got {cols:?}"
        );
        assert!(
            cols.contains(&"engine".into()),
            "engine column missing; got {cols:?}"
        );
    }

    #[test]
    fn v006_creates_pii_spans_table_with_indexes() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).expect("apply migrations");

        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pii_spans'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);

        let idx_full: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_pii_spans_capture'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_full, 1);

        let idx_pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_pii_spans_pending'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_pending, 1);
    }

    #[test]
    fn v004_to_v005_preserves_clipboard_rows() {
        use rusqlite::params;

        let mut conn = Connection::open_in_memory().unwrap();
        // Apply through V004 only.
        let v1_to_v4 = Migrations::new(vec![M::up(V001), M::up(V002), M::up(V003), M::up(V004)]);
        v1_to_v4.to_latest(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO clipboard_items
                (id, kind, text_content, content_hash, created_at, pinned, sensitive)
             VALUES
                (?1, 'text', 'hello', 'abc', 1, 0, 0),
                (?2, 'text', 'secret', 'def', 2, 0, 1)",
            params!["row-a", "row-b"],
        )
        .unwrap();

        // Apply V005 by running the full migration set.
        migrate(&mut conn).expect("apply V005 on top");

        let surviving: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(surviving, 2);
    }
}
