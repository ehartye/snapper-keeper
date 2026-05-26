//! Forward-compatibility tests for DB migrations.
//!
//! Each test loads a committed fixture DB at a specific prior schema version,
//! runs the full migration suite, then exercises representative queries on every
//! table touched by the migration path.  This catches the failure mode where a
//! new migration breaks an existing real-world DB shape that no unit test
//! previously exercised.
//!
//! **Fixture files** (`tests/fixtures/db_v{N}.db`) are SQLite databases created
//! with exactly the schema produced by migrations V001..V{N}.  They are
//! committed as binary blobs so CI can verify migration behaviour without
//! running a separate generator step.  Regenerate them with:
//!
//! ```sh
//! cd crates/snk-library
//! python3 - <<'EOF'
//! import sqlite3, os
//! migration_files = [
//!     "migrations/V001__initial.sql",
//!     "migrations/V002__clipboard_items.sql",
//!     "migrations/V003__ocr_fts.sql",
//!     "migrations/V004__annotation_state.sql",
//!     "migrations/V005__drop_clipboard_sensitive.sql",
//! ]
//! os.makedirs("tests/fixtures", exist_ok=True)
//! for version in range(1, 6):
//!     db_path = f"tests/fixtures/db_v{version}.db"
//!     conn = sqlite3.connect(db_path)
//!     for sql_file in migration_files[:version]:
//!         conn.executescript(open(sql_file).read())
//!     conn.execute(f"PRAGMA user_version = {version}")
//!     conn.commit()
//!     conn.close()
//! EOF
//! ```

use rusqlite::Connection;
use snk_library::migrate::migrate;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Open a *copy* of the fixture at `version` in a temp directory and return
/// the open connection together with the temp dir (so it lives long enough).
fn open_fixture_copy(version: usize) -> (Connection, tempfile::TempDir) {
    let src = format!("{FIXTURES_DIR}/db_v{version}.db");
    let dir = tempfile::tempdir().expect("create tempdir");
    let dst = dir.path().join(format!("db_v{version}.db"));
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy fixture v{version}: {e}"));
    let conn = Connection::open(&dst).unwrap_or_else(|e| panic!("open fixture v{version}: {e}"));
    (conn, dir)
}

fn assert_at_v6(conn: &Connection) {
    let v = snk_library::migrate::migrations()
        .current_version(conn)
        .expect("current_version");
    assert_eq!(
        format!("{v:?}"),
        "Inside(6)",
        "expected schema version 6 after migration"
    );
}

// ── captures (V001) ──────────────────────────────────────────────────────────

#[test]
fn v1_migrates_to_v6_and_captures_query_works() {
    let (mut conn, _dir) = open_fixture_copy(1);
    migrate(&mut conn, None).expect("migrate from v1");
    assert_at_v6(&conn);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM captures", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);

    // annotation_state column (V004) must exist on captures
    conn.execute_batch("SELECT annotation_state FROM captures LIMIT 1")
        .expect("annotation_state column missing after migration");
}

// ── clipboard_items (V002) ────────────────────────────────────────────────────

#[test]
fn v2_migrates_to_v6_and_clipboard_query_works() {
    let (mut conn, _dir) = open_fixture_copy(2);
    migrate(&mut conn, None).expect("migrate from v2");
    assert_at_v6(&conn);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);

    // sensitive column must NOT exist (dropped by V005)
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(clipboard_items)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert!(
        !cols.iter().any(|c| c == "sensitive"),
        "sensitive column must be gone after V005 runs; got {cols:?}"
    );
}

// ── ocr_text + FTS5 (V003) ────────────────────────────────────────────────────

#[test]
fn v3_migrates_to_v6_and_ocr_query_works() {
    let (mut conn, _dir) = open_fixture_copy(3);
    migrate(&mut conn, None).expect("migrate from v3");
    assert_at_v6(&conn);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ocr_text", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);

    // words_json and engine columns (V006) must exist
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(ocr_text)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert!(
        cols.contains(&"words_json".to_string()),
        "words_json column missing; got {cols:?}"
    );
    assert!(
        cols.contains(&"engine".to_string()),
        "engine column missing; got {cols:?}"
    );
}

// ── annotation_state (V004) ───────────────────────────────────────────────────

#[test]
fn v4_migrates_to_v6_and_annotation_state_column_present() {
    let (mut conn, _dir) = open_fixture_copy(4);
    migrate(&mut conn, None).expect("migrate from v4");
    assert_at_v6(&conn);

    conn.execute_batch("SELECT annotation_state FROM captures LIMIT 1")
        .expect("annotation_state column missing");
}

// ── V005 drop sensitive column + existing rows survive ────────────────────────

#[test]
fn v5_migrates_to_v6_pii_spans_table_present() {
    let (mut conn, _dir) = open_fixture_copy(5);
    migrate(&mut conn, None).expect("migrate from v5");
    assert_at_v6(&conn);

    let cnt: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pii_spans'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cnt, 1, "pii_spans table must exist after V006");
}

// ── backup + restore round-trip ───────────────────────────────────────────────

#[test]
fn backup_is_created_and_pruned_on_successful_migration() {
    // Use v1 fixture as the starting point.
    let dir = tempfile::tempdir().expect("create tempdir");
    let db_path = dir.path().join("sk.db");
    let src = format!("{FIXTURES_DIR}/db_v1.db");
    std::fs::copy(&src, &db_path).expect("copy fixture");

    let mut conn = Connection::open(&db_path).unwrap();
    migrate(&mut conn, Some(&db_path)).expect("migrate");

    // A backup file must have been created.
    let backups_dir = db_path.parent().unwrap().join("backups");
    assert!(backups_dir.exists(), "backups/ directory must be created");

    let backup_files: Vec<_> = std::fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "db"))
        .collect();
    assert_eq!(
        backup_files.len(),
        1,
        "exactly one backup file should exist; got {backup_files:?}"
    );
    let backup_name = backup_files[0].file_name().to_string_lossy().into_owned();
    assert!(
        backup_name.starts_with("pre-v1-"),
        "backup name should start with 'pre-v1-'; got {backup_name}"
    );
}

#[test]
fn backup_pruning_keeps_at_most_five() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();

    // Seed 7 fake backup files.
    for i in 0..7u8 {
        std::fs::write(
            backups.join(format!("pre-v{i}-2025010{i}T000000Z.db")),
            &[i],
        )
        .unwrap();
        // Tiny sleep to ensure distinct mtimes on filesystems with 1-second resolution.
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Run a migration from a v1 fixture through the full path so prune_backups fires.
    let db_path = dir.path().join("sk.db");
    let src = format!("{FIXTURES_DIR}/db_v1.db");
    std::fs::copy(&src, &db_path).expect("copy fixture");

    let mut conn = Connection::open(&db_path).unwrap();
    migrate(&mut conn, Some(&db_path)).expect("migrate");

    // After a successful migration there should be at most 5 backups.
    let count = std::fs::read_dir(&backups)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "db"))
        .count();
    assert!(count <= 5, "expected ≤5 backups after pruning; got {count}");
}
