use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");
const V003: &str = include_str!("../migrations/V003__ocr_fts.sql");
const V004: &str = include_str!("../migrations/V004__annotation_state.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V001), M::up(V002), M::up(V003), M::up(V004)])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 4,
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
}
