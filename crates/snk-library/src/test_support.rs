//! Test-only helpers shared across `snk-library` tests.
//!
//! Compiled only under `#[cfg(test)]` and `pub(crate)` so it's reachable
//! from any module's `mod tests` without leaking into downstream crates.

use tempfile::TempDir;

use crate::Db;

/// Open a fresh on-disk SQLite library for a single test.
///
/// Returns the `TempDir` alongside the `Db` so the caller can bind both
/// to local variables and let `Drop` clean the directory at end of test.
///
/// ```ignore
/// let (_tmp, db) = test_support::fresh_db();
/// // _tmp lives until the end of the test, then the dir is removed.
/// ```
///
/// Replaces the prior `let dir = tempfile::tempdir().unwrap();
/// std::mem::forget(dir);` workaround that leaked one directory per
/// test (cumulative bloat over CI runs).
pub(crate) fn fresh_db() -> (TempDir, Db) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("sk.db");
    let db = Db::open(&path).expect("open sk.db");
    (dir, db)
}
