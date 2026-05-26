use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use tracing::info;

use crate::Result;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    // VERIFIED: privacy-md/local-only-storage
    // All persistence is to a local SQLite file. No remote DB, no network
    // synchronization. This function takes a local path and opens it
    // in-process; there is no code path that uploads, syncs, or shares
    // the resulting database.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::LibraryError::io(parent, e))?;
        }
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        match crate::migrate::migrate(&mut conn, Some(path)) {
            Ok(()) => {
                info!(path = %path.display(), "opened db");
                Ok(Self { conn: Mutex::new(conn) })
            }
            Err(crate::LibraryError::Migration {
                backup_path: Some(backup),
                from,
                to,
                detail,
                ..
            }) => Err(restore_after_migration_failure(conn, path, backup, from, to, detail)),
            Err(e) => Err(e),
        }
    }

    /// Open the DB without running migrations.  Used after a migration failure
    /// when a backup has already been restored; the DB is at its pre-migration
    /// schema and should be readable with the old layout.
    pub(crate) fn open_no_migrate(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::LibraryError::io(parent, e))?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        info!(path = %path.display(), "opened db (no migrate — running at prior schema version)");
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open with a custom Migrations set.  Used by tests to inject deliberate failures.
    #[cfg(test)]
    pub(crate) fn open_with_custom_migrations(
        path: &Path,
        m: &rusqlite_migration::Migrations<'static>,
    ) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::LibraryError::io(parent, e))?;
        }
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        match crate::migrate::migrate_with(&mut conn, Some(path), m) {
            Ok(()) => Ok(Self { conn: Mutex::new(conn) }),
            Err(crate::LibraryError::Migration {
                backup_path: Some(backup),
                from,
                to,
                detail,
                ..
            }) => Err(restore_after_migration_failure(conn, path, backup, from, to, detail)),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn with_conn<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut guard = self.conn.lock().expect("db mutex poisoned");
        f(&mut guard)
    }
}

/// Drop the connection, remove WAL/SHM files, then restore the backup over the DB file.
/// Returns the `LibraryError::Migration` with `recoverable` set to whether the copy succeeded.
/// Dropping `conn` before the `fs::copy` avoids Windows file-locking failures, and removing
/// WAL/SHM prevents the failed migration's uncommitted pages from being replayed onto the
/// restored database.
fn restore_after_migration_failure(
    conn: Connection,
    path: &Path,
    backup: String,
    from: u32,
    to: u32,
    detail: String,
) -> crate::LibraryError {
    drop(conn);
    let db_str = path.to_string_lossy();
    let _ = std::fs::remove_file(format!("{db_str}-wal"));
    let _ = std::fs::remove_file(format!("{db_str}-shm"));
    let recoverable = std::fs::copy(&backup, path).is_ok();
    crate::LibraryError::Migration {
        from,
        to,
        recoverable,
        backup_path: Some(backup),
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite_migration::{Migrations, M};

    #[test]
    fn open_creates_parent_dir_and_wal_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/sk.db");
        let db = Db::open(&path).expect("open");
        db.with_conn(|c| {
            let mode: String = c
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .unwrap();
            assert_eq!(mode.to_lowercase(), "wal");
            Ok(())
        })
        .unwrap();
        assert!(path.exists());
    }

    #[test]
    fn migration_failure_restores_db_and_returns_recoverable_true() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Build a minimal v1 DB: one table, user_version = 1.
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
            conn.pragma_update(None, "user_version", 1u32).unwrap();
        }
        let original_bytes = std::fs::read(&db_path).unwrap();

        // Migrations: step 1 already applied (user_version=1), step 2 is broken.
        let broken = Migrations::new(vec![
            M::up("CREATE TABLE t (id INTEGER PRIMARY KEY)"),
            M::up("THIS IS NOT VALID SQL"),
        ]);

        let err = Db::open_with_custom_migrations(&db_path, &broken)
            .expect_err("broken migration must fail");

        match err {
            crate::LibraryError::Migration { recoverable, backup_path, .. } => {
                assert!(recoverable, "restore should succeed; got recoverable=false");
                assert!(backup_path.is_some(), "backup_path must be present in error");
            }
            other => panic!("expected Migration error, got {other:?}"),
        }

        // The DB file must be restored to the exact pre-migration bytes.
        let restored_bytes = std::fs::read(&db_path).unwrap();
        assert_eq!(
            original_bytes, restored_bytes,
            "DB must be byte-identical to backup after restore"
        );
    }
}
