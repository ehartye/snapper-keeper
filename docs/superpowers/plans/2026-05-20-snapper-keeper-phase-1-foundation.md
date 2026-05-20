# snapper-keeper — Phase 1: Foundation & Vertical Slice

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Get a working end-to-end vertical slice — press `Ctrl/Cmd+Shift+3`, capture the primary monitor, persist to disk + DB, see the thumbnail appear in the library window — on top of a properly scaffolded Tauri 2 + Rust workspace.

**Architecture:** Cargo workspace with one crate per Tauri plugin (`snk-library`, `snk-hotkeys`, `snk-capture` in phase 1). pnpm workspace with one TS package per plugin (typed bindings) and an `app/` shell consuming them. All persistence flows through `snk-library`. See `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md` for the full design.

**Tech Stack:** Tauri 2, Rust (rusqlite + rusqlite_migration + xcap), React 18 + TypeScript + Vite, Tailwind, Zustand, TanStack Query, vitest (frontend), `cargo test` (Rust), pnpm 9, GitHub Actions.

**Phase 1 scope (in):**
- Cargo + pnpm workspace, tooling, and CI baseline (lint + build smoke + unit tests on all three OSes)
- `snk-library` crate: SQLite + migrations + the captures/tags/settings tables, query/mutation API
- `snk-hotkeys` crate: register hotkeys from settings, emit events on trigger
- `snk-capture` crate: full-screen capture only (no region overlay yet), file write + library insert
- `app/` shell: Tauri config, tray icon, library window with a minimal capture grid
- Press hotkey → capture → see in grid (manual smoke test)

**Out of scope for phase 1 (later phases):**
- Region-select overlay, window capture, timed capture, OCR
- Annotation editor, clipboard plugin, clipboard popup
- FTS5 search, tags UI, settings UI
- Signed/notarized installers, auto-updater, release pipeline
- First-run wizard, full library window with sidebar

---

## Pre-flight

You are starting in a worktree (created by the executing-plans skill) with `main` already containing:

- `.gitignore` (with `.superpowers/` already excluded)
- `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md`

You will be on a fresh feature branch. All commits land on that branch.

**Required tools installed before you start:**

- Rust toolchain (`rustup` stable, currently 1.78+)
- Node.js 20+ and pnpm 9+ (`npm install -g pnpm@9`)
- Platform prereqs from <https://tauri.app/start/prerequisites/>:
  - **Windows:** Microsoft Visual Studio C++ Build Tools, WebView2 (usually present), `cargo install create-tauri-app`
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)

Verify before starting:

```bash
rustc --version    # 1.78+
node --version     # 20+
pnpm --version     # 9+
```

---

## Task 1: Initialize Cargo workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `rust-toolchain.toml`
- Create: `.rustfmt.toml`
- Create: `.cargo/config.toml`

**Step 1: Write `Cargo.toml` at repo root**

```toml
[workspace]
resolver = "2"
members = [
    "crates/snk-library",
    "crates/snk-hotkeys",
    "crates/snk-capture",
    "app/src-tauri",
]

[workspace.package]
edition = "2021"
rust-version = "1.78"
license = "MIT OR Apache-2.0"
publish = false

[workspace.dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2" }
tauri-plugin-global-shortcut = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tokio = { version = "1", features = ["sync", "rt-multi-thread", "macros"] }
rusqlite = { version = "0.31", features = ["bundled", "uuid"] }
rusqlite_migration = "1"
uuid = { version = "1", features = ["v7", "serde"] }
xcap = "0.0.13"
image = { version = "0.25", default-features = false, features = ["png"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
```

**Step 2: Write `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

**Step 3: Write `.rustfmt.toml`**

```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

**Step 4: Write `.cargo/config.toml`**

```toml
[build]
# Build artifacts go into a single shared target dir
target-dir = "target"

[net]
git-fetch-with-cli = true
```

**Step 5: Sanity check**

Run: `cargo --version`
Expected: prints cargo version. No `cargo build` yet — there are no crate members on disk.

**Step 6: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .rustfmt.toml .cargo/config.toml
git commit -m "chore: initialize Cargo workspace"
```

---

## Task 2: Initialize pnpm workspace and root tooling

**Files:**
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `.npmrc`
- Create: `tsconfig.base.json`
- Create: `.editorconfig`
- Create: `.prettierrc.json`
- Create: `.eslintrc.cjs`

**Step 1: Write `package.json`**

```json
{
  "name": "snapper-keeper",
  "version": "0.0.0",
  "private": true,
  "packageManager": "pnpm@9.0.0",
  "scripts": {
    "lint": "pnpm -r run lint",
    "typecheck": "pnpm -r run typecheck",
    "test": "pnpm -r run test",
    "build": "pnpm -r run build",
    "tauri": "pnpm --filter @snk/app tauri"
  },
  "devDependencies": {
    "@types/node": "^20.11.0",
    "eslint": "^9.0.0",
    "@typescript-eslint/eslint-plugin": "^7.0.0",
    "@typescript-eslint/parser": "^7.0.0",
    "eslint-plugin-react": "^7.34.0",
    "eslint-plugin-react-hooks": "^4.6.0",
    "prettier": "^3.2.0",
    "typescript": "^5.4.0"
  }
}
```

**Step 2: Write `pnpm-workspace.yaml`**

```yaml
packages:
  - "app"
  - "packages/*"
```

**Step 3: Write `.npmrc`**

```
strict-peer-dependencies=false
auto-install-peers=true
```

**Step 4: Write `tsconfig.base.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "jsx": "react-jsx",
    "lib": ["ES2022", "DOM", "DOM.Iterable"]
  }
}
```

**Step 5: Write `.editorconfig`**

```
root = true

[*]
indent_style = space
indent_size = 2
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true

[*.{rs,toml}]
indent_size = 4
```

**Step 6: Write `.prettierrc.json`**

```json
{
  "semi": true,
  "singleQuote": true,
  "trailingComma": "all",
  "printWidth": 100,
  "arrowParens": "always"
}
```

**Step 7: Write `.eslintrc.cjs`**

```js
module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: { ecmaVersion: 2022, sourceType: 'module', ecmaFeatures: { jsx: true } },
  plugins: ['@typescript-eslint', 'react', 'react-hooks'],
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:react/recommended',
    'plugin:react-hooks/recommended',
  ],
  settings: { react: { version: 'detect' } },
  ignorePatterns: ['dist', 'node_modules', 'target', '.turbo', 'src-tauri'],
  rules: {
    'react/react-in-jsx-scope': 'off',
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
  },
};
```

**Step 8: Install root dev deps**

Run: `pnpm install`
Expected: pnpm creates `node_modules/` and `pnpm-lock.yaml`. No package warnings of consequence.

**Step 9: Commit**

```bash
git add package.json pnpm-workspace.yaml .npmrc tsconfig.base.json .editorconfig .prettierrc.json .eslintrc.cjs pnpm-lock.yaml
git commit -m "chore: initialize pnpm workspace and root tooling"
```

---

## Task 3: Scaffold the `snk-library` crate

**Files:**
- Create: `crates/snk-library/Cargo.toml`
- Create: `crates/snk-library/src/lib.rs`
- Create: `crates/snk-library/src/error.rs`

**Step 1: Write `crates/snk-library/Cargo.toml`**

```toml
[package]
name = "snk-library"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
tauri.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
rusqlite.workspace = true
rusqlite_migration.workspace = true
uuid.workspace = true
chrono.workspace = true
tokio.workspace = true
anyhow.workspace = true

[dev-dependencies]
tempfile = "3"
```

**Step 2: Write `crates/snk-library/src/error.rs`**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LibraryError {
    #[error("database error: {message}")]
    Database { message: String, retryable: bool },

    #[error("io error at {path}: {kind}")]
    Io { path: String, kind: String },

    #[error("migration failed from {from} to {to}")]
    Migration { from: u32, to: u32, recoverable: bool },

    #[error("not found: {what}")]
    NotFound { what: String },
}

impl From<rusqlite::Error> for LibraryError {
    fn from(e: rusqlite::Error) -> Self {
        let retryable = matches!(
            e,
            rusqlite::Error::SqliteFailure(ref err, _)
                if err.code == rusqlite::ErrorCode::DatabaseBusy
                || err.code == rusqlite::ErrorCode::DatabaseLocked
        );
        LibraryError::Database { message: e.to_string(), retryable }
    }
}

impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        LibraryError::Io {
            path: String::new(),
            kind: e.kind().to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, LibraryError>;
```

**Step 3: Write `crates/snk-library/src/lib.rs`**

```rust
//! snk-library — single owner of the SQLite persistence layer.

pub mod error;

pub use error::{LibraryError, Result};

// Tauri plugin entry point — wired in Task 9.
```

**Step 4: Verify it compiles**

Run: `cargo build -p snk-library`
Expected: compiles clean. No warnings about unused imports.

**Step 5: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): scaffold crate and error types"
```

---

## Task 4: snk-library — connection management

**Files:**
- Create: `crates/snk-library/src/db.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write the failing test**

Append to `crates/snk-library/src/db.rs`:

```rust
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
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        // WAL gives us concurrent readers + a writer without DB-level locks fighting us.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        info!(path = %path.display(), "opened db");
        Ok(Self { conn: Mutex::new(conn) })
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
            let mode: String =
                c.query_row("PRAGMA journal_mode", [], |row| row.get(0)).unwrap();
            assert_eq!(mode.to_lowercase(), "wal");
            Ok(())
        })
        .unwrap();
        assert!(path.exists());
    }
}
```

**Step 2: Expose the module — modify `crates/snk-library/src/lib.rs`**

Replace contents with:

```rust
//! snk-library — single owner of the SQLite persistence layer.

pub mod db;
pub mod error;

pub use db::Db;
pub use error::{LibraryError, Result};
```

**Step 3: Run the test and verify it passes**

Run: `cargo test -p snk-library`
Expected: 1 test passes (`db::tests::open_creates_parent_dir_and_wal_mode`).

**Step 4: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): add Db connection wrapper with WAL mode"
```

---

## Task 5: snk-library — migrations and V001 schema

**Files:**
- Create: `crates/snk-library/migrations/V001__initial.sql`
- Create: `crates/snk-library/src/migrate.rs`
- Modify: `crates/snk-library/src/db.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write `crates/snk-library/migrations/V001__initial.sql`**

```sql
-- Phase 1 schema — captures, tags, settings.
-- FTS, clipboard, ocr_text added in later migrations.

CREATE TABLE captures (
    id              TEXT PRIMARY KEY,                          -- uuid v7
    file_path       TEXT NOT NULL,                             -- relative · captures/YYYY/MM/uuid.png
    annotated_path  TEXT,
    width           INTEGER NOT NULL,
    height          INTEGER NOT NULL,
    source_app      TEXT,
    source_window_title TEXT,
    monitor         TEXT,
    created_at      INTEGER NOT NULL,                          -- unix ms
    deleted_at      INTEGER,
    pinned          INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_captures_created_at ON captures(created_at DESC);
CREATE INDEX idx_captures_deleted_at ON captures(deleted_at) WHERE deleted_at IS NOT NULL;

CREATE TABLE tags (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    color      TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE capture_tags (
    capture_id TEXT NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
    tag_id     TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (capture_id, tag_id)
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL  -- json
);

CREATE TABLE hotkey_bindings (
    action_id TEXT PRIMARY KEY,
    chord     TEXT NOT NULL
);
```

**Step 2: Write `crates/snk-library/src/migrate.rs`**

```rust
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::Result;

const V001: &str = include_str!("../migrations/V001__initial.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V001)])
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations().to_latest(conn).map_err(|e| crate::LibraryError::Migration {
        from: 0,
        to: 1,
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
        for table in ["captures", "tags", "capture_tags", "settings", "hotkey_bindings"] {
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
}
```

**Step 3: Hook migrations into `Db::open` — modify `crates/snk-library/src/db.rs`**

Find:

```rust
        let conn = Connection::open(path)?;
        // WAL gives us concurrent readers + a writer without DB-level locks fighting us.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        info!(path = %path.display(), "opened db");
        Ok(Self { conn: Mutex::new(conn) })
```

Replace with:

```rust
        let mut conn = Connection::open(path)?;
        // WAL gives us concurrent readers + a writer without DB-level locks fighting us.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::migrate::migrate(&mut conn)?;
        info!(path = %path.display(), "opened db");
        Ok(Self { conn: Mutex::new(conn) })
```

**Step 4: Re-export — modify `crates/snk-library/src/lib.rs`**

```rust
//! snk-library — single owner of the SQLite persistence layer.

pub mod db;
pub mod error;
pub mod migrate;

pub use db::Db;
pub use error::{LibraryError, Result};
```

**Step 5: Run tests**

Run: `cargo test -p snk-library`
Expected: 3 tests pass.

**Step 6: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): add migrations machinery and V001 schema"
```

---

## Task 6: snk-library — Capture model + repo with insert

**Files:**
- Create: `crates/snk-library/src/captures.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write the failing test — create `crates/snk-library/src/captures.rs`**

```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Db, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capture {
    pub id: String,
    pub file_path: String,
    pub annotated_path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
    pub monitor: Option<String>,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCapture {
    pub file_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub source_app: Option<String>,
    pub source_window_title: Option<String>,
    pub monitor: Option<String>,
}

pub fn insert(db: &Db, new: NewCapture) -> Result<Capture> {
    let id = Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().timestamp_millis();
    let file_path = new
        .file_path
        .to_str()
        .ok_or_else(|| crate::LibraryError::Io {
            path: new.file_path.display().to_string(),
            kind: "non-utf8 path".into(),
        })?
        .to_string();

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO captures
                (id, file_path, width, height, source_app, source_window_title, monitor, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &id,
                &file_path,
                new.width,
                new.height,
                new.source_app,
                new.source_window_title,
                new.monitor,
                created_at,
            ],
        )?;
        Ok(())
    })?;

    Ok(Capture {
        id,
        file_path,
        annotated_path: None,
        width: new.width,
        height: new.height,
        source_app: new.source_app,
        source_window_title: new.source_window_title,
        monitor: new.monitor,
        created_at,
        deleted_at: None,
        pinned: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Db {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        std::mem::forget(dir);
        Db::open(&path).unwrap()
    }

    #[test]
    fn insert_returns_capture_with_uuid_v7() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("captures/2026/05/x.png"),
            width: 1920,
            height: 1080,
            source_app: Some("Firefox".into()),
            source_window_title: Some("github".into()),
            monitor: Some("Monitor 0".into()),
        };
        let c = insert(&db, new).unwrap();
        assert_eq!(c.width, 1920);
        assert_eq!(c.height, 1080);
        assert_eq!(c.source_app.as_deref(), Some("Firefox"));
        // UUIDv7 strings are 36 chars
        assert_eq!(c.id.len(), 36);
        assert!(!c.pinned);
        assert!(c.deleted_at.is_none());
    }

    #[test]
    fn insert_creates_a_persisted_row() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("x.png"),
            width: 1, height: 1,
            source_app: None, source_window_title: None, monitor: None,
        };
        let c = insert(&db, new).unwrap();
        db.with_conn(|conn| {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM captures WHERE id = ?1", [&c.id], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    }
}
```

**Step 2: Re-export — modify `crates/snk-library/src/lib.rs`**

```rust
//! snk-library — single owner of the SQLite persistence layer.

pub mod captures;
pub mod db;
pub mod error;
pub mod migrate;

pub use captures::{Capture, NewCapture};
pub use db::Db;
pub use error::{LibraryError, Result};
```

**Step 3: Run tests**

Run: `cargo test -p snk-library`
Expected: 5 tests pass.

**Step 4: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): Capture model and insert"
```

---

## Task 7: snk-library — list and get capture queries

**Files:**
- Modify: `crates/snk-library/src/captures.rs`

**Step 1: Write the failing tests — append to `crates/snk-library/src/captures.rs`**

Find the existing `pub fn insert(...)` block. Immediately after it, before `#[cfg(test)]`, add:

```rust
fn row_to_capture(row: &rusqlite::Row<'_>) -> rusqlite::Result<Capture> {
    Ok(Capture {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        annotated_path: row.get("annotated_path")?,
        width: row.get::<_, i64>("width")? as u32,
        height: row.get::<_, i64>("height")? as u32,
        source_app: row.get("source_app")?,
        source_window_title: row.get("source_window_title")?,
        monitor: row.get("monitor")?,
        created_at: row.get("created_at")?,
        deleted_at: row.get("deleted_at")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
    })
}

pub fn get(db: &Db, id: &str) -> Result<Capture> {
    db.with_conn(|conn| {
        let row = conn
            .query_row(
                "SELECT * FROM captures WHERE id = ?1 AND deleted_at IS NULL",
                [id],
                row_to_capture,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => crate::LibraryError::NotFound {
                    what: format!("capture {id}"),
                },
                other => other.into(),
            })?;
        Ok(row)
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListCapturesQuery {
    pub limit: Option<u32>,
    pub include_deleted: bool,
}

pub fn list(db: &Db, q: ListCapturesQuery) -> Result<Vec<Capture>> {
    let limit = q.limit.unwrap_or(200).min(1000);
    db.with_conn(|conn| {
        let sql = if q.include_deleted {
            "SELECT * FROM captures ORDER BY created_at DESC LIMIT ?1"
        } else {
            "SELECT * FROM captures WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map([limit], row_to_capture)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}
```

Inside the existing `#[cfg(test)] mod tests { ... }`, append these tests before the closing `}`:

```rust
    #[test]
    fn get_returns_inserted() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("a.png"),
            width: 10, height: 10,
            source_app: None, source_window_title: None, monitor: None,
        };
        let inserted = insert(&db, new).unwrap();
        let fetched = get(&db, &inserted.id).unwrap();
        assert_eq!(fetched, inserted);
    }

    #[test]
    fn get_returns_not_found_for_missing_id() {
        let db = fresh_db();
        match get(&db, "no-such-id") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn list_returns_newest_first_and_excludes_deleted() {
        let db = fresh_db();
        let mk = |i: u32| NewCapture {
            file_path: PathBuf::from(format!("{i}.png")),
            width: i, height: i,
            source_app: None, source_window_title: None, monitor: None,
        };
        let a = insert(&db, mk(1)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = insert(&db, mk(2)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let c = insert(&db, mk(3)).unwrap();

        // Soft-delete `b`
        db.with_conn(|conn| {
            conn.execute("UPDATE captures SET deleted_at=?1 WHERE id=?2", rusqlite::params![1_i64, &b.id])?;
            Ok(())
        }).unwrap();

        let rows = list(&db, ListCapturesQuery::default()).unwrap();
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec![c.id.as_str(), a.id.as_str()]);
    }
```

**Step 2: Run tests**

Run: `cargo test -p snk-library`
Expected: 8 tests pass.

**Step 3: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): list + get capture queries"
```

---

## Task 8: snk-library — file write helper

**Files:**
- Create: `crates/snk-library/src/files.rs`
- Modify: `crates/snk-library/src/lib.rs`

**Step 1: Write `crates/snk-library/src/files.rs`**

```rust
use std::path::{Path, PathBuf};

use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::Result;

/// Compute the relative path a new capture file should live at:
///   captures/YYYY/MM/<uuid>.png
pub fn capture_relative_path(id: &Uuid, ext: &str) -> PathBuf {
    let now = Utc::now();
    PathBuf::from("captures")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{id}.{ext}"))
}

/// Atomic-ish file write: write to <path>.tmp, fsync, rename.
/// Rename is atomic on the same filesystem on POSIX and on NTFS.
pub fn write_atomic(library_root: &Path, relative: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let full = library_root.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = full.with_extension(format!(
        "{}.tmp",
        full.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &full)?;
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_uses_year_month_dirs() {
        let id = Uuid::now_v7();
        let p = capture_relative_path(&id, "png");
        let s = p.to_string_lossy();
        assert!(s.starts_with("captures/") || s.starts_with("captures\\"));
        assert!(s.ends_with(".png"));
    }

    #[test]
    fn write_atomic_writes_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let rel = Path::new("captures/2026/05/x.png");
        let full = write_atomic(dir.path(), rel, b"hello").unwrap();
        assert!(full.exists());
        let read = std::fs::read(&full).unwrap();
        assert_eq!(read, b"hello");
        // tmp should be gone
        assert!(!full.with_extension("png.tmp").exists());
    }
}
```

**Step 2: Re-export — modify `crates/snk-library/src/lib.rs`**

```rust
//! snk-library — single owner of the SQLite persistence layer.

pub mod captures;
pub mod db;
pub mod error;
pub mod files;
pub mod migrate;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use db::Db;
pub use error::{LibraryError, Result};
```

**Step 3: Run tests**

Run: `cargo test -p snk-library`
Expected: 10 tests pass.

**Step 4: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): atomic file write + relative-path helper"
```

---

## Task 9: snk-library — Tauri plugin wiring

**Files:**
- Create: `crates/snk-library/src/plugin.rs`
- Create: `crates/snk-library/src/commands.rs`
- Modify: `crates/snk-library/src/lib.rs`
- Modify: `crates/snk-library/Cargo.toml`

**Step 1: Update Cargo features — modify `crates/snk-library/Cargo.toml`**

Find:

```toml
[dependencies]
tauri.workspace = true
```

Replace with:

```toml
[dependencies]
tauri = { workspace = true, features = [] }
```

**Step 2: Write `crates/snk-library/src/plugin.rs`**

```rust
use std::path::PathBuf;
use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

use crate::Db;

pub struct LibraryState {
    pub db: Arc<Db>,
    pub root: PathBuf,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-library")
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_captures,
            crate::commands::get_capture,
        ])
        .setup(|app, _api| {
            let root = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("resolve app data dir: {e}"))?;
            let db_path = root.join("snapper-keeper.db");
            let db = Db::open(&db_path).map_err(|e| format!("open db: {e}"))?;
            app.manage(LibraryState { db: Arc::new(db), root });
            Ok(())
        })
        .build()
}
```

**Step 3: Write `crates/snk-library/src/commands.rs`**

```rust
use tauri::{Runtime, State};

use crate::captures::{self, Capture, ListCapturesQuery};
use crate::plugin::LibraryState;
use crate::Result;

#[tauri::command]
pub fn list_captures<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    query: Option<ListCapturesQuery>,
) -> Result<Vec<Capture>> {
    captures::list(&state.db, query.unwrap_or_default())
}

#[tauri::command]
pub fn get_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<Capture> {
    captures::get(&state.db, &id)
}
```

**Step 4: Re-export — modify `crates/snk-library/src/lib.rs`**

```rust
//! snk-library — single owner of the SQLite persistence layer.

pub mod captures;
pub mod commands;
pub mod db;
pub mod error;
pub mod files;
pub mod migrate;
pub mod plugin;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use db::Db;
pub use error::{LibraryError, Result};
pub use plugin::{init, LibraryState};
```

**Step 5: Verify it compiles**

Run: `cargo build -p snk-library`
Expected: clean compile (no `tauri::generate_handler` errors).

Run: `cargo test -p snk-library`
Expected: 10 tests still pass.

**Step 6: Commit**

```bash
git add crates/snk-library/
git commit -m "feat(snk-library): Tauri plugin entry + list/get commands"
```

---

## Task 10: Scaffold the `snk-hotkeys` crate

**Files:**
- Create: `crates/snk-hotkeys/Cargo.toml`
- Create: `crates/snk-hotkeys/src/lib.rs`

**Step 1: Write `crates/snk-hotkeys/Cargo.toml`**

```toml
[package]
name = "snk-hotkeys"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
tauri.workspace = true
tauri-plugin-global-shortcut.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

**Step 2: Write `crates/snk-hotkeys/src/lib.rs`**

```rust
//! snk-hotkeys — register global hotkeys and emit events when triggered.
//!
//! Phase 1 wires a fixed set of action ids → default chords. A later phase
//! reads bindings from `snk-library` (settings) and supports remapping.

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HotkeyAction {
    CaptureFullScreen,
}

impl HotkeyAction {
    pub fn event_name(self) -> &'static str {
        match self {
            HotkeyAction::CaptureFullScreen => "hotkey:capture-full-screen",
        }
    }

    pub fn default_chord(self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            HotkeyAction::CaptureFullScreen => "Cmd+Shift+3",
        }
        #[cfg(not(target_os = "macos"))]
        match self {
            HotkeyAction::CaptureFullScreen => "CmdOrCtrl+Shift+3",
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-hotkeys")
        .setup(|app, _api| {
            // Defer registration until global-shortcut plugin is initialized.
            let handle = app.app_handle().clone();
            app.run_on_main_thread(move || {
                if let Err(e) = register_defaults(&handle) {
                    warn!(error = %e, "failed to register default hotkeys");
                }
            })
            .ok();
            Ok(())
        })
        .build()
}

fn register_defaults<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let actions = [HotkeyAction::CaptureFullScreen];
    for action in actions {
        let chord = action.default_chord();
        let app2 = app.clone();
        app.global_shortcut()
            .on_shortcut(chord, move |_app, _sc, ev| {
                if matches!(ev.state(), ShortcutState::Pressed) {
                    let _ = app2.emit(action.event_name(), ());
                }
            })
            .map_err(|e| format!("register {chord}: {e}"))?;
        info!(%chord, "registered hotkey");
    }
    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo build -p snk-hotkeys`
Expected: clean compile.

**Step 4: Commit**

```bash
git add crates/snk-hotkeys/
git commit -m "feat(snk-hotkeys): scaffold and register CaptureFullScreen hotkey"
```

---

## Task 11: Scaffold the `snk-capture` crate

**Files:**
- Create: `crates/snk-capture/Cargo.toml`
- Create: `crates/snk-capture/src/lib.rs`
- Create: `crates/snk-capture/src/error.rs`

**Step 1: Write `crates/snk-capture/Cargo.toml`**

```toml
[package]
name = "snk-capture"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
snk-library = { path = "../snk-library" }
tauri.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true
xcap.workspace = true
image.workspace = true
uuid.workspace = true
anyhow.workspace = true
```

**Step 2: Write `crates/snk-capture/src/error.rs`**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CaptureError {
    #[error("no monitors found")]
    NoMonitors,

    #[error("xcap error: {message}")]
    Os { message: String },

    #[error("encode error: {message}")]
    Encode { message: String },

    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),
}

impl From<xcap::XCapError> for CaptureError {
    fn from(e: xcap::XCapError) -> Self {
        CaptureError::Os { message: e.to_string() }
    }
}

impl From<image::ImageError> for CaptureError {
    fn from(e: image::ImageError) -> Self {
        CaptureError::Encode { message: e.to_string() }
    }
}

impl From<snk_library::LibraryError> for CaptureError {
    fn from(e: snk_library::LibraryError) -> Self {
        CaptureError::Library(e)
    }
}

pub type Result<T> = std::result::Result<T, CaptureError>;
```

**Step 3: Write `crates/snk-capture/src/lib.rs`**

```rust
//! snk-capture — screen capture entry points.
//!
//! Phase 1 supports primary-monitor full-screen capture only. Region overlay,
//! window capture, timed capture, and the floating post-capture toolbar come
//! in later phases.

pub mod error;

pub use error::{CaptureError, Result};
```

**Step 4: Verify it compiles**

Run: `cargo build -p snk-capture`
Expected: clean compile.

**Step 5: Commit**

```bash
git add crates/snk-capture/
git commit -m "feat(snk-capture): scaffold crate and error types"
```

---

## Task 12: snk-capture — primary monitor full-screen capture

**Files:**
- Create: `crates/snk-capture/src/grab.rs`
- Modify: `crates/snk-capture/src/lib.rs`

**Step 1: Write `crates/snk-capture/src/grab.rs`**

```rust
use std::io::Cursor;

use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use xcap::Monitor;

use crate::Result;

pub struct GrabResult {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub monitor_name: String,
}

pub fn grab_primary_monitor() -> Result<GrabResult> {
    let monitors = Monitor::all()?;
    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary())
        .or_else(|| Monitor::all().ok().and_then(|mut v| v.pop()))
        .ok_or(crate::CaptureError::NoMonitors)?;

    let image = primary.capture_image()?;
    let (w, h) = (image.width(), image.height());
    let name = primary.name().to_string();

    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(image.as_raw(), w, h, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
        width: w,
        height: h,
        monitor_name: name,
    })
}
```

**Step 2: Wire module — modify `crates/snk-capture/src/lib.rs`**

```rust
//! snk-capture — screen capture entry points.
//!
//! Phase 1 supports primary-monitor full-screen capture only. Region overlay,
//! window capture, timed capture, and the floating post-capture toolbar come
//! in later phases.

pub mod error;
pub mod grab;

pub use error::{CaptureError, Result};
pub use grab::{grab_primary_monitor, GrabResult};
```

**Step 3: Compile**

Run: `cargo build -p snk-capture`
Expected: clean compile.

Note: this code does not have automated tests. The `Monitor::all()` call hits real OS APIs and can't run on CI runners that lack a display. We rely on the manual smoke test in Task 20.

**Step 4: Commit**

```bash
git add crates/snk-capture/
git commit -m "feat(snk-capture): primary monitor full-screen grab via xcap"
```

---

## Task 13: snk-capture — orchestrator + Tauri plugin wiring

**Files:**
- Create: `crates/snk-capture/src/orchestrate.rs`
- Create: `crates/snk-capture/src/commands.rs`
- Create: `crates/snk-capture/src/plugin.rs`
- Modify: `crates/snk-capture/src/lib.rs`

**Step 1: Write `crates/snk-capture/src/orchestrate.rs`**

```rust
use std::sync::Arc;

use snk_library::{captures, files, Capture, Db, NewCapture};
use uuid::Uuid;

use crate::grab::{grab_primary_monitor, GrabResult};
use crate::Result;

/// Capture the primary monitor, write the PNG to disk, and insert a row.
/// Returns the persisted Capture row.
pub fn capture_full_screen(db: &Arc<Db>, library_root: &std::path::Path) -> Result<Capture> {
    let GrabResult { png_bytes, width, height, monitor_name } = grab_primary_monitor()?;
    let id = Uuid::now_v7();
    let relative = files::capture_relative_path(&id, "png");
    let _full = files::write_atomic(library_root, &relative, &png_bytes)?;
    let row = captures::insert(
        db,
        NewCapture {
            file_path: relative,
            width,
            height,
            source_app: None,
            source_window_title: None,
            monitor: Some(monitor_name),
        },
    )?;
    Ok(row)
}
```

**Step 2: Write `crates/snk-capture/src/commands.rs`**

```rust
use snk_library::{plugin::LibraryState, Capture};
use tauri::{Runtime, State};

use crate::Result;

#[tauri::command]
pub fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
) -> Result<Capture> {
    crate::orchestrate::capture_full_screen(&state.db, &state.root)
}
```

**Step 3: Write `crates/snk-capture/src/plugin.rs`**

```rust
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-capture")
        .invoke_handler(tauri::generate_handler![crate::commands::capture_full_screen])
        .build()
}
```

**Step 4: Re-export — modify `crates/snk-capture/src/lib.rs`**

```rust
//! snk-capture — screen capture entry points.
//!
//! Phase 1 supports primary-monitor full-screen capture only. Region overlay,
//! window capture, timed capture, and the floating post-capture toolbar come
//! in later phases.

pub mod commands;
pub mod error;
pub mod grab;
pub mod orchestrate;
pub mod plugin;

pub use error::{CaptureError, Result};
pub use grab::{grab_primary_monitor, GrabResult};
pub use plugin::init;
```

**Step 5: Compile**

Run: `cargo build -p snk-capture`
Expected: clean compile.

**Step 6: Commit**

```bash
git add crates/snk-capture/
git commit -m "feat(snk-capture): orchestrate capture → library write + Tauri plugin"
```

---

## Task 14: Initialize the Tauri app under `app/`

**Files:**
- Create: `app/package.json`
- Create: `app/vite.config.ts`
- Create: `app/tsconfig.json`
- Create: `app/index.html`
- Create: `app/src/main.tsx`
- Create: `app/src/App.tsx`
- Create: `app/src/index.css`
- Create: `app/postcss.config.cjs`
- Create: `app/tailwind.config.cjs`

**Step 1: Write `app/package.json`**

```json
{
  "name": "@snk/app",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "tauri": "tauri",
    "lint": "eslint src --max-warnings 0",
    "typecheck": "tsc -b --noEmit",
    "test": "echo 'no frontend tests yet'"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-global-shortcut": "^2.0.0",
    "@tanstack/react-query": "^5.40.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "zustand": "^4.5.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.4.0",
    "vite": "^5.2.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "typescript": "^5.4.0"
  }
}
```

**Step 2: Write `app/vite.config.ts`**

```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2022',
    sourcemap: true,
  },
});
```

**Step 3: Write `app/tsconfig.json`**

```json
{
  "extends": "../tsconfig.base.json",
  "include": ["src"],
  "compilerOptions": {
    "outDir": "dist",
    "noEmit": true
  }
}
```

**Step 4: Write `app/index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>snapper-keeper</title>
  </head>
  <body class="bg-slate-950 text-slate-100">
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

**Step 5: Write `app/src/index.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

html, body, #root {
  height: 100%;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}
```

**Step 6: Write `app/postcss.config.cjs`**

```js
module.exports = {
  plugins: { tailwindcss: {}, autoprefixer: {} },
};
```

**Step 7: Write `app/tailwind.config.cjs`**

```js
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {},
  },
  plugins: [],
};
```

**Step 8: Write `app/src/main.tsx`**

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import App from './App';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 5_000, refetchOnWindowFocus: false } },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
```

**Step 9: Write `app/src/App.tsx`** (placeholder; Task 17 replaces it)

```tsx
export default function App() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">snapper-keeper</h1>
      <p className="text-slate-400 mt-2">Initializing…</p>
    </main>
  );
}
```

**Step 10: Install**

Run: `pnpm install`
Expected: workspace resolves, no fatal errors.

**Step 11: Commit**

```bash
git add app/ pnpm-lock.yaml
git commit -m "feat(app): scaffold Vite + React + Tailwind frontend"
```

---

## Task 15: Scaffold `src-tauri` for `app/`

**Files:**
- Create: `app/src-tauri/Cargo.toml`
- Create: `app/src-tauri/build.rs`
- Create: `app/src-tauri/tauri.conf.json`
- Create: `app/src-tauri/src/main.rs`
- Create: `app/src-tauri/icons/icon.png` (placeholder — see Step 6)
- Create: `app/src-tauri/capabilities/default.json`

**Step 1: Write `app/src-tauri/Cargo.toml`**

```toml
[package]
name = "snapper-keeper-app"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[build-dependencies]
tauri-build.workspace = true

[dependencies]
tauri = { workspace = true, features = ["tray-icon"] }
tauri-plugin-global-shortcut.workspace = true
snk-library = { path = "../../crates/snk-library" }
snk-hotkeys = { path = "../../crates/snk-hotkeys" }
snk-capture = { path = "../../crates/snk-capture" }
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

**Step 2: Write `app/src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build();
}
```

**Step 3: Write `app/src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "snapper-keeper",
  "version": "0.0.1",
  "identifier": "com.snapper-keeper.app",
  "build": {
    "beforeDevCommand": "pnpm --filter @snk/app dev",
    "beforeBuildCommand": "pnpm --filter @snk/app build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "library",
        "title": "snapper-keeper",
        "width": 1100,
        "height": 720,
        "minWidth": 700,
        "minHeight": 500,
        "resizable": true,
        "visible": true,
        "decorations": true
      }
    ],
    "security": {
      "csp": null
    },
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.png"]
  }
}
```

**Step 4: Write `app/src-tauri/capabilities/default.json`**

```json
{
  "$schema": "https://schema.tauri.app/capabilities/2",
  "identifier": "default",
  "description": "Default permissions for the library window",
  "windows": ["library"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:event:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered"
  ]
}
```

**Step 5: Write `app/src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SNK_LOG").unwrap_or_else(|_| EnvFilter::new("info,snk=debug")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(snk_library::init())
        .plugin(snk_hotkeys::init())
        .plugin(snk_capture::init())
        .setup(|app| {
            // Build tray menu
            let capture_item = MenuItem::with_id(
                app,
                "tray:capture-full-screen",
                "Capture full screen",
                true,
                None::<&str>,
            )?;
            let open_lib = MenuItem::with_id(app, "tray:open-library", "Open library", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&capture_item, &open_lib, &quit])?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray:capture-full-screen" => {
                        let _ = app.emit("hotkey:capture-full-screen", ());
                    }
                    "tray:open-library" => {
                        if let Some(win) = app.get_webview_window("library") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "tray:quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(win) = tray.app_handle().get_webview_window("library") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            info!("snapper-keeper started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!(error = %e, "tauri runtime exited");
        });
}
```

**Step 6: Provide a placeholder icon**

The Tauri CLI requires an icon. Run from repo root:

```bash
pnpm --filter @snk/app tauri icon https://tauri.app/_astro/tauri_logo.svg
```

If you don't have network access, generate a placeholder 512×512 solid-color PNG yourself:

```bash
# Python one-liner (or use any image tool you prefer):
python -c "from PIL import Image; Image.new('RGBA',(512,512),(48,80,135,255)).save('app/src-tauri/icons/icon.png')"
```

Expected: `app/src-tauri/icons/icon.png` exists. (`tauri icon` will also generate platform-specific assets; that's a bonus.)

**Step 7: Verify it compiles**

Run: `cargo build -p snapper-keeper-app`
Expected: clean compile. (First build may take a few minutes — Tauri dependency graph.)

**Step 8: Commit**

```bash
git add app/src-tauri/
git commit -m "feat(app): scaffold src-tauri with tray + plugin registration"
```

---

## Task 16: Frontend TS packages — `@snk/library` and `@snk/capture`

**Files:**
- Create: `packages/snk-library/package.json`
- Create: `packages/snk-library/tsconfig.json`
- Create: `packages/snk-library/src/index.ts`
- Create: `packages/snk-library/src/types.ts`
- Create: `packages/snk-capture/package.json`
- Create: `packages/snk-capture/tsconfig.json`
- Create: `packages/snk-capture/src/index.ts`

**Step 1: Write `packages/snk-library/package.json`**

```json
{
  "name": "@snk/library",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "lint": "eslint src --max-warnings 0",
    "typecheck": "tsc -b --noEmit",
    "test": "echo 'no tests yet'"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0"
  },
  "devDependencies": {
    "typescript": "^5.4.0"
  }
}
```

**Step 2: Write `packages/snk-library/tsconfig.json`**

```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

**Step 3: Write `packages/snk-library/src/types.ts`**

```ts
export interface Capture {
  id: string;
  file_path: string;
  annotated_path: string | null;
  width: number;
  height: number;
  source_app: string | null;
  source_window_title: string | null;
  monitor: string | null;
  created_at: number;
  deleted_at: number | null;
  pinned: boolean;
}

export interface ListCapturesQuery {
  limit?: number;
  include_deleted?: boolean;
}
```

**Step 4: Write `packages/snk-library/src/index.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';

import type { Capture, ListCapturesQuery } from './types';

export * from './types';

export function listCaptures(query?: ListCapturesQuery): Promise<Capture[]> {
  return invoke<Capture[]>('plugin:snk-library|list_captures', { query });
}

export function getCapture(id: string): Promise<Capture> {
  return invoke<Capture>('plugin:snk-library|get_capture', { id });
}
```

**Step 5: Write `packages/snk-capture/package.json`**

```json
{
  "name": "@snk/capture",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "lint": "eslint src --max-warnings 0",
    "typecheck": "tsc -b --noEmit",
    "test": "echo 'no tests yet'"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@snk/library": "workspace:*"
  },
  "devDependencies": {
    "typescript": "^5.4.0"
  }
}
```

**Step 6: Write `packages/snk-capture/tsconfig.json`**

```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

**Step 7: Write `packages/snk-capture/src/index.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';

import type { Capture } from '@snk/library';

export const CAPTURE_FULL_SCREEN_EVENT = 'hotkey:capture-full-screen';

export function captureFullScreen(): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_full_screen');
}
```

**Step 8: Add packages to the app's deps — modify `app/package.json`**

Find:

```json
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
```

Replace with:

```json
  "dependencies": {
    "@snk/library": "workspace:*",
    "@snk/capture": "workspace:*",
    "@tauri-apps/api": "^2.0.0",
```

**Step 9: Re-resolve workspace**

Run: `pnpm install`
Expected: workspace links the new packages into `app/node_modules`.

Run: `pnpm typecheck`
Expected: typechecks clean across all packages.

**Step 10: Commit**

```bash
git add packages/ app/package.json pnpm-lock.yaml
git commit -m "feat(packages): TS bindings for snk-library and snk-capture"
```

---

## Task 17: Library window UI — capture grid

**Files:**
- Modify: `app/src/App.tsx`
- Create: `app/src/windows/library/LibraryWindow.tsx`
- Create: `app/src/windows/library/CaptureGrid.tsx`
- Create: `app/src/windows/library/Thumbnail.tsx`
- Create: `app/src/lib/queryKeys.ts`
- Create: `app/src/lib/assetUrl.ts`

**Step 1: Write `app/src/lib/queryKeys.ts`**

```ts
export const queryKeys = {
  captures: {
    list: (query?: { limit?: number; include_deleted?: boolean }) =>
      ['captures', 'list', query ?? {}] as const,
    one: (id: string) => ['captures', 'one', id] as const,
  },
};
```

**Step 2: Write `app/src/lib/assetUrl.ts`**

```ts
import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * Convert a library-relative file path to a webview-loadable URL.
 * The library root is the app data dir; Tauri's asset protocol serves it.
 */
export function captureAssetUrl(libraryRoot: string, relative: string): string {
  // Normalize separators
  const full = `${libraryRoot.replace(/[\\/]+$/, '')}/${relative.replace(/\\/g, '/')}`;
  return convertFileSrc(full);
}
```

**Step 3: Write `app/src/windows/library/Thumbnail.tsx`**

```tsx
import { useState } from 'react';

import type { Capture } from '@snk/library';

interface Props {
  capture: Capture;
  src: string;
}

export function Thumbnail({ capture, src }: Props) {
  const [loaded, setLoaded] = useState(false);
  return (
    <div className="bg-slate-900 border border-slate-800 rounded-md overflow-hidden">
      <div className="relative aspect-video bg-slate-950">
        <img
          src={src}
          alt={`Capture ${capture.id}`}
          onLoad={() => setLoaded(true)}
          className={`w-full h-full object-cover transition-opacity ${
            loaded ? 'opacity-100' : 'opacity-0'
          }`}
        />
      </div>
      <div className="px-2 py-1.5">
        <div className="text-xs text-slate-200 truncate">
          {new Date(capture.created_at).toLocaleTimeString()}
        </div>
        <div className="text-[10px] text-slate-500">
          {capture.width}×{capture.height}
          {capture.monitor ? ` · ${capture.monitor}` : ''}
        </div>
      </div>
    </div>
  );
}
```

**Step 4: Write `app/src/windows/library/CaptureGrid.tsx`**

```tsx
import { useQuery } from '@tanstack/react-query';
import { path } from '@tauri-apps/api';

import { listCaptures } from '@snk/library';

import { captureAssetUrl } from '../../lib/assetUrl';
import { queryKeys } from '../../lib/queryKeys';
import { Thumbnail } from './Thumbnail';

export function CaptureGrid() {
  const root = useQuery({
    queryKey: ['app-data-dir'],
    queryFn: () => path.appDataDir(),
  });
  const captures = useQuery({
    queryKey: queryKeys.captures.list(),
    queryFn: () => listCaptures(),
  });

  if (root.isLoading || captures.isLoading) {
    return <p className="text-slate-500">Loading…</p>;
  }
  if (root.error || captures.error) {
    return (
      <p className="text-red-400">
        Error loading library: {String(root.error ?? captures.error)}
      </p>
    );
  }

  const rows = captures.data ?? [];
  if (rows.length === 0) {
    return (
      <div className="text-slate-500 text-sm">
        No captures yet. Press the hotkey or use the tray menu.
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
      {rows.map((c) => (
        <Thumbnail key={c.id} capture={c} src={captureAssetUrl(root.data!, c.file_path)} />
      ))}
    </div>
  );
}
```

**Step 5: Write `app/src/windows/library/LibraryWindow.tsx`**

```tsx
import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';

import { CAPTURE_FULL_SCREEN_EVENT, captureFullScreen } from '@snk/capture';

import { queryKeys } from '../../lib/queryKeys';
import { CaptureGrid } from './CaptureGrid';

export function LibraryWindow() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen(CAPTURE_FULL_SCREEN_EVENT, async () => {
      try {
        await captureFullScreen();
      } catch (e) {
        console.error('capture failed', e);
      }
      await queryClient.invalidateQueries({ queryKey: ['captures'] });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('listen failed', e));
    return () => unlisten?.();
  }, [queryClient]);

  return (
    <main className="h-full flex flex-col">
      <header className="px-4 py-2 border-b border-slate-800 flex items-center gap-3">
        <h1 className="text-sm font-semibold">snapper-keeper</h1>
        <span className="text-xs text-slate-500">phase 1 · vertical slice</span>
        <div className="flex-1" />
        <button
          className="bg-slate-800 hover:bg-slate-700 text-slate-100 px-3 py-1 rounded text-xs"
          onClick={async () => {
            try {
              await captureFullScreen();
              await queryClient.invalidateQueries({ queryKey: ['captures'] });
            } catch (e) {
              console.error(e);
            }
          }}
        >
          Capture full screen
        </button>
      </header>
      <section className="flex-1 overflow-auto p-4">
        <CaptureGrid />
      </section>
    </main>
  );
}
```

**Step 6: Replace `app/src/App.tsx`**

```tsx
import { LibraryWindow } from './windows/library/LibraryWindow';

export default function App() {
  return <LibraryWindow />;
}
```

**Step 7: Sanity check**

Run: `pnpm --filter @snk/app typecheck`
Expected: typechecks clean.

Run: `pnpm --filter @snk/app build`
Expected: Vite builds `app/dist` without error.

**Step 8: Commit**

```bash
git add app/src/
git commit -m "feat(app): library window with capture grid and hotkey wiring"
```

---

## Task 18: Tauri asset protocol allow-list for the library directory

**Files:**
- Modify: `app/src-tauri/tauri.conf.json`
- Modify: `app/src-tauri/capabilities/default.json`

**Step 1: Allow asset access to the app data dir — modify `app/src-tauri/tauri.conf.json`**

Find:

```json
    "security": {
      "csp": null
    },
```

Replace with:

```json
    "security": {
      "csp": null,
      "assetProtocol": {
        "enable": true,
        "scope": {
          "allow": ["$APPDATA/**", "$APPLOCALDATA/**"]
        }
      }
    },
```

**Step 2: Grant the library window permission to use the asset protocol — modify `app/src-tauri/capabilities/default.json`**

Replace the existing `"permissions"` array with:

```json
  "permissions": [
    "core:default",
    "core:window:default",
    "core:event:default",
    "core:path:default",
    "core:asset:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered"
  ]
```

**Step 3: Sanity check**

Run: `cargo build -p snapper-keeper-app`
Expected: clean compile. The capability/config schema is validated at build time.

**Step 4: Commit**

```bash
git add app/src-tauri/tauri.conf.json app/src-tauri/capabilities/default.json
git commit -m "feat(app): allow asset protocol access to app data dir for thumbnails"
```

---

## Task 19: Manual end-to-end smoke

**Files:** none.

**Step 1: Run the app in dev**

Run: `pnpm --filter @snk/app tauri dev`
Expected:
- Vite dev server starts on port 5173.
- Rust crates compile (first run ~3–5 min).
- Tauri window labeled "snapper-keeper" opens showing "No captures yet."
- Tray icon appears in system tray / menu bar.

**Step 2: Capture via button**

Click the "Capture full screen" button in the window header.

Expected:
- Screen flickers (no — xcap doesn't flash; just a brief delay).
- A thumbnail appears in the grid showing the captured monitor.
- The captured PNG exists under your OS app data dir:
  - **macOS:** `~/Library/Application Support/com.snapper-keeper.app/captures/YYYY/MM/<uuid>.png`
  - **Windows:** `%APPDATA%\com.snapper-keeper.app\captures\YYYY\MM\<uuid>.png`

**Step 3: Capture via hotkey**

Press `Cmd+Shift+3` (macOS) or `Ctrl+Shift+3` (Windows).

> **Note (macOS):** the system screenshot shortcut on the same chord will also fire. Phase 2 adds the first-run wizard to disable that. For now, expect *both* shortcuts to fire — the system one writes to Desktop, ours writes to the library. We'll resolve this with the first-run flow later.

Expected: a new thumbnail appears in the library grid within ~1 second.

**Step 4: Capture via tray menu**

Click the tray icon → "Capture full screen".
Expected: a new thumbnail appears.

**Step 5: Stop the dev server**

`Ctrl+C` in the terminal running `tauri dev`.

**Step 6: No commit** (manual test only).

If anything failed, fix the underlying issue and commit the fix separately. Common pitfalls:

- **macOS Screen Recording permission**: a system prompt may appear on first capture attempt. Approve in System Settings → Privacy & Security → Screen Recording, then restart the dev session.
- **Tray icon missing**: verify `app/src-tauri/icons/icon.png` exists. Re-run `pnpm --filter @snk/app tauri icon <path>` if not.
- **Hotkey doesn't fire**: confirm `global-shortcut` plugin is registered in `main.rs` *before* `snk_hotkeys::init()`.

---

## Task 20: Frontend lint + typecheck scripts

**Files:**
- Modify: `packages/snk-library/package.json`
- Modify: `packages/snk-capture/package.json`
- Modify: `app/package.json`

(Most of these scripts already exist; this task verifies they all work end-to-end and adds anything missing.)

**Step 1: Run lint**

Run: `pnpm lint`
Expected: no errors. If there are unused-import or any-type warnings, fix them inline (do not suppress).

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: clean.

**Step 3: Commit any fixes**

```bash
git add -A
git commit -m "chore: lint + typecheck baseline green" || echo "nothing to commit"
```

---

## Task 21: CI workflow — lint, typecheck, build

**Files:**
- Create: `.github/workflows/ci.yml`

**Step 1: Write `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  lint-typecheck:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v3
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm lint
      - run: pnpm typecheck

  rust-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux Tauri deps
        # All plugin crates depend on `tauri`, which transitively links to webkit2gtk
        # via wry. We need these headers even just to compile, not just to run.
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev libxdo-dev
      - run: cargo fmt -- --check
      - run: cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
      - run: cargo test --workspace --exclude snapper-keeper-app

  build-app:
    needs: [lint-typecheck, rust-test]
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v3
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux deps
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev
      - run: pnpm install --frozen-lockfile
      - run: pnpm --filter @snk/app build
      - run: cargo build -p snapper-keeper-app
```

**Notes:**
- `snapper-keeper-app` is excluded from `cargo test` and `cargo clippy` in the rust-test job because building it requires platform-specific deps (the `build-app` job covers it).
- The `xcap` crate's screen-capture code path doesn't run during build, so Linux is fine for compile verification even though phase 1 doesn't target Linux as a release platform.

**Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: lint, typecheck, rust tests, and build matrix"
```

**Step 3: Push and verify (after PR)**

Once this is merged or pushed to a remote, the CI workflow runs and should pass. If it fails, address the failure with a follow-up commit.

---

## Task 22: README placeholder

**Files:**
- Create: `README.md`

**Step 1: Write `README.md`**

```markdown
# snapper-keeper

Cross-platform (Windows + macOS) screen capture and clipboard manager.

> **Status:** phase 1 (foundation + vertical slice). Not yet usable for daily work — see `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md` for the design and `docs/superpowers/plans/` for active plans.

## Development

Prereqs: Rust 1.78+, Node 20+, pnpm 9+, Tauri platform deps (https://tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm --filter @snk/app tauri dev
```

Build a release bundle:

```bash
pnpm --filter @snk/app tauri build
```

## Architecture

One Tauri plugin per feature. All persistence flows through `crates/snk-library`. See the design doc for the full plugin set and dependency rules.
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README"
```

---

## Done

At this point you should have:

- A working dev build that captures the primary monitor and persists it to the library
- A library window with a live-updating thumbnail grid
- Hotkey + tray menu + on-screen button as three entry points to capture
- Cargo + pnpm workspace, lint/typecheck/test scripts all green
- CI building on ubuntu / macOS / Windows

The vertical slice proves the architecture: plugin boundaries hold, `snk-library` is the single data owner, the frontend talks to plugins via typed bindings, and the OS surface (xcap, global-shortcut) is properly isolated behind plugin crates.

**Next phases (each its own plan document):**

- **Phase 2** — region-select overlay, window/timed capture, floating post-capture toolbar, source-app/window-title detection
- **Phase 3** — `snk-annotate` + annotation editor window
- **Phase 4** — `snk-clipboard` + clipboard popup, watcher, sensitive-flag filtering, auto-paste
- **Phase 5** — `snk-ocr` + Tesseract sidecar, FTS5 search migration, search UI
- **Phase 6** — library window polish (sidebar, smart sections, tags, settings)
- **Phase 7** — signing, notarization, auto-updater, release pipeline, first-run wizard
