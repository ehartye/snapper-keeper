use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, SchemaVersion, M};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");
const V003: &str = include_str!("../migrations/V003__ocr_fts.sql");
const V004: &str = include_str!("../migrations/V004__annotation_state.sql");
const V005: &str = include_str!("../migrations/V005__drop_clipboard_sensitive.sql");
const V006: &str = include_str!("../migrations/V006__phase10_ocr_bounds_and_pii.sql");

/// All migration SQL strings in order. The index position (1-based) is the schema version.
const MIGRATION_STRS: &[&str] = &[V001, V002, V003, V004, V005, V006];

pub fn migrations() -> Migrations<'static> {
    Migrations::new(MIGRATION_STRS.iter().map(|s| M::up(s)).collect())
}

fn schema_version_as_u32(v: &SchemaVersion) -> u32 {
    usize::from(v) as u32
}

fn backups_dir(db_path: &Path) -> Option<PathBuf> {
    db_path.parent().map(|p| p.join("backups"))
}

/// Checkpoint the WAL into the main DB file, then copy it to `backups/`.
/// Returns the path of the created backup file.
fn create_backup(conn: &Connection, db_path: &Path, prior_version: u32) -> Result<PathBuf> {
    let dir = backups_dir(db_path).ok_or_else(|| crate::LibraryError::Io {
        path: db_path.display().to_string(),
        reason: "cannot determine parent directory for backups".into(),
    })?;
    std::fs::create_dir_all(&dir).map_err(|e| crate::LibraryError::io(&dir, e))?;

    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;

    let ts = Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup_path = dir.join(format!("pre-v{prior_version}-{ts}.db"));
    std::fs::copy(db_path, &backup_path).map_err(|e| crate::LibraryError::io(db_path, e))?;

    Ok(backup_path)
}

/// Keep only the `MAX_BACKUPS` most-recent `.db` files in `backups/`, pruning the rest.
fn prune_backups(backups_dir: &Path) {
    const MAX_BACKUPS: usize = 5;

    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(backups_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "db"))
        .filter_map(|e| {
            let mt = e.metadata().ok()?.modified().ok()?;
            Some((mt, e.path()))
        })
        .collect();

    // Sort newest-first; drop anything beyond the limit.
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));
    for (_, path) in entries.into_iter().skip(MAX_BACKUPS) {
        let _ = std::fs::remove_file(path);
    }
}

/// Find the most-recently modified `.db` file in `backups/`, if any.
pub(crate) fn latest_backup_path(db_path: &Path) -> Option<PathBuf> {
    let dir = backups_dir(db_path)?;
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "db"))
        .filter_map(|e| {
            let mt = e.metadata().ok()?.modified().ok()?;
            Some((mt, e.path()))
        })
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));
    entries.into_iter().next().map(|(_, p)| p)
}

/// Run all pending migrations.
///
/// When `db_path` is `Some`:
/// - Checkpoints the WAL and creates a timestamped backup in `backups/` before
///   applying any pending migrations.
/// - On success, prunes the backup directory to the 5 most-recent files.
/// - On failure, restores the backup and returns a recoverable error.
///
/// When `db_path` is `None` (e.g. in-memory or test connections), the backup
/// step is skipped.
pub fn migrate(conn: &mut Connection, db_path: Option<&Path>) -> Result<()> {
    let prior = migrations()
        .current_version(conn)
        .unwrap_or(SchemaVersion::NoneSet);
    let prior_v = schema_version_as_u32(&prior);
    let target_v = MIGRATION_STRS.len() as u32;

    // Nothing to do if already at the latest version.
    let needs_migration = prior_v < target_v;

    let backup_path: Option<PathBuf> = if needs_migration {
        if let Some(path) = db_path {
            if path.exists() {
                match create_backup(conn, path, prior_v) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        tracing::warn!(err = %e, "pre-migration backup failed; proceeding without backup");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let result = migrations()
        .to_latest(conn)
        .map_err(|_| crate::LibraryError::Migration {
            from: prior_v,
            to: target_v,
            recoverable: false,
        });

    match result {
        Ok(()) => {
            if let Some(ref backup) = backup_path {
                if let Some(dir) = backup.parent() {
                    prune_backups(dir);
                }
            }
            Ok(())
        }
        Err(_) => {
            // Attempt to restore the backup so the user's data is not lost.
            let recoverable = if let (Some(path), Some(ref backup)) = (db_path, &backup_path) {
                std::fs::copy(backup, path).is_ok()
            } else {
                false
            };
            Err(crate::LibraryError::Migration {
                from: prior_v,
                to: target_v,
                recoverable,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v001_applies_cleanly() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, None).expect("apply v001");

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
        migrate(&mut conn, None).unwrap();
        migrate(&mut conn, None).unwrap();
    }

    #[test]
    fn v002_creates_clipboard_items_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, None).expect("apply migrations");

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
        migrate(&mut conn, None).expect("migrations apply");

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
        migrate(&mut conn, None).expect("apply migrations");

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
        migrate(&mut conn, None).expect("migrations apply");

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
        migrate(&mut conn, None).expect("apply migrations");

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
        migrate(&mut conn, None).expect("apply migrations");

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
        migrate(&mut conn, None).expect("apply V005 on top");

        let surviving: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(surviving, 2);
    }
}
