use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use tracing::info;

use crate::Result;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::LibraryError::io(parent, e))?;
        }
        let mut conn = Connection::open(path)?;
        // WAL gives us concurrent readers + a writer without DB-level locks fighting us.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::migrate::migrate(&mut conn)?;
        info!(path = %path.display(), "opened db");
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub(crate) fn with_conn<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut guard = self.conn.lock().expect("db mutex poisoned");
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
