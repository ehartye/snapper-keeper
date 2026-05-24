# Sensitive-clipboard Exclusion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Honor OS-level sensitive-clipboard flags (Windows + macOS) so password-manager copies never persist; add a user-managed app-blocklist as a safety net; drop the dead `sensitive` schema column.

**Architecture:** New per-OS modules under `crates/snk-clipboard/src/platform/` expose `is_sensitive()` and `current_source_app()`. The watcher's decision logic moves into a pure `worker_step` function for unit testing. Windows swaps polling for `AddClipboardFormatListener` + `WM_CLIPBOARDUPDATE`; macOS keeps polling but tightens to 100 ms. A new Settings → Clipboard panel writes the blocklist into the existing `clipboard.app_blocklist` setting; the watcher reads it fresh each event.

**Tech Stack:** Rust (`windows-rs` for Win32 API, `objc2-app-kit` for NSPasteboard/NSWorkspace), SQLite migration, Tauri 2 IPC, React + TS for the Settings UI.

**Approved design:** [`docs/superpowers/specs/2026-05-24-sensitive-clipboard-design.md`](../specs/2026-05-24-sensitive-clipboard-design.md)

---

## Task 1: Add native-OS crate dependencies

**Files:**
- Modify: `crates/snk-clipboard/Cargo.toml`

**Step 1: Extend the Windows feature list and add macOS app-kit dep**

Replace the existing `[target.'cfg(windows)'.dependencies]` and `[target.'cfg(target_os = "macos")'.dependencies]` blocks:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_System_DataExchange",
    "Win32_System_Memory",
    "Win32_System_Threading",
    "Win32_System_LibraryLoader",
    "Win32_Storage_FileSystem",
] }

[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.24"
core-foundation = "0.10"
objc2 = "0.5"
objc2-app-kit = { version = "0.2", features = ["NSPasteboard", "NSWorkspace", "NSRunningApplication"] }
objc2-foundation = { version = "0.2", features = ["NSString", "NSArray", "NSURL"] }
```

The `_DataExchange` features get us `IsClipboardFormatAvailable`, `OpenClipboard`, `GetClipboardData`, `RegisterClipboardFormatW`, `AddClipboardFormatListener`. `_Memory` covers `GlobalLock`/`GlobalUnlock` for reading the 4-byte values out of the registered formats. `_LibraryLoader` is for `GetModuleHandle`. `_Storage_FileSystem` is for `GetFileVersionInfoW`.

**Step 2: Verify it compiles**

Run: `cargo check -p snk-clipboard`
Expected: clean compile (no warnings). The new deps download + compile but no source code uses them yet.

**Step 3: Commit**

```bash
git add crates/snk-clipboard/Cargo.toml Cargo.lock
git commit -m "build(snk-clipboard): add Win32_System_DataExchange + objc2-app-kit deps"
```

---

## Task 2: V005 migration — drop the dead `sensitive` column

**Files:**
- Create: `crates/snk-library/migrations/V005__drop_clipboard_sensitive.sql`
- Modify: `crates/snk-library/src/migrate.rs`
- Test: `crates/snk-library/src/migrate.rs` (existing tests module)

**Step 1: Write the failing test**

Append to the `mod tests` in `crates/snk-library/src/migrate.rs`, after the existing `v003_creates_ocr_and_fts_tables`:

```rust
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
fn v004_to_v005_preserves_clipboard_rows() {
    use rusqlite::params;

    let mut conn = Connection::open_in_memory().unwrap();
    // Apply through V004 only.
    let v1_to_v4 = Migrations::new(vec![
        M::up(V001),
        M::up(V002),
        M::up(V003),
        M::up(V004),
    ]);
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
```

**Step 2: Run tests to verify they fail**

Cargo only accepts one positional test filter per invocation, so run each test separately:

```bash
cargo test -p snk-library migrate::tests::v005_drops_sensitive_column_from_clipboard_items -- --nocapture
cargo test -p snk-library migrate::tests::v004_to_v005_preserves_clipboard_rows -- --nocapture
```

Expected:
- `v005_drops_sensitive_column_from_clipboard_items` FAILS — `sensitive` column still present in `clipboard_items`.
- `v004_to_v005_preserves_clipboard_rows` PASSES trivially.

> **Test #2 framing:** This test is a regression guard, not a strict TDD red-green test. Before V005 exists, `migrate()` is a no-op past V004 and the test trivially passes. After V005 lands, the test becomes load-bearing — it catches accidental row destruction if anyone later replaces the `ALTER TABLE ... DROP COLUMN` with a destructive operation like `DELETE FROM`.

**Step 3: Add the migration file**

Create `crates/snk-library/migrations/V005__drop_clipboard_sensitive.sql`:

```sql
-- V005: drop the dead `sensitive` column from clipboard_items.
--
-- The column has been NOT NULL DEFAULT 0 since V002 and was never written
-- (no production query reads or writes it). Sensitive-clipboard exclusion
-- is now enforced at the watcher — content is dropped before it ever
-- reaches this table — so the column is unreachable. SQLite 3.35+
-- supports ALTER TABLE ... DROP COLUMN directly.
ALTER TABLE clipboard_items DROP COLUMN sensitive;
```

**Step 4: Register the migration**

Modify `crates/snk-library/src/migrate.rs`:

```rust
const V001: &str = include_str!("../migrations/V001__initial.sql");
const V002: &str = include_str!("../migrations/V002__clipboard_items.sql");
const V003: &str = include_str!("../migrations/V003__ocr_fts.sql");
const V004: &str = include_str!("../migrations/V004__annotation_state.sql");
const V005: &str = include_str!("../migrations/V005__drop_clipboard_sensitive.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(V001),
        M::up(V002),
        M::up(V003),
        M::up(V004),
        M::up(V005),
    ])
}
```

Also update the hardcoded version literal in `migrate()`:

```rust
pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations()
        .to_latest(conn)
        .map_err(|e| crate::LibraryError::Migration {
            from: 0,
            to: 5,
            recoverable: e.to_string().contains("Backup"),
        })?;
    Ok(())
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p snk-library migrate::`
Expected: 6/6 pass (4 existing + 2 new). If a `migration_count_matches_latest_applied_version` test lands later (it was mentioned in the original plan draft as coming from PR #80 but is not present in this branch), it would count .sql files in `migrations/` (now 5) and must still match the applied schema version.

**Step 6: Commit**

```bash
git add crates/snk-library/migrations/V005__drop_clipboard_sensitive.sql crates/snk-library/src/migrate.rs
git commit -m "feat(library): V005 drops dead clipboard_items.sensitive column"
```

---

## Task 3: `SourceApp` value object

**Files:**
- Create: `crates/snk-clipboard/src/source_app.rs`
- Modify: `crates/snk-clipboard/src/lib.rs` (or wherever module declarations live)

**Step 1: Find the lib.rs / module-declaration site**

Run: `head -20 crates/snk-clipboard/src/lib.rs`
Look for the `pub mod` declarations. The plan assumes `lib.rs` exists with declarations like `pub mod commands; pub mod watcher;` etc. If structured differently (e.g. `mod commands; pub use commands::*;`), match the existing style.

**Step 2: Write the failing test**

Create `crates/snk-clipboard/src/source_app.rs`:

```rust
//! Per-OS detection of "which app wrote to the clipboard".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAppKind {
    MacosBundleId,
    WindowsExe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceApp {
    pub identifier: String,
    pub display_name: String,
    pub kind: SourceAppKind,
}

impl SourceApp {
    /// Whether two identifiers refer to the same app — case rules differ
    /// per `kind` to match OS norms.
    pub fn identifier_matches(&self, other: &str) -> bool {
        match self.kind {
            SourceAppKind::WindowsExe => self.identifier.eq_ignore_ascii_case(other),
            SourceAppKind::MacosBundleId => self.identifier == other,
        }
    }
}

pub fn current() -> Option<SourceApp> {
    crate::platform::current_source_app()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_identifier_match_is_case_insensitive() {
        let app = SourceApp {
            identifier: "1Password.exe".into(),
            display_name: "1Password".into(),
            kind: SourceAppKind::WindowsExe,
        };
        assert!(app.identifier_matches("1password.exe"));
        assert!(app.identifier_matches("1PASSWORD.EXE"));
        assert!(!app.identifier_matches("KeePass.exe"));
    }

    #[test]
    fn macos_identifier_match_is_case_sensitive() {
        let app = SourceApp {
            identifier: "com.1password.1password8".into(),
            display_name: "1Password 8".into(),
            kind: SourceAppKind::MacosBundleId,
        };
        assert!(app.identifier_matches("com.1password.1password8"));
        assert!(!app.identifier_matches("COM.1password.1password8"));
        assert!(!app.identifier_matches("com.bitwarden.desktop"));
    }
}
```

**Step 3: Declare the module + a `platform` stub**

Add to `crates/snk-clipboard/src/lib.rs` (alongside existing module declarations):

```rust
pub mod source_app;
mod platform;
```

Create `crates/snk-clipboard/src/platform/mod.rs`:

```rust
//! Per-OS implementations re-exported through trait-shaped helpers.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::*;
#[cfg(target_os = "windows")]
pub(crate) use windows::*;

// Stub for OSes (Linux dev builds) without a real impl.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn current_source_app() -> Option<crate::source_app::SourceApp> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn is_sensitive() -> bool {
    false
}
```

Create matching empty stubs:
- `crates/snk-clipboard/src/platform/macos.rs`:
  ```rust
  pub(crate) fn current_source_app() -> Option<crate::source_app::SourceApp> { None }
  pub(crate) fn is_sensitive() -> bool { false }
  ```
- `crates/snk-clipboard/src/platform/windows.rs`:
  ```rust
  pub(crate) fn current_source_app() -> Option<crate::source_app::SourceApp> { None }
  pub(crate) fn is_sensitive() -> bool { false }
  ```

These stubs let everything compile end-to-end; later tasks replace each with the real native impl.

**Step 4: Run the tests**

Run: `cargo test -p snk-clipboard source_app::tests`
Expected: 2/2 pass.

**Step 5: Commit**

```bash
git add crates/snk-clipboard/src/source_app.rs crates/snk-clipboard/src/platform/
git add crates/snk-clipboard/src/lib.rs
git commit -m "feat(clipboard): scaffold SourceApp value object + platform stubs"
```

---

## Task 4: Blocklist module — DB-backed match logic

**Files:**
- Create: `crates/snk-clipboard/src/blocklist.rs`
- Modify: `crates/snk-clipboard/src/lib.rs` (add `pub mod blocklist;`)

**Step 1: Write the failing tests**

Create `crates/snk-clipboard/src/blocklist.rs`:

```rust
//! User-managed app-blocklist filter for the clipboard watcher.
//!
//! Reads the `clipboard.app_blocklist` setting via snk-library. Setting
//! shape is a JSON array of `BlocklistEntry`. Match is delegated to
//! `SourceApp::identifier_matches` so OS-specific case rules apply.

use serde::{Deserialize, Serialize};
use tracing::warn;

use snk_library::{settings, Db};

use crate::source_app::{SourceApp, SourceAppKind};

const SETTING_KEY: &str = "clipboard.app_blocklist";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub identifier: String,
    pub display_name: String,
    pub kind: SourceAppKind,
}

/// Returns true if `source` matches an entry in the persisted blocklist.
///
/// Fail-open: an unset setting, an empty array, or a malformed JSON value
/// all return false. The watcher therefore degrades to "OS flag only"
/// rather than failing the entire event loop.
pub fn matches(db: &Db, source: &SourceApp) -> bool {
    let raw = match settings::get(db, SETTING_KEY) {
        Ok(Some(v)) => v,
        Ok(None) => return false,
        Err(e) => {
            warn!(error = ?e, "blocklist setting read failed; treating as empty");
            return false;
        }
    };
    let entries: Vec<BlocklistEntry> = match serde_json::from_value(raw) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "blocklist setting malformed; treating as empty");
            return false;
        }
    };
    entries
        .iter()
        .any(|e| e.kind == source.kind && source.identifier_matches(&e.identifier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use snk_library::settings;

    // Reuse snk-library's test_support::fresh_db pattern via a tiny local
    // helper — that helper is private to snk-library, so we mint our own
    // here against the same crate's public API.
    fn fresh_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        let db = Db::open(&path).unwrap();
        (dir, db)
    }

    fn mac(id: &str) -> SourceApp {
        SourceApp {
            identifier: id.into(),
            display_name: id.into(),
            kind: SourceAppKind::MacosBundleId,
        }
    }

    fn win(id: &str) -> SourceApp {
        SourceApp {
            identifier: id.into(),
            display_name: id.into(),
            kind: SourceAppKind::WindowsExe,
        }
    }

    #[test]
    fn returns_false_when_setting_unset() {
        let (_t, db) = fresh_db();
        assert!(!matches(&db, &mac("com.x.y")));
    }

    #[test]
    fn returns_false_when_setting_is_empty_array() {
        let (_t, db) = fresh_db();
        settings::set(&db, SETTING_KEY, &json!([])).unwrap();
        assert!(!matches(&db, &win("foo.exe")));
    }

    #[test]
    fn returns_true_on_exact_match_macos() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "com.1password.1password8",
                "display_name": "1Password 8",
                "kind": "macos_bundle_id"
            }]),
        )
        .unwrap();
        assert!(matches(&db, &mac("com.1password.1password8")));
        assert!(!matches(&db, &mac("com.bitwarden.desktop")));
    }

    #[test]
    fn windows_match_is_case_insensitive() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "1Password.exe",
                "display_name": "1Password",
                "kind": "windows_exe"
            }]),
        )
        .unwrap();
        assert!(matches(&db, &win("1password.exe")));
        assert!(matches(&db, &win("1PASSWORD.EXE")));
    }

    #[test]
    fn macos_match_is_case_sensitive() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "com.example.app",
                "display_name": "App",
                "kind": "macos_bundle_id"
            }]),
        )
        .unwrap();
        assert!(matches(&db, &mac("com.example.app")));
        assert!(!matches(&db, &mac("Com.Example.App")));
    }

    #[test]
    fn cross_kind_entries_are_inert() {
        let (_t, db) = fresh_db();
        settings::set(
            &db,
            SETTING_KEY,
            &json!([{
                "identifier": "1Password.exe",
                "display_name": "1Password",
                "kind": "windows_exe"
            }]),
        )
        .unwrap();
        // macOS source can't match a windows_exe entry.
        assert!(!matches(&db, &mac("1Password.exe")));
    }

    #[test]
    fn malformed_json_falls_open() {
        let (_t, db) = fresh_db();
        // Plant a non-array JSON value directly.
        settings::set(&db, SETTING_KEY, &json!({"not": "an array"})).unwrap();
        assert!(!matches(&db, &win("foo.exe")));
    }
}
```

**Step 2: Declare the module**

Append to `crates/snk-clipboard/src/lib.rs`:

```rust
pub mod blocklist;
```

Also add `tempfile` and `serde_json` to `crates/snk-clipboard/Cargo.toml`'s `[dev-dependencies]` if not already present:

```toml
[dev-dependencies]
serde_json = { workspace = true }
tempfile = "3"
```

**Step 3: Run tests to verify they pass**

Run: `cargo test -p snk-clipboard blocklist::tests`
Expected: 7/7 pass.

**Step 4: Commit**

```bash
git add crates/snk-clipboard/src/blocklist.rs crates/snk-clipboard/src/lib.rs crates/snk-clipboard/Cargo.toml Cargo.lock
git commit -m "feat(clipboard): blocklist module with DB-backed match logic"
```

---

## Task 5: Sensitivity trait + fake probe for testability

**Files:**
- Create: `crates/snk-clipboard/src/sensitivity.rs`
- Modify: `crates/snk-clipboard/src/lib.rs`

**Step 1: Write the module + the FakeProbe and tests**

Create `crates/snk-clipboard/src/sensitivity.rs`:

```rust
//! Cross-OS sensitive-clipboard detection.
//!
//! The real implementations live in `platform/{macos,windows}.rs`. This
//! module exposes the public function the watcher calls, plus a trait
//! used by tests to inject a fake without touching the real OS clipboard.

/// Implementations of this trait answer "is the current OS clipboard
/// flagged as sensitive". Production = platform-specific OS call;
/// tests = `FakeProbe`.
pub trait SensitivityProbe: Send {
    fn is_sensitive(&self) -> bool;
}

/// Production probe — defers to the per-OS impl.
#[derive(Default)]
pub struct OsProbe;

impl SensitivityProbe for OsProbe {
    fn is_sensitive(&self) -> bool {
        crate::platform::is_sensitive()
    }
}

/// Test probe — returns a canned value. Use for unit tests of
/// downstream code (the watcher) without touching the OS clipboard.
#[derive(Debug, Clone, Copy)]
pub struct FakeProbe {
    pub answer: bool,
}

impl SensitivityProbe for FakeProbe {
    fn is_sensitive(&self) -> bool {
        self.answer
    }
}

/// Convenience for callers that don't want to manage a probe instance.
pub fn is_sensitive() -> bool {
    OsProbe.is_sensitive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_probe_returns_canned_value() {
        assert!(!FakeProbe { answer: false }.is_sensitive());
        assert!(FakeProbe { answer: true }.is_sensitive());
    }

    #[test]
    fn os_probe_delegates_to_platform_module() {
        // The Linux stub returns false; on macOS/Windows with no
        // concealed flag set, also false. Both are valid CI environments
        // for this test, so just exercise the path doesn't panic.
        let _ = OsProbe.is_sensitive();
    }
}
```

**Step 2: Declare the module**

Add to `crates/snk-clipboard/src/lib.rs`:

```rust
pub mod sensitivity;
```

**Step 3: Run tests**

Run: `cargo test -p snk-clipboard sensitivity::tests`
Expected: 2/2 pass.

**Step 4: Commit**

```bash
git add crates/snk-clipboard/src/sensitivity.rs crates/snk-clipboard/src/lib.rs
git commit -m "feat(clipboard): SensitivityProbe trait + FakeProbe for tests"
```

---

## Task 6: macOS sensitivity impl — NSPasteboard concealed types

**Files:**
- Modify: `crates/snk-clipboard/src/platform/macos.rs`

**Step 1: Replace the stub with a real impl**

Replace the entire contents of `crates/snk-clipboard/src/platform/macos.rs`:

```rust
//! macOS native implementations: NSPasteboard for clipboard flag
//! inspection, NSWorkspace for the frontmost-app source.

use objc2::rc::Retained;
use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSArray, NSString};

use crate::source_app::SourceApp;

const CONCEALED_TYPES: &[&str] = &[
    "org.nspasteboard.ConcealedType",
    "org.nspasteboard.TransientType",
    "org.nspasteboard.AutoGeneratedType",
];

pub(crate) fn is_sensitive() -> bool {
    // SAFETY: NSPasteboard generalPasteboard is a thread-safe singleton
    // accessor; objc2-app-kit's binding is `unsafe` because it crosses
    // the Objective-C boundary but the call itself has no preconditions.
    let pasteboard: Retained<NSPasteboard> = unsafe { NSPasteboard::generalPasteboard() };
    let types: Retained<NSArray<NSString>> = unsafe { pasteboard.types() };
    let len = types.len();
    for i in 0..len {
        let t: Retained<NSString> = types.objectAtIndex(i);
        let s = t.to_string();
        if CONCEALED_TYPES.iter().any(|c| *c == s.as_str()) {
            return true;
        }
    }
    false
}

pub(crate) fn current_source_app() -> Option<SourceApp> {
    // Placeholder for now — implemented in the next task.
    None
}
```

**Step 2: Verify it still compiles**

On macOS, run: `cargo check -p snk-clipboard --target aarch64-apple-darwin`
On Windows/Linux dev, this code is gated by `cfg(target_os = "macos")` and won't be compiled — verify with: `cargo check -p snk-clipboard` (should be clean).

If you're not on macOS, also push a temporary branch and let CI exercise the macOS-arm64 leg via PR.

**Step 3: Commit**

```bash
git add crates/snk-clipboard/src/platform/macos.rs
git commit -m "feat(clipboard): macOS sensitivity check via NSPasteboard concealed types"
```

---

## Task 7: macOS source-app impl — NSWorkspace frontmost

**Files:**
- Modify: `crates/snk-clipboard/src/platform/macos.rs`

**Step 1: Replace `current_source_app` with the real impl**

Add the import at the top of the file:

```rust
use objc2_app_kit::{NSPasteboard, NSWorkspace, NSRunningApplication};
```

Replace the placeholder `current_source_app`:

```rust
pub(crate) fn current_source_app() -> Option<SourceApp> {
    // SAFETY: NSWorkspace::sharedWorkspace and frontmostApplication are
    // thread-safe Cocoa singletons. The returned NSRunningApplication may
    // be nil (returned as Option) during fast app switches.
    let workspace: Retained<NSWorkspace> = unsafe { NSWorkspace::sharedWorkspace() };
    let app: Retained<NSRunningApplication> = unsafe { workspace.frontmostApplication() }?;

    let bundle_id: String = unsafe { app.bundleIdentifier() }?.to_string();
    let display_name: String = unsafe { app.localizedName() }
        .map(|s| s.to_string())
        .unwrap_or_else(|| bundle_id.clone());

    Some(SourceApp {
        identifier: bundle_id,
        display_name,
        kind: crate::source_app::SourceAppKind::MacosBundleId,
    })
}
```

**Step 2: Verify compile**

On macOS: `cargo check -p snk-clipboard --target aarch64-apple-darwin`
Expected: clean.

**Step 3: Commit**

```bash
git add crates/snk-clipboard/src/platform/macos.rs
git commit -m "feat(clipboard): macOS source-app detection via NSWorkspace"
```

---

## Task 8: Windows sensitivity impl — clipboard format inspection

**Files:**
- Modify: `crates/snk-clipboard/src/platform/windows.rs`

**Step 1: Replace the stub**

Replace the entire contents of `crates/snk-clipboard/src/platform/windows.rs`:

```rust
//! Windows native implementations: Win32 clipboard API for sensitive
//! flag inspection, foreground-window lookup for source-app detection.

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

use crate::source_app::SourceApp;

// The two registered formats Win+V honors (and the older
// CFSTR_EXCLUDECLIPBOARDCONTENTFROMMONITORING).
const FMT_EXCLUDE_FROM_MONITORING: &str = "ExcludeClipboardContentFromMonitoring";
const FMT_CAN_INCLUDE_IN_HISTORY: &str = "CanIncludeInClipboardHistory";
const FMT_CAN_UPLOAD_TO_CLOUD: &str = "CanUploadToCloudClipboard";

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn register_format(name: &str) -> u32 {
    let wide = wide_null(name);
    // SAFETY: RegisterClipboardFormatW is reentrant + thread-safe; same
    // format name returns the same id on repeated calls.
    unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) }
}

/// Read a `u32` value out of a clipboard format that stores a single
/// DWORD. Returns `None` if the format isn't present or the data isn't
/// the expected size.
fn read_u32_format(fmt: u32) -> Option<u32> {
    unsafe {
        if !IsClipboardFormatAvailable(fmt).as_bool() {
            return None;
        }
        // The watcher already holds the clipboard open when this is
        // called from the WM_CLIPBOARDUPDATE handler. For belt-and-
        // suspenders, OpenClipboard(HWND(0)) is a no-op if we already
        // own it; we don't call CloseClipboard here.
        let handle: HANDLE = GetClipboardData(fmt).ok()?;
        let ptr: *mut c_void = GlobalLock(handle.0 as _);
        if ptr.is_null() {
            return None;
        }
        let value = *(ptr as *const u32);
        let _ = GlobalUnlock(handle.0 as _);
        Some(value)
    }
}

pub(crate) fn is_sensitive() -> bool {
    let exclude = register_format(FMT_EXCLUDE_FROM_MONITORING);
    let can_include = register_format(FMT_CAN_INCLUDE_IN_HISTORY);
    let can_upload = register_format(FMT_CAN_UPLOAD_TO_CLOUD);

    // CFSTR_EXCLUDECLIPBOARDCONTENTFROMMONITORING is presence-only — its
    // existence flags the content as excluded.
    unsafe {
        if IsClipboardFormatAvailable(exclude).as_bool() {
            return true;
        }
    }

    // CanIncludeInClipboardHistory / CanUploadToCloudClipboard are DWORDs
    // with value 0 = "do not include / do not upload".
    if matches!(read_u32_format(can_include), Some(0)) {
        return true;
    }
    if matches!(read_u32_format(can_upload), Some(0)) {
        return true;
    }
    false
}

pub(crate) fn current_source_app() -> Option<SourceApp> {
    // Placeholder — implemented in the next task.
    None
}
```

> **NOTE for the implementer:** `is_sensitive` assumes the caller (the new Windows event-driven watcher in Task 12) has already called `OpenClipboard(HWND(0))`. If called outside that context the `GetClipboardData` calls will fail and the function will return `false` (fail-open is OK here because the WM_CLIPBOARDUPDATE-driven watcher always owns the clipboard at the moment of the check). If you need to call this from elsewhere, wrap the body in `OpenClipboard`/`CloseClipboard`.

**Step 2: Verify compile**

Run: `cargo check -p snk-clipboard --target x86_64-pc-windows-msvc`
(Or `--target x86_64-apple-darwin` to skip the Windows path locally — the macOS leg already compiles independently.)

**Step 3: Commit**

```bash
git add crates/snk-clipboard/src/platform/windows.rs
git commit -m "feat(clipboard): Windows sensitivity check via clipboard format inspection"
```

---

## Task 9: Windows source-app impl — foreground process exe basename

**Files:**
- Modify: `crates/snk-clipboard/src/platform/windows.rs`

**Step 1: Add the foreground-app code**

Append to `crates/snk-clipboard/src/platform/windows.rs` (and add imports at the top):

```rust
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_NAME_FORMAT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
```

Replace the placeholder `current_source_app`:

```rust
pub(crate) fn current_source_app() -> Option<SourceApp> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        let tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if tid == 0 || pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf: [u16; 1024] = [0; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        if ok.is_err() || len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let exe = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())?;

        // display_name = FileDescription from version info if available,
        // else the exe basename.
        let display_name = file_description(&path).unwrap_or_else(|| exe.clone());

        Some(SourceApp {
            identifier: exe.to_ascii_lowercase(),
            display_name,
            kind: crate::source_app::SourceAppKind::WindowsExe,
        })
    }
}

fn file_description(exe_path: &str) -> Option<String> {
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };

    let wide_path = wide_null(exe_path);
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(wide_path.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buf: Vec<u8> = vec![0; size as usize];
        if GetFileVersionInfoW(
            PCWSTR(wide_path.as_ptr()),
            0,
            size,
            buf.as_mut_ptr() as *mut c_void,
        )
        .is_err()
        {
            return None;
        }
        // Use the language-neutral 040904B0 codepage to query
        // FileDescription. (Most binaries ship at least the English entry.)
        let sub_block = wide_null(r"\StringFileInfo\040904B0\FileDescription");
        let mut value_ptr: *mut c_void = std::ptr::null_mut();
        let mut value_len: u32 = 0;
        let ok = VerQueryValueW(
            buf.as_ptr() as *const c_void,
            PCWSTR(sub_block.as_ptr()),
            &mut value_ptr,
            &mut value_len,
        );
        if !ok.as_bool() || value_ptr.is_null() || value_len == 0 {
            return None;
        }
        let slice = std::slice::from_raw_parts(value_ptr as *const u16, value_len as usize);
        let s = String::from_utf16_lossy(slice);
        Some(s.trim_end_matches('\0').to_string())
    }
}
```

**Step 2: Verify compile**

Run: `cargo check -p snk-clipboard --target x86_64-pc-windows-msvc`
Expected: clean.

**Step 3: Commit**

```bash
git add crates/snk-clipboard/src/platform/windows.rs
git commit -m "feat(clipboard): Windows source-app via GetForegroundWindow + version info"
```

---

## Task 10: Extract `worker_step` from the watcher

**Files:**
- Modify: `crates/snk-clipboard/src/watcher.rs`

**Step 1: Add the pure-function types + skeleton**

Insert near the top of `crates/snk-clipboard/src/watcher.rs`, after the existing `use` statements:

```rust
use std::path::Path;

use crate::blocklist;
use crate::sensitivity::{self, SensitivityProbe};
use crate::source_app::{self, SourceApp};

/// A single observed clipboard change that the watcher must decide
/// what to do with.
pub(crate) enum ClipboardEvent {
    /// Text content was on the clipboard at the time of observation.
    Text(String),
    /// Image bytes were on the clipboard (already PNG-encoded by arboard).
    Image(Vec<u8>),
}

/// Why the watcher did not record a particular event.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkipReason {
    SensitiveFlag,
    AppBlocked(String), // identifier
    DuplicateHash,
    EmptyContent,
}

/// Outcome of a single decision cycle.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StepResult {
    Saved { item_id: String },
    DedupedTo { existing_id: String },
    Skipped(SkipReason),
}

/// Shared per-thread state the watcher carries across cycles.
pub(crate) struct WatcherState {
    pub last_hash: Option<String>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self { last_hash: None }
    }
}
```

**Step 2: Add the pure decision function below the existing module body**

```rust
/// Pure decision cycle. The probe + source-app lookup are injected so
/// unit tests can run this without touching the real OS clipboard.
pub(crate) fn worker_step(
    event: ClipboardEvent,
    state: &mut WatcherState,
    db: &Db,
    library_root: &Path,
    probe: &dyn SensitivityProbe,
    source: Option<SourceApp>,
) -> StepResult {
    if probe.is_sensitive() {
        // Record the hash so a follow-up identical observation doesn't
        // re-run the whole pipeline. We compute it cheaply from the event.
        state.last_hash = Some(hash_of_event(&event));
        return StepResult::Skipped(SkipReason::SensitiveFlag);
    }

    if let Some(ref src) = source {
        if blocklist::matches(db, src) {
            state.last_hash = Some(hash_of_event(&event));
            return StepResult::Skipped(SkipReason::AppBlocked(src.identifier.clone()));
        }
    }

    match event {
        ClipboardEvent::Text(text) => {
            if text.is_empty() {
                return StepResult::Skipped(SkipReason::EmptyContent);
            }
            let hash = crate::hasher::hash_text(&text);
            if state.last_hash.as_deref() == Some(&hash) {
                return StepResult::Skipped(SkipReason::DuplicateHash);
            }
            state.last_hash = Some(hash.clone());

            match snk_library::clipboard::find_by_hash(db, &hash) {
                Ok(Some(existing)) => {
                    let _ = snk_library::clipboard::bump_timestamp(db, &existing.id);
                    StepResult::DedupedTo { existing_id: existing.id }
                }
                _ => {
                    let new_item = NewClipboardItem {
                        kind: ClipboardItemKind::Text,
                        text_content: Some(text),
                        file_path: None,
                        content_hash: hash,
                        source_app: source.as_ref().map(|s| s.identifier.clone()),
                        source_window_title: None,
                    };
                    match snk_library::clipboard::insert(db, new_item) {
                        Ok(item) => {
                            let _ = snk_library::clipboard::evict_unpinned(db, MAX_UNPINNED);
                            StepResult::Saved { item_id: item.id }
                        }
                        Err(_) => StepResult::Skipped(SkipReason::EmptyContent),
                    }
                }
            }
        }
        ClipboardEvent::Image(bytes) => {
            if bytes.is_empty() {
                return StepResult::Skipped(SkipReason::EmptyContent);
            }
            let hash = crate::hasher::hash_image_bytes(&bytes);
            if state.last_hash.as_deref() == Some(&hash) {
                return StepResult::Skipped(SkipReason::DuplicateHash);
            }
            state.last_hash = Some(hash.clone());

            match snk_library::clipboard::find_by_hash(db, &hash) {
                Ok(Some(existing)) => {
                    let _ = snk_library::clipboard::bump_timestamp(db, &existing.id);
                    StepResult::DedupedTo { existing_id: existing.id }
                }
                _ => {
                    let id = uuid::Uuid::now_v7();
                    let relative = files::clipboard_image_relative_path(&id);
                    if files::write_atomic(library_root, &relative, &bytes).is_err() {
                        return StepResult::Skipped(SkipReason::EmptyContent);
                    }
                    let new_item = NewClipboardItem {
                        kind: ClipboardItemKind::Image,
                        text_content: None,
                        file_path: Some(relative),
                        content_hash: hash,
                        source_app: source.as_ref().map(|s| s.identifier.clone()),
                        source_window_title: None,
                    };
                    match snk_library::clipboard::insert(db, new_item) {
                        Ok(item) => {
                            let _ = snk_library::clipboard::evict_unpinned(db, MAX_UNPINNED);
                            StepResult::Saved { item_id: item.id }
                        }
                        Err(_) => StepResult::Skipped(SkipReason::EmptyContent),
                    }
                }
            }
        }
    }
}

fn hash_of_event(event: &ClipboardEvent) -> String {
    match event {
        ClipboardEvent::Text(t) => crate::hasher::hash_text(t),
        ClipboardEvent::Image(b) => crate::hasher::hash_image_bytes(b),
    }
}
```

**Step 3: Compile**

Run: `cargo check -p snk-clipboard`
Expected: clean. (`poll_text`/`poll_image` still exist; we leave them in place until Task 11 replaces them.)

**Step 4: Commit**

```bash
git add crates/snk-clipboard/src/watcher.rs
git commit -m "refactor(clipboard): extract pure worker_step from watcher (#57 for this plugin)"
```

---

## Task 11: Unit-test `worker_step` exhaustively

**Files:**
- Modify: `crates/snk-clipboard/src/watcher.rs` (append a `#[cfg(test)] mod tests`)

**Step 1: Add the tests**

Append to `crates/snk-clipboard/src/watcher.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitivity::FakeProbe;
    use crate::source_app::{SourceApp, SourceAppKind};
    use serde_json::json;
    use snk_library::settings;

    fn fresh_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        let db = Db::open(&path).unwrap();
        (dir, db)
    }

    #[test]
    fn sensitive_flag_skips_without_persisting() {
        let (tmp, db) = fresh_db();
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text("secret".into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: true },
            None,
        );
        assert_eq!(result, StepResult::Skipped(SkipReason::SensitiveFlag));
        assert!(state.last_hash.is_some(), "last_hash should be set on skip");

        let count: i64 = db
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(count, 0, "no row should be inserted on sensitive skip");
    }

    #[test]
    fn blocked_app_skips_without_persisting() {
        let (tmp, db) = fresh_db();
        settings::set(
            &db,
            "clipboard.app_blocklist",
            &json!([{
                "identifier": "1password.exe",
                "display_name": "1Password",
                "kind": "windows_exe"
            }]),
        )
        .unwrap();
        let src = SourceApp {
            identifier: "1password.exe".into(),
            display_name: "1Password".into(),
            kind: SourceAppKind::WindowsExe,
        };
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text("password123".into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            Some(src.clone()),
        );
        assert_eq!(result, StepResult::Skipped(SkipReason::AppBlocked(src.identifier)));
        let count: i64 = db
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn allowed_text_event_is_saved_with_source_app() {
        let (tmp, db) = fresh_db();
        let src = SourceApp {
            identifier: "code.exe".into(),
            display_name: "Visual Studio Code".into(),
            kind: SourceAppKind::WindowsExe,
        };
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text("hello".into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            Some(src.clone()),
        );
        match result {
            StepResult::Saved { item_id } => {
                let stored: Option<String> = db
                    .with_conn(|c| {
                        c.query_row(
                            "SELECT source_app FROM clipboard_items WHERE id = ?1",
                            [&item_id],
                            |r| r.get(0),
                        )
                    })
                    .unwrap();
                assert_eq!(stored, Some(src.identifier));
            }
            other => panic!("expected Saved, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_hash_skips_without_re_inserting() {
        let (tmp, db) = fresh_db();
        let mut state = WatcherState::new();
        let probe = FakeProbe { answer: false };

        let first = worker_step(
            ClipboardEvent::Text("dup".into()),
            &mut state,
            &db,
            tmp.path(),
            &probe,
            None,
        );
        assert!(matches!(first, StepResult::Saved { .. }));

        let second = worker_step(
            ClipboardEvent::Text("dup".into()),
            &mut state,
            &db,
            tmp.path(),
            &probe,
            None,
        );
        assert_eq!(second, StepResult::Skipped(SkipReason::DuplicateHash));
    }

    #[test]
    fn empty_text_is_skipped() {
        let (tmp, db) = fresh_db();
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text(String::new()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            None,
        );
        assert_eq!(result, StepResult::Skipped(SkipReason::EmptyContent));
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p snk-clipboard watcher::tests`
Expected: 5/5 pass.

**Step 3: Commit**

```bash
git add crates/snk-clipboard/src/watcher.rs
git commit -m "test(clipboard): worker_step decision-matrix unit tests"
```

---

## Task 12: macOS polling — tighten interval + wire sensitivity in

**Files:**
- Modify: `crates/snk-clipboard/src/watcher.rs`

**Step 1: Replace the macOS-applicable polling loop**

Locate `pub fn start_watcher(db: Arc<Db>, library_root: std::path::PathBuf)` in `watcher.rs`. Replace the entire function body with:

```rust
pub fn start_watcher(db: Arc<Db>, library_root: std::path::PathBuf) {
    #[cfg(target_os = "windows")]
    {
        crate::platform_watcher::windows::start(db, library_root);
        return;
    }

    #[cfg(not(target_os = "windows"))]
    {
        start_polling(db, library_root, std::time::Duration::from_millis(100));
    }
}

#[cfg(not(target_os = "windows"))]
fn start_polling(
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    interval: std::time::Duration,
) {
    use crate::sensitivity::OsProbe;
    use crate::source_app;

    std::thread::spawn(move || {
        let mut clip = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "failed to open clipboard for watching");
                return;
            }
        };
        let mut state = WatcherState::new();
        let probe = OsProbe;

        loop {
            std::thread::sleep(interval);
            if SKIP_NEXT
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                debug!("skipping own clipboard write");
                continue;
            }

            // Try text first; image only if text is absent.
            let event = if let Ok(t) = clip.get_text() {
                if t.is_empty() {
                    continue;
                }
                ClipboardEvent::Text(t)
            } else if let Ok(img) = clip.get_image() {
                if img.bytes.is_empty() {
                    continue;
                }
                ClipboardEvent::Image(img.bytes.into_owned())
            } else {
                continue;
            };

            let source = source_app::current();
            let _ = worker_step(event, &mut state, &db, &library_root, &probe, source);
        }
    });
}
```

You'll also want to delete the now-dead `poll_text` and `poll_image` helpers in the same file. The `MAX_UNPINNED` constant is still used by `worker_step`, leave it.

**Step 2: Compile and run existing tests**

Run: `cargo test -p snk-clipboard`
Expected: all `worker_step` tests still pass; new code compiles. On non-Windows, the function references `platform_watcher::windows::start` only under `cfg(target_os = "windows")` so it doesn't need to exist yet — but to keep the compiler quiet, add a stub module:

Create `crates/snk-clipboard/src/platform_watcher.rs`:

```rust
//! Per-OS event-driven clipboard observation. macOS uses polling
//! (handled directly in `watcher.rs`); Windows uses
//! AddClipboardFormatListener + WM_CLIPBOARDUPDATE.

#[cfg(target_os = "windows")]
pub mod windows {
    use std::sync::Arc;

    use snk_library::Db;

    pub fn start(_db: Arc<Db>, _library_root: std::path::PathBuf) {
        // Placeholder until Task 13.
    }
}
```

Add `pub mod platform_watcher;` to `crates/snk-clipboard/src/lib.rs`.

**Step 3: Commit**

```bash
git add crates/snk-clipboard/src/watcher.rs crates/snk-clipboard/src/platform_watcher.rs crates/snk-clipboard/src/lib.rs
git commit -m "feat(clipboard): tighten macOS poll to 100ms, route through worker_step"
```

---

## Task 13: Windows event-driven watcher

**Files:**
- Modify: `crates/snk-clipboard/src/platform_watcher.rs`

**Step 1: Implement the message-only window**

Replace the `windows` submodule body in `crates/snk-clipboard/src/platform_watcher.rs`:

```rust
#[cfg(target_os = "windows")]
pub mod windows {
    use std::path::PathBuf;
    use std::sync::Arc;

    use arboard::Clipboard;
    use snk_library::Db;
    use tracing::{debug, error, warn};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassExW,
        TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
        WNDCLASSEXW,
    };

    use crate::sensitivity::OsProbe;
    use crate::source_app;
    use crate::watcher::{worker_step, ClipboardEvent, WatcherState, SKIP_NEXT};
    use std::sync::atomic::Ordering;

    /// Storage handed to the window-procedure callback via window
    /// user-data. Kept off the stack so the window callback can find it.
    struct WatcherCtx {
        db: Arc<Db>,
        library_root: PathBuf,
        state: WatcherState,
        probe: OsProbe,
        clipboard: Clipboard,
    }

    static mut CTX: Option<WatcherCtx> = None;

    pub fn start(db: Arc<Db>, library_root: PathBuf) {
        // Spin up a dedicated thread that owns the message-only window.
        // The watcher must run on this thread because WM_CLIPBOARDUPDATE
        // is dispatched into the thread that owns the listener handle.
        std::thread::Builder::new()
            .name("snk-clipboard-listener".into())
            .spawn(move || {
                let clipboard = match Clipboard::new() {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "failed to open clipboard for watching");
                        return;
                    }
                };
                // SAFETY: the watcher thread is the only writer to CTX.
                unsafe {
                    CTX = Some(WatcherCtx {
                        db,
                        library_root,
                        state: WatcherState::new(),
                        probe: OsProbe,
                        clipboard,
                    });
                }
                run_message_loop();
            })
            .expect("spawn snk-clipboard-listener thread");
    }

    fn run_message_loop() {
        unsafe {
            let instance = GetModuleHandleW(None).expect("module handle");
            let class_name = w!("SnkClipboardListener");
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };
            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                w!("snk-clipboard"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
            .expect("create message-only window");

            if AddClipboardFormatListener(hwnd).is_err() {
                error!("AddClipboardFormatListener failed; falling back to polling thread");
                let _ = RemoveClipboardFormatListener(hwnd);
                // Fallback path: spawn the polling loop. CTX is already
                // owned by this thread, but the fallback runs the polling
                // logic against the same Db / library_root.
                if let Some(ctx) = CTX.as_ref() {
                    crate::watcher::start_polling(
                        ctx.db.clone(),
                        ctx.library_root.clone(),
                        std::time::Duration::from_millis(500),
                    );
                }
                return;
            }

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, Some(hwnd), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = RemoveClipboardFormatListener(hwnd);
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_CLIPBOARDUPDATE {
            handle_clipboard_update();
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn handle_clipboard_update() {
        if SKIP_NEXT
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            debug!("skipping own clipboard write");
            return;
        }

        let ctx = unsafe { CTX.as_mut() }
            .expect("watcher CTX initialized before clipboard event");

        // Try text first, then image. arboard reads from the system
        // clipboard, which the WM_CLIPBOARDUPDATE handler can access
        // without explicit OpenClipboard (arboard handles that itself).
        let event = match ctx.clipboard.get_text() {
            Ok(t) if !t.is_empty() => ClipboardEvent::Text(t),
            _ => match ctx.clipboard.get_image() {
                Ok(img) if !img.bytes.is_empty() => {
                    ClipboardEvent::Image(img.bytes.into_owned())
                }
                _ => return,
            },
        };

        let source = source_app::current();
        let result = worker_step(
            event,
            &mut ctx.state,
            &ctx.db,
            &ctx.library_root,
            &ctx.probe,
            source,
        );
        match result {
            crate::watcher::StepResult::Skipped(reason) => {
                debug!(?reason, "clipboard event skipped");
            }
            crate::watcher::StepResult::Saved { item_id } => {
                debug!(item_id, "clipboard event saved");
            }
            crate::watcher::StepResult::DedupedTo { existing_id } => {
                debug!(existing_id, "clipboard event deduplicated");
            }
        }
    }
}
```

> **NOTE on `start_polling` cross-module access:** the fallback path calls back into `watcher::start_polling`. Mark that function `pub(crate)` in `watcher.rs` so the fallback can reach it. Originally it was a private helper inside `watcher.rs`; the change is one keyword.

**Step 2: Make `start_polling` and `SKIP_NEXT` visible to platform_watcher**

In `crates/snk-clipboard/src/watcher.rs`:

- Change `static SKIP_NEXT: AtomicBool` → `pub(crate) static SKIP_NEXT: AtomicBool`.
- Change `fn start_polling` → `pub(crate) fn start_polling`.
- Also make `ClipboardEvent`, `WatcherState`, `StepResult`, `worker_step` all `pub(crate)` if they aren't already.

**Step 3: Compile on Windows**

Run: `cargo check -p snk-clipboard --target x86_64-pc-windows-msvc`
Expected: clean. The message-only window code is `cfg(target_os = "windows")` so it has no impact on macOS/Linux.

**Step 4: Commit**

```bash
git add crates/snk-clipboard/src/platform_watcher.rs crates/snk-clipboard/src/watcher.rs
git commit -m "feat(clipboard): Windows event-driven watcher via WM_CLIPBOARDUPDATE"
```

---

## Task 14: IPC command — `detect_frontmost_app`

**Files:**
- Modify: `crates/snk-clipboard/src/commands.rs`
- Modify: `crates/snk-clipboard/src/plugin.rs`
- Modify: `crates/snk-clipboard/build.rs`
- Modify: `crates/snk-clipboard/permissions/default.toml`

**Step 1: Add the command**

Append to `crates/snk-clipboard/src/commands.rs`:

```rust
use crate::source_app::{self, SourceApp};

#[tauri::command]
pub fn detect_frontmost_app<R: Runtime>(_app: tauri::AppHandle<R>) -> Option<SourceApp> {
    source_app::current()
}
```

**Step 2: Register the command in the plugin**

Modify `crates/snk-clipboard/src/plugin.rs` — extend the `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    crate::commands::paste_item,
    crate::commands::show_popup,
    crate::commands::detect_frontmost_app,
])
```

**Step 3: Update build.rs to register the permission**

Modify `crates/snk-clipboard/build.rs`:

```rust
const COMMANDS: &[&str] = &["paste_item", "show_popup", "detect_frontmost_app"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 4: Grant the permission in the default capability**

Modify `crates/snk-clipboard/permissions/default.toml`:

```toml
[default]
description = "Default permissions for snk-clipboard: allows paste, popup, and frontmost-app detection."
permissions = ["allow-paste-item", "allow-show-popup", "allow-detect-frontmost-app"]
```

**Step 5: Compile and verify**

Run: `cargo check -p snk-clipboard`
Expected: clean. The `tauri-plugin` build script regenerates the permission stubs under `permissions/autogenerated/`.

**Step 6: Commit**

```bash
git add crates/snk-clipboard/src/commands.rs crates/snk-clipboard/src/plugin.rs crates/snk-clipboard/build.rs crates/snk-clipboard/permissions/default.toml
git commit -m "feat(clipboard): detect_frontmost_app IPC command"
```

---

## Task 15: TS binding for `detect_frontmost_app` + blocklist types

**Files:**
- Modify: `packages/snk-clipboard/src/index.ts` (or wherever bindings live — check the file layout first)

**Step 1: Locate the existing binding file**

Run: `ls packages/snk-clipboard/src/`
Expected: there's an `index.ts` or `commands.ts` exporting `pasteItem` / `showPopup`. Use the same style for the new export.

**Step 2: Add the SourceApp type and the binding**

Append to the binding file:

```typescript
import { invoke } from '@tauri-apps/api/core';

export type SourceAppKind = 'macos_bundle_id' | 'windows_exe';

export interface SourceApp {
  identifier: string;
  display_name: string;
  kind: SourceAppKind;
}

export interface BlocklistEntry {
  identifier: string;
  display_name: string;
  kind: SourceAppKind;
}

export async function detectFrontmostApp(): Promise<SourceApp | null> {
  return invoke<SourceApp | null>('plugin:snk-clipboard|detect_frontmost_app');
}

export const APP_BLOCKLIST_SETTING_KEY = 'clipboard.app_blocklist';
```

**Step 3: Verify TS compiles + lints**

Run:

```bash
pnpm --filter @snk/clipboard typecheck
pnpm --filter @snk/clipboard lint
```

Expected: clean. If `tsconfig` doesn't include the file (new file in an existing package), nothing extra is required — `tsc -b` picks it up.

**Step 4: Commit**

```bash
git add packages/snk-clipboard/src/
git commit -m "feat(clipboard): TS binding for detectFrontmostApp + BlocklistEntry types"
```

---

## Task 16: Settings UI panel — list + remove

**Files:**
- Modify: `app/src/windows/settings/SettingsWindow.tsx` (add a new section)
- Create: `app/src/windows/settings/ClipboardSettings.tsx`

**Step 1: Create the panel skeleton**

Create `app/src/windows/settings/ClipboardSettings.tsx`:

```tsx
import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { getSetting, setSetting } from '@snk/library';
import {
  APP_BLOCKLIST_SETTING_KEY,
  detectFrontmostApp,
  type BlocklistEntry,
  type SourceApp,
} from '@snk/clipboard';

import { queryKeys } from '../../lib/queryKeys';

function readEntries(value: unknown): BlocklistEntry[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (e: any) =>
      typeof e?.identifier === 'string' &&
      typeof e?.display_name === 'string' &&
      (e.kind === 'macos_bundle_id' || e.kind === 'windows_exe'),
  );
}

export function ClipboardSettings() {
  const queryClient = useQueryClient();
  const { data: rawValue } = useQuery({
    queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    queryFn: () => getSetting(APP_BLOCKLIST_SETTING_KEY),
  });
  const entries = readEntries(rawValue);

  const [addOpen, setAddOpen] = useState(false);
  const [confirmFrontmost, setConfirmFrontmost] = useState<SourceApp | null>(null);

  async function persist(next: BlocklistEntry[]) {
    await setSetting(APP_BLOCKLIST_SETTING_KEY, next);
    await queryClient.invalidateQueries({
      queryKey: queryKeys.settings.one(APP_BLOCKLIST_SETTING_KEY),
    });
  }

  function remove(identifier: string, kind: BlocklistEntry['kind']) {
    void persist(
      entries.filter((e) => !(e.identifier === identifier && e.kind === kind)),
    );
  }

  async function addFromFrontmost() {
    const app = await detectFrontmostApp();
    if (app) setConfirmFrontmost(app);
  }

  return (
    <div>
      <h2 className="text-sm font-display uppercase tracking-wider text-fg-muted mb-2">
        Excluded apps
      </h2>
      <p className="text-[11px] text-fg-muted mb-3">
        Clipboard events from these apps are never recorded. OS-level
        "concealed" flags are always honored regardless of this list.
      </p>

      <ul className="border border-border rounded">
        {entries.length === 0 && (
          <li className="px-3 py-2 text-xs text-fg-muted">
            No exclusions configured.
          </li>
        )}
        {entries.map((e) => (
          <li
            key={`${e.kind}:${e.identifier}`}
            className="flex items-center justify-between px-3 py-2 border-b border-border last:border-0"
          >
            <div>
              <div className="text-sm text-fg">{e.display_name}</div>
              <div className="text-[10px] text-fg-muted">
                {e.identifier} · {e.kind}
              </div>
            </div>
            <button
              onClick={() => remove(e.identifier, e.kind)}
              className="text-fg-muted hover:text-danger text-xs"
              aria-label={`Remove ${e.display_name}`}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      <div className="flex gap-2 mt-3">
        <button
          onClick={() => setAddOpen(true)}
          className="text-xs text-fg hover:text-primary"
        >
          + Add app…
        </button>
        <button
          onClick={addFromFrontmost}
          className="text-xs text-fg hover:text-primary"
        >
          + Add from frontmost app
        </button>
      </div>

      {addOpen && (
        <AddAppModal
          existing={entries}
          onClose={() => setAddOpen(false)}
          onAdd={(entry) => {
            void persist([...entries, entry]);
            setAddOpen(false);
          }}
        />
      )}
      {confirmFrontmost && (
        <ConfirmFrontmostModal
          app={confirmFrontmost}
          existing={entries}
          onClose={() => setConfirmFrontmost(null)}
          onConfirm={(entry) => {
            void persist([...entries, entry]);
            setConfirmFrontmost(null);
          }}
        />
      )}
    </div>
  );
}

interface AddAppModalProps {
  existing: BlocklistEntry[];
  onClose: () => void;
  onAdd: (entry: BlocklistEntry) => void;
}

function AddAppModal({ existing, onClose, onAdd }: AddAppModalProps) {
  const [identifier, setIdentifier] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [kind, setKind] = useState<BlocklistEntry['kind']>('macos_bundle_id');
  const [error, setError] = useState<string | null>(null);

  function submit() {
    const id = identifier.trim();
    if (!id) {
      setError('Identifier is required.');
      return;
    }
    const dup = existing.find((e) => e.identifier === id && e.kind === kind);
    if (dup) {
      setError('Already in the list.');
      return;
    }
    onAdd({
      identifier: id,
      display_name: displayName.trim() || id,
      kind,
    });
  }

  return (
    <div
      className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-bg border-2 border-border rounded p-4 w-80"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-sm font-display uppercase mb-3">Add excluded app</h3>
        <label className="block text-[10px] text-fg-muted mb-1">Kind</label>
        <select
          value={kind}
          onChange={(e) => setKind(e.target.value as BlocklistEntry['kind'])}
          className="w-full text-xs bg-surface border border-border rounded p-1 mb-2"
        >
          <option value="macos_bundle_id">macOS bundle ID</option>
          <option value="windows_exe">Windows exe</option>
        </select>
        <label className="block text-[10px] text-fg-muted mb-1">Identifier</label>
        <input
          value={identifier}
          onChange={(e) => setIdentifier(e.target.value)}
          placeholder={
            kind === 'macos_bundle_id'
              ? 'com.example.app'
              : 'example.exe'
          }
          className="w-full text-xs bg-surface border border-border rounded p-1 mb-2"
        />
        <label className="block text-[10px] text-fg-muted mb-1">
          Display name (optional)
        </label>
        <input
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          className="w-full text-xs bg-surface border border-border rounded p-1 mb-3"
        />
        {error && <div className="text-[10px] text-danger mb-2">{error}</div>}
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="text-xs text-fg-muted">
            Cancel
          </button>
          <button
            onClick={submit}
            className="text-xs text-bg bg-primary px-2 py-1 rounded"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}

interface ConfirmFrontmostModalProps {
  app: SourceApp;
  existing: BlocklistEntry[];
  onClose: () => void;
  onConfirm: (entry: BlocklistEntry) => void;
}

function ConfirmFrontmostModal({
  app,
  existing,
  onClose,
  onConfirm,
}: ConfirmFrontmostModalProps) {
  const dup = existing.find(
    (e) => e.identifier === app.identifier && e.kind === app.kind,
  );
  return (
    <div
      className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-bg border-2 border-border rounded p-4 w-80"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-sm font-display uppercase mb-3">
          Block frontmost app?
        </h3>
        <div className="text-sm text-fg mb-1">{app.display_name}</div>
        <div className="text-[10px] text-fg-muted mb-3">
          {app.identifier} · {app.kind}
        </div>
        {dup && (
          <div className="text-[10px] text-danger mb-2">
            This app is already in the list.
          </div>
        )}
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="text-xs text-fg-muted">
            Cancel
          </button>
          <button
            disabled={!!dup}
            onClick={() =>
              onConfirm({
                identifier: app.identifier,
                display_name: app.display_name,
                kind: app.kind,
              })
            }
            className="text-xs text-bg bg-primary px-2 py-1 rounded disabled:opacity-50"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
```

**Step 2: Mount the panel inside SettingsWindow**

Modify `app/src/windows/settings/SettingsWindow.tsx`. Find a sensible section break (probably between the autostart settings and theme settings — check the existing layout first) and add:

```tsx
import { ClipboardSettings } from './ClipboardSettings';
// ... inside the SettingsWindow function's JSX, in the panel/section list:
<section className="border-t border-border pt-4 mt-4">
  <ClipboardSettings />
</section>
```

**Step 3: Verify TS + lint**

Run:

```bash
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app lint
```

Expected: clean.

**Step 4: Commit**

```bash
git add app/src/windows/settings/ClipboardSettings.tsx app/src/windows/settings/SettingsWindow.tsx
git commit -m "feat(settings): clipboard exclusion list panel"
```

---

## Task 17: Settings UI tests

**Files:**
- Create: `app/src/windows/settings/ClipboardSettings.test.tsx`

**Step 1: Write the tests**

Create `app/src/windows/settings/ClipboardSettings.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';

import { ClipboardSettings } from './ClipboardSettings';
import { renderWithQuery } from '../../test/renderWithQuery';

const mockedInvoke = vi.mocked(invoke);

describe('<ClipboardSettings />', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it('renders empty state when setting is unset', async () => {
    mockedInvoke.mockResolvedValueOnce(null); // get_setting returns null
    renderWithQuery(<ClipboardSettings />);
    expect(await screen.findByText(/no exclusions configured/i)).toBeInTheDocument();
  });

  it('renders entries from the setting value', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'com.1password.1password8', display_name: '1Password 8', kind: 'macos_bundle_id' },
      { identifier: 'KeePassXC.exe', display_name: 'KeePassXC', kind: 'windows_exe' },
    ]);
    renderWithQuery(<ClipboardSettings />);
    expect(await screen.findByText('1Password 8')).toBeInTheDocument();
    expect(screen.getByText('KeePassXC')).toBeInTheDocument();
  });

  it('Add app modal submits a new entry via set_setting', async () => {
    mockedInvoke.mockResolvedValueOnce([]); // initial get_setting
    renderWithQuery(<ClipboardSettings />);

    fireEvent.click(await screen.findByText(/add app/i));
    fireEvent.change(screen.getByPlaceholderText(/com.example.app/i), {
      target: { value: 'com.bitwarden.desktop' },
    });
    mockedInvoke.mockResolvedValueOnce(undefined); // set_setting
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'com.bitwarden.desktop', display_name: 'com.bitwarden.desktop', kind: 'macos_bundle_id' },
    ]); // refetch
    fireEvent.click(screen.getByText(/^add$/i));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'clipboard.app_blocklist',
        value: [
          {
            identifier: 'com.bitwarden.desktop',
            display_name: 'com.bitwarden.desktop',
            kind: 'macos_bundle_id',
          },
        ],
      });
    });
  });

  it('add-from-frontmost calls detect_frontmost_app and prefills the confirmation modal', async () => {
    mockedInvoke.mockResolvedValueOnce([]); // get_setting
    renderWithQuery(<ClipboardSettings />);

    mockedInvoke.mockResolvedValueOnce({
      identifier: 'com.1password.1password8',
      display_name: '1Password 8',
      kind: 'macos_bundle_id',
    });
    fireEvent.click(await screen.findByText(/add from frontmost/i));

    expect(await screen.findByText(/block frontmost app/i)).toBeInTheDocument();
    expect(screen.getByText('1Password 8')).toBeInTheDocument();
  });

  it('remove button persists the updated list', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'foo.exe', display_name: 'Foo', kind: 'windows_exe' },
      { identifier: 'bar.exe', display_name: 'Bar', kind: 'windows_exe' },
    ]);
    renderWithQuery(<ClipboardSettings />);

    fireEvent.click(await screen.findByLabelText(/remove foo/i));
    mockedInvoke.mockResolvedValueOnce(undefined); // set_setting
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'bar.exe', display_name: 'Bar', kind: 'windows_exe' },
    ]);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
        key: 'clipboard.app_blocklist',
        value: [{ identifier: 'bar.exe', display_name: 'Bar', kind: 'windows_exe' }],
      });
    });
  });

  it('duplicate identifier blocks add with inline error', async () => {
    mockedInvoke.mockResolvedValueOnce([
      { identifier: 'foo.exe', display_name: 'Foo', kind: 'windows_exe' },
    ]);
    renderWithQuery(<ClipboardSettings />);
    fireEvent.click(await screen.findByText(/add app/i));

    fireEvent.change(screen.getByDisplayValue('macos_bundle_id'), {
      target: { value: 'windows_exe' },
    });
    fireEvent.change(screen.getByPlaceholderText(/example.exe/i), {
      target: { value: 'foo.exe' },
    });
    fireEvent.click(screen.getByText(/^add$/i));

    expect(await screen.findByText(/already in the list/i)).toBeInTheDocument();
  });
});
```

**Step 2: Run the tests**

Run: `pnpm --filter @snk/app exec vitest run ClipboardSettings`
Expected: 6/6 pass.

**Step 3: Commit**

```bash
git add app/src/windows/settings/ClipboardSettings.test.tsx
git commit -m "test(settings): ClipboardSettings panel + modal interactions"
```

---

## Task 18: Real-clipboard integration test (Windows + macOS)

**Files:**
- Create: `crates/snk-clipboard/tests/sensitivity_integration.rs`

**Step 1: Add serial_test if not already present**

Modify `crates/snk-clipboard/Cargo.toml`:

```toml
[dev-dependencies]
serde_json = { workspace = true }
tempfile = "3"
serial_test = "3"
```

**Step 2: Write the test**

Create `crates/snk-clipboard/tests/sensitivity_integration.rs`:

```rust
//! Integration test — manipulates the real OS clipboard. Runs serial
//! against any other env-mutating tests via `serial_test::serial(clipboard)`.

#[cfg(target_os = "macos")]
#[test]
#[serial_test::serial(clipboard)]
fn macos_concealed_type_marks_sensitive() {
    use objc2::rc::Retained;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::{NSArray, NSString};

    unsafe {
        let pasteboard: Retained<NSPasteboard> = NSPasteboard::generalPasteboard();
        let _ = pasteboard.clearContents();

        let value = NSString::from_str("secret");
        let concealed_type = NSString::from_str("org.nspasteboard.ConcealedType");
        let types = NSArray::from_slice(&[concealed_type.as_ref()]);
        let _ = pasteboard.declareTypes_owner(&types, None);
        let _ = pasteboard.setString_forType(&value, &concealed_type);
    }

    assert!(
        snk_clipboard::sensitivity::is_sensitive(),
        "concealed type should be reported as sensitive"
    );
}

#[cfg(target_os = "macos")]
#[test]
#[serial_test::serial(clipboard)]
fn macos_plain_text_is_not_sensitive() {
    use objc2::rc::Retained;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::NSString;

    unsafe {
        let pasteboard: Retained<NSPasteboard> = NSPasteboard::generalPasteboard();
        let _ = pasteboard.clearContents();
        let value = NSString::from_str("hello world");
        let _ = pasteboard.setString_forType(
            &value,
            &NSString::from_str("public.utf8-plain-text"),
        );
    }

    assert!(!snk_clipboard::sensitivity::is_sensitive());
}

#[cfg(target_os = "windows")]
#[test]
#[serial_test::serial(clipboard)]
fn windows_can_include_in_history_zero_marks_sensitive() {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
        SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };

    unsafe {
        let _ = OpenClipboard(None);
        let _ = EmptyClipboard();

        // Write a dummy text payload first.
        let text = "hello\0";
        let bytes = text.as_bytes();
        let handle =
            GlobalAlloc(GMEM_MOVEABLE, bytes.len()).expect("GlobalAlloc text");
        let ptr = GlobalLock(handle);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        let _ = GlobalUnlock(handle);
        let _ = SetClipboardData(1u32 /* CF_TEXT */, Some(HANDLE(handle.0)));

        // Set the CanIncludeInClipboardHistory format to 0 (a DWORD).
        let fmt = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
        let h = GlobalAlloc(GMEM_MOVEABLE, 4).expect("GlobalAlloc dword");
        let p = GlobalLock(h);
        *(p as *mut u32) = 0;
        let _ = GlobalUnlock(h);
        let _ = SetClipboardData(fmt, Some(HANDLE(h.0)));

        let _ = CloseClipboard();
    }

    assert!(snk_clipboard::sensitivity::is_sensitive());
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[test]
fn linux_test_skipped() {
    eprintln!("SKIP: sensitivity integration runs on macOS + Windows only");
}
```

> **NOTE:** these tests modify the real OS clipboard. Don't run them while you're using your machine — they overwrite whatever you've copied. CI is fine.

**Step 3: Run on the local platform**

- macOS: `cargo test -p snk-clipboard --test sensitivity_integration -- --include-ignored`
- Windows: same.

Expected: both per-OS tests pass on their respective platforms. Linux prints `SKIP:`.

**Step 4: Commit**

```bash
git add crates/snk-clipboard/tests/sensitivity_integration.rs crates/snk-clipboard/Cargo.toml Cargo.lock
git commit -m "test(clipboard): real-OS sensitivity integration tests (macOS + Windows)"
```

---

## Task 19: Reconcile design spec mentions

**Files:**
- Modify: `docs/superpowers/specs/2026-05-20-snapper-keeper-design.md`

**Step 1: Re-confirm the existing claims match the new impl**

Run:

```bash
grep -n -i "sensitive\|conceal\|app_blocklist" docs/superpowers/specs/2026-05-20-snapper-keeper-design.md
```

The existing §8.4 already documents the right behavior (drop at watcher). Three light edits to close the loop:

**Step 2: Reference the new design doc + impl**

Find §8.4 ("Sensitive-clipboard detection") and append after the existing content:

```markdown
**Implementation status (post-2026-05-24):** wired in [`docs/superpowers/specs/2026-05-24-sensitive-clipboard-design.md`](2026-05-24-sensitive-clipboard-design.md). The schema column originally proposed (`sensitive INTEGER`) was dropped in V005 because content is filtered at the watcher and the column never becomes load-bearing.
```

Find §4.4 (the ER-diagram or wherever `sensitive` shows up in the schema mermaid) and remove the `int sensitive "1 if app flagged"` line — that column no longer exists post-V005.

**Step 3: Confirm PRIVACY.md doesn't need touching**

Run: `grep -i "sensitive\|conceal" PRIVACY.md`
Expected: no matches. (The issue body wrongly claimed it did.) If it does match, edit accordingly.

**Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-05-20-snapper-keeper-design.md
git commit -m "docs(spec): cross-link sensitive-clipboard impl + drop schema column from ER diagram"
```

---

## Task 20: Final smoke check

**Files:** none (validation only)

**Step 1: Run the whole test suite**

```bash
cargo test --workspace --exclude snapper-keeper-app
pnpm --filter @snk/app exec vitest run
pnpm lint
pnpm typecheck
```

Expected: all green.

**Step 2: Manual local exercise**

Per the project's manual-test list:

1. Run `pnpm --filter @snk/app tauri dev` on Windows or macOS.
2. Copy text from a regular app (e.g. Notepad / Notes). Confirm it appears in the Ctrl/Cmd+Shift+V popup.
3. Open 1Password (or any password manager that sets the OS flag), reveal a password, click the "copy" button.
4. Open the snapper-keeper popup. Confirm the password does **not** appear.
5. Settings → Clipboard → click "Add from frontmost app" while focused on (say) VS Code; confirm the modal shows `code.exe` (Windows) or `com.microsoft.VSCode` (macOS); confirm add.
6. Copy text from VS Code; confirm it's skipped.
7. Remove the entry; copy text from VS Code; confirm it's saved.

**Step 3: Open the PR**

After all commits push cleanly to a feature branch, open the PR referencing `Closes #22` and noting #38 and #57 as incidentally improved (not closed — they have other scope).

---

## Spec coverage map

| Spec section | Covered by tasks |
|---|---|
| §2 Goals — OS flag honoring | 6 (macOS), 8 (Windows), 11 (unit), 18 (integration) |
| §2 Goals — user-managed blocklist | 4 (logic), 14–17 (UI + IPC), 11 (watcher integration) |
| §2 Goals — V005 migration | 2 |
| §2 Goals — source_app capture | 7 (macOS), 9 (Windows), 10 (worker_step wires it in) |
| §2 Goals — Windows event-driven watcher | 13 |
| §3.1 Module layout | 3, 4, 5, 6, 8, 9, 12, 13 |
| §3.2 Per-event decision flow | 10, 11 |
| §3.3 Windows event mechanism | 13 (+ fallback to polling) |
| §3.4 macOS polling at 100 ms | 12 |
| §4.1–4.4 Component contracts | 3, 4, 5, 6–9, 10 |
| §4.5 Settings UI | 16, 17 |
| §5.1 V005 migration | 2 |
| §5.2 Setting JSON shape | 4 (parsing), 15 (TS types), 16 (UI) |
| §5.3 `source_app` column populated | 10 (worker_step uses it on insert) |
| §6 Error handling matrix | 4 (fail-open blocklist), 11 (worker_step result types), 13 (fallback) |
| §7.1 Unit tests | 3, 4, 5, 11 |
| §7.2 Integration tests | 18 |
| §7.3 Migration tests | 2 |
| §7.4 UI tests | 17 |
| Spec §8.4 cross-link | 19 |

---

**Plan complete and saved to `docs/superpowers/plans/2026-05-24-sensitive-clipboard.md`. Three execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Team-Driven (this session, experimental)** - Multiple persistent agents work in parallel with direct inter-agent communication; best when tasks need coordination. Requires Opus 4.6+ and `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`. Costs 2–4x more.

**3. Parallel Session (separate)** - Open new session with executing-plans, batch execution with human checkpoints

**Which approach?**
