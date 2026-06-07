# Opt-in Encryption-at-Rest (SQLCipher) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an off-by-default Settings → Privacy → "Encrypt library" option that encrypts the local SQLite DB with SQLCipher, using a random 32-byte key stored in the OS keychain.

**Architecture:** Split key *mechanics* (cross-platform, key-passed-explicitly, CI-testable on Linux) from key *storage* (OS keychain, win/mac only, dependency-injected). `Db::open` probes the DB file header to decide plaintext vs encrypted, loads the key from the keychain only when encrypted, and runs `PRAGMA key` first on every open path. Enabling encryption is a one-way export-and-swap followed by an app restart.

**Tech Stack:** Rust, `rusqlite` (`bundled-sqlcipher-vendored-openssl`), `keyring` v3, `getrandom`, Tauri 2, React/TS + vitest.

**Source design:** `docs/superpowers/specs/2026-06-07-encryption-at-rest-design.md`

**Conventions (from CLAUDE.md):**
- One task = one commit. Commit messages below are **exact strings** — use verbatim.
- Stage files **explicitly** (`git add <path>`), never `git add .` / `-A`.
- serde discriminator tag is `"kind"` → no variant field may be named `kind`; use `reason`/`detail`.
- New Tauri commands require the **3-file ACL dance**: `invoke_handler!` (plugin.rs) + `COMMANDS` (build.rs) + `permissions/default.toml`.
- Generated TS (`packages/**/generated/*.ts`) is pinned to **LF** via `.gitattributes` — do not let Windows rewrite it to CRLF.
- Files >500 lines are a red flag — keep new modules focused.
- Every commit message ends with the `Co-Authored-By` trailer (shown in each commit step).

---

## File Structure

| File | Responsibility |
|---|---|
| `scripts/build-local.sh` | (modify) Force a real Perl on Windows so the vendored-OpenSSL build works under Git Bash. |
| `Cargo.toml` (workspace) | (modify) Flip `rusqlite` to `bundled-sqlcipher-vendored-openssl`. |
| `crates/snk-library/Cargo.toml` | (modify) Add `getrandom`; add target-gated `keyring` (win/mac). |
| `crates/snk-library/src/error.rs` | (modify) Add `Locked`, `Keyring`, `Encrypt`, `EncryptionUnsupported`. |
| `crates/snk-library/src/db.rs` | (modify) `is_encrypted()` header-probe; `configure_conn()`; `open_with_key()`; key-aware `Db::open`. |
| `crates/snk-library/src/keystore.rs` | (new) OS-gated key gen/store/load/delete over `keyring` v3; stub elsewhere. |
| `crates/snk-library/src/encrypt.rs` | (new) `encrypt_in_place()` mechanic: backup → export → store-key (injected) → swap → purge. |
| `crates/snk-library/src/commands.rs` | (modify) `enable_encryption`, `encryption_status` commands. |
| `crates/snk-library/src/authz.rs` | (modify) `ENABLE_ENCRYPTION_WINDOWS`. |
| `crates/snk-library/src/plugin.rs` | (modify) Register commands; handle `Locked` in setup. |
| `crates/snk-library/build.rs` | (modify) Add the two commands to `COMMANDS`. |
| `crates/snk-library/permissions/default.toml` | (modify) Allow the two commands. |
| `crates/snk-library/src/lib.rs` | (modify) `pub mod keystore; pub mod encrypt;`. |
| `packages/snk-library/src/index.ts` | (modify) `enableEncryption()`, `encryptionStatus()` bindings. |
| `app/src/windows/settings/PrivacySettings.tsx` | (new) Privacy section + warn modal + restart-on-success. |
| `app/src/windows/settings/PrivacySettings.test.tsx` | (new) Render both states. |
| `app/src/windows/settings/SettingsWindow.tsx` | (modify) Mount `<PrivacySettings/>`. |
| `app/src/windows/settings/AboutSection.tsx` | (modify) "Encryption is on…" note when enabled. |
| `app/src-tauri/tauri.conf.json` | (modify) `ITSAppUsesNonExemptEncryption`. |
| `PRIVACY.md` | (modify) At-rest wording; image files remain plaintext. |
| `README.md` | (modify) Strawberry Perl dev prerequisite (Windows). |

**Task dependency note:** Task 1 (Perl fix) must land **before** Task 2 (rusqlite flip) so CI's `e2e-process-smoke (windows)` job keeps building. Tasks 3–6 (error, probe, open, encrypt, keystore) are the Rust core. Task 7 wires commands. Tasks 8–11 are frontend + docs.

---

## Task 1: Force a real Perl on Windows in build-local.sh (prerequisite)

The SQLCipher build (Task 2) pulls in vendored OpenSSL, whose `Configure` runs under Perl. Git Bash puts msys Perl (`/usr/bin/perl`) first, which lacks `Locale::Maketext::Simple` and breaks the build. This must land first so flipping the dep doesn't break `e2e-process-smoke (windows)` (that job runs `build-local.sh`). GitHub `windows-latest` runners ship Strawberry Perl at `C:\Strawberry`.

**Files:**
- Modify: `scripts/build-local.sh:46-48` (inside the Windows `MINGW*|MSYS*|CYGWIN*` branch, after `TARGET`/`BUNDLES` are set)

- [ ] **Step 1: Add the Perl-forcing block**

In `scripts/build-local.sh`, locate the Windows case arm:

```bash
    TARGET="x86_64-pc-windows-msvc"
    BUNDLES="nsis"
    ;;
```

Insert, immediately before the `;;`:

```bash
    TARGET="x86_64-pc-windows-msvc"
    BUNDLES="nsis"
    # rusqlite's SQLCipher feature builds vendored OpenSSL, whose Configure
    # runs under Perl. Git Bash puts msys Perl (/usr/bin/perl) first, which
    # lacks Locale::Maketext::Simple and aborts the build. Force Strawberry
    # Perl (pre-installed on GitHub windows runners at C:\Strawberry) to the
    # front of PATH.
    STRAWBERRY_PERL="/c/Strawberry/perl/bin"
    if [[ -x "$STRAWBERRY_PERL/perl.exe" ]]; then
      export PATH="$STRAWBERRY_PERL:$PATH"
      echo "build-local: using Strawberry Perl at $STRAWBERRY_PERL"
    else
      echo "build-local: Strawberry Perl not found at C:\\Strawberry — the SQLCipher/OpenSSL build needs a full Perl." >&2
      echo "build-local: install it with: winget install --id StrawberryPerl.StrawberryPerl" >&2
      exit 1
    fi
    ;;
```

- [ ] **Step 2: Verify the script still parses**

Run: `bash -n scripts/build-local.sh`
Expected: no output, exit 0 (syntactic check; a full build needs an interactive Windows desktop and is verified by CI's `e2e-process-smoke`).

- [ ] **Step 3: Commit**

```bash
git add scripts/build-local.sh
git commit -m "build(local): force Strawberry Perl on Windows for the vendored-OpenSSL build

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: Flip rusqlite to SQLCipher + add keyring/getrandom deps

`bundled-sqlcipher-vendored-openssl` replaces `bundled`. It is always compiled in (runtime opt-in requires the lib present); a SQLCipher build opens plaintext DBs natively, so off-by-default users are unaffected (spike-proven). `keyring` is target-gated so Linux CI never compiles it.

**Files:**
- Modify: `Cargo.toml:40`
- Modify: `crates/snk-library/Cargo.toml:10-22` (dependencies) and append a `[target.…]` section

- [ ] **Step 1: Flip the workspace rusqlite feature**

In `Cargo.toml`, change line 40 from:

```toml
rusqlite = { version = "0.31", features = ["bundled", "uuid"] }
```

to:

```toml
rusqlite = { version = "0.31", features = ["bundled-sqlcipher-vendored-openssl", "uuid"] }
```

- [ ] **Step 2: Add getrandom + target-gated keyring to snk-library**

In `crates/snk-library/Cargo.toml`, add to `[dependencies]` (after `ts-rs.workspace = true`):

```toml
getrandom = "0.2"
```

Then append a new section at the end of the file:

```toml
# keyring backs OS credential-store access for the encryption key. Target-gated
# to the shipping platforms so Linux CI (rust-test/coverage) never compiles a
# no-backend mock; matches the cfg-gate on src/keystore.rs.
[target.'cfg(any(windows, target_os = "macos"))'.dependencies]
keyring = { version = "3", features = ["windows-native", "apple-native"] }
```

- [ ] **Step 3: Verify the workspace still builds and tests pass**

Run: `cargo build -p snk-library`
Expected: compiles (this builds OpenSSL + SQLCipher; first build is slow). On Windows run from a shell where Strawberry Perl is on PATH (PowerShell finds it on the system PATH; Git Bash does not — see Task 1).

Run: `cargo test -p snk-library`
Expected: all existing tests PASS (plaintext DBs still open on a SQLCipher build).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crates/snk-library/Cargo.toml
git commit -m "build(library): switch rusqlite to SQLCipher; add keyring + getrandom

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: Add encryption error variants

**Files:**
- Modify: `crates/snk-library/src/error.rs:13-39` (enum) and `:95-206` (tests)
- Regenerates: `packages/snk-library/src/generated/errors.ts` (via ts-rs on test run)

- [ ] **Step 1: Write the failing test**

In `crates/snk-library/src/error.rs`, add to `mod tests`:

```rust
    #[test]
    fn encryption_variants_use_kind_discriminator() {
        let locked = LibraryError::Locked {
            detail: "no key".into(),
        };
        assert!(serde_json::to_string(&locked)
            .unwrap()
            .contains("\"kind\":\"locked\""));
        assert!(locked.to_string().contains("locked"));

        let keyring = LibraryError::Keyring {
            reason: "store unavailable".into(),
        };
        assert!(serde_json::to_string(&keyring)
            .unwrap()
            .contains("\"kind\":\"keyring\""));

        let encrypt = LibraryError::Encrypt {
            reason: "export failed".into(),
        };
        assert!(serde_json::to_string(&encrypt)
            .unwrap()
            .contains("\"kind\":\"encrypt\""));

        let unsupported = LibraryError::EncryptionUnsupported {
            reason: "no keychain on this platform".into(),
        };
        assert!(serde_json::to_string(&unsupported)
            .unwrap()
            .contains("\"kind\":\"encryption-unsupported\""));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library encryption_variants_use_kind_discriminator`
Expected: FAIL — `no variant named Locked` (compile error).

- [ ] **Step 3: Add the variants**

In `crates/snk-library/src/error.rs`, add to the `LibraryError` enum (after the `Unauthorized` variant, before the closing `}`):

```rust
    #[error("library is locked: {detail}")]
    Locked { detail: String },

    #[error("keychain error: {reason}")]
    Keyring { reason: String },

    #[error("encryption failed: {reason}")]
    Encrypt { reason: String },

    #[error("encryption is not supported on this platform: {reason}")]
    EncryptionUnsupported { reason: String },
```

- [ ] **Step 4: Run test, then regenerate the TS bindings**

Run: `cargo test -p snk-library encryption_variants_use_kind_discriminator`
Expected: PASS.

ts-rs export tests are `#[ignore]`d, so a normal `cargo test` does NOT rewrite `errors.ts`. Regenerate it explicitly (this is what CI's `verify-ts-bindings.sh` runs):

Run: `cargo test -p snk-library export_bindings -- --include-ignored`
Expected: PASS.

Run: `git status --short packages/snk-library/src/generated/errors.ts`
Expected: the file shows as modified (re-exported with the new variants). Confirm it still has LF endings — on Windows, `git diff` should show no whole-file CRLF rewrite; `.gitattributes` pins `packages/**/generated/*.ts` to LF.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-library/src/error.rs packages/snk-library/src/generated/errors.ts
git commit -m "feat(library): add Locked/Keyring/Encrypt/EncryptionUnsupported errors

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: Header-probe (`is_encrypted`)

A plaintext SQLite file begins with the 16 bytes `"SQLite format 3\0"`; a SQLCipher file does not. Empty/missing → treat as plaintext-new.

**Files:**
- Modify: `crates/snk-library/src/db.rs` (add free function + tests)

- [ ] **Step 1: Write the failing test**

In `crates/snk-library/src/db.rs`, add to `mod tests`:

```rust
    #[test]
    fn is_encrypted_distinguishes_plaintext_empty_and_encrypted() {
        let dir = tempfile::tempdir().unwrap();

        // Missing file → not encrypted.
        let missing = dir.path().join("missing.db");
        assert!(!is_encrypted(&missing).unwrap());

        // A real plaintext DB → not encrypted.
        let plain = dir.path().join("plain.db");
        Db::open(&plain).unwrap();
        assert!(!is_encrypted(&plain).unwrap());

        // A file that does NOT start with the SQLite magic → encrypted.
        let enc = dir.path().join("enc.db");
        std::fs::write(&enc, b"\x01\x02\x03\x04not-a-sqlite-header-bytes").unwrap();
        assert!(is_encrypted(&enc).unwrap());

        // Empty file → not encrypted.
        let empty = dir.path().join("empty.db");
        std::fs::write(&empty, b"").unwrap();
        assert!(!is_encrypted(&empty).unwrap());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library is_encrypted_distinguishes`
Expected: FAIL — `cannot find function is_encrypted`.

- [ ] **Step 3: Implement `is_encrypted`**

In `crates/snk-library/src/db.rs`, add after the `use` lines (before `pub struct Db`):

```rust
/// The 16-byte magic every plaintext SQLite database file starts with.
/// A SQLCipher-encrypted database has no plaintext header.
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Decide whether `path` holds an encrypted (SQLCipher) database by inspecting
/// its header. A missing or empty file is treated as not-encrypted (a fresh
/// plaintext DB will be created). The DB file is the single source of truth for
/// encryption state — nothing is persisted in the `settings` table (which would
/// be unreadable when encrypted).
pub fn is_encrypted(path: &Path) -> Result<bool> {
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(crate::LibraryError::io(path, e)),
    };
    let mut header = [0u8; 16];
    let n = f.read(&mut header).map_err(|e| crate::LibraryError::io(path, e))?;
    if n == 0 {
        return Ok(false); // empty file
    }
    Ok(&header[..] != &SQLITE_MAGIC[..])
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p snk-library is_encrypted_distinguishes`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-library/src/db.rs
git commit -m "feat(library): add is_encrypted header-probe for DB files

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: `configure_conn` + `open_with_key` (key-aware open mechanic)

Extract the per-connection pragmas into one helper that runs `PRAGMA key` **first**, then add a cross-platform `open_with_key(path, key)` that all open paths build on. This is the testable mechanic — the key is passed explicitly, so it runs on Linux CI.

**Files:**
- Modify: `crates/snk-library/src/db.rs` (refactor open paths; add `open_with_key`; add tests)

- [ ] **Step 1: Write the failing test**

In `crates/snk-library/src/db.rs`, add to `mod tests`:

```rust
    const TEST_KEY: [u8; 32] = [7u8; 32];

    #[test]
    fn open_with_key_round_trips_and_rejects_wrong_access() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("enc.db");

        // Create an encrypted DB and write a row via a tag.
        {
            let db = Db::open_with_key(&path, Some(&TEST_KEY)).expect("keyed open");
            crate::tags::create(&db, "secret", "#fff").expect("insert");
        }

        // The file must now be encrypted on disk.
        assert!(is_encrypted(&path).unwrap(), "DB should be encrypted at rest");

        // Reopening with the same key reads the row back.
        {
            let db = Db::open_with_key(&path, Some(&TEST_KEY)).expect("reopen keyed");
            let tags = crate::tags::list(&db).expect("list");
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].name, "secret");
        }

        // Opening an encrypted DB as plaintext (no key) must fail.
        assert!(
            Db::open_with_key(&path, None).is_err(),
            "encrypted DB must not open without a key"
        );
    }
```

(Adjust `tags[0].name` if `Tag`'s field differs — see `crates/snk-library/src/tags.rs`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library open_with_key_round_trips`
Expected: FAIL — `no function open_with_key`.

- [ ] **Step 3: Add `configure_conn` and `open_with_key`, route existing paths through them**

In `crates/snk-library/src/db.rs`, add this helper (e.g. just above `impl Db`):

```rust
/// Apply per-connection pragmas. When `key` is `Some`, `PRAGMA key` MUST run
/// first — SQLCipher requires the key before any other statement touches the
/// database. The 64-hex raw-key form makes SQLCipher use the bytes directly as
/// the AES key (no PBKDF2 over a passphrase).
fn configure_conn(conn: &Connection, key: Option<&[u8; 32]>) -> Result<()> {
    if let Some(k) = key {
        let hex: String = k.iter().map(|b| format!("{b:02x}")).collect();
        conn.pragma_update(None, "key", format!("x'{hex}'"))?;
    }
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}
```

Add the keyed open constructor inside `impl Db` (after `open_no_migrate`):

```rust
    /// Open at `path` with an explicit optional key, run migrations, and verify
    /// the database is readable. This is the cross-platform encryption mechanic:
    /// the key is passed in, not fetched from the OS keychain, so it is fully
    /// unit-testable without a credential store.
    pub fn open_with_key(path: &Path, key: Option<&[u8; 32]>) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::LibraryError::io(parent, e))?;
        }
        let mut conn = Connection::open(path)?;
        configure_conn(&conn, key)?;
        // Fail fast if the key is wrong/absent for an encrypted DB: this read
        // errors before we attempt migrations.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))?;

        match crate::migrate::migrate(&mut conn, Some(path)) {
            Ok(()) => Ok(Self {
                conn: Mutex::new(conn),
            }),
            Err(crate::LibraryError::Migration {
                backup_path: Some(backup),
                from,
                to,
                detail,
                ..
            }) => Err(restore_after_migration_failure(
                conn, path, backup, from, to, detail,
            )),
            Err(e) => Err(e),
        }
    }
```

Now replace the three inlined pragma triples with `configure_conn` calls:

- In `open` (around `db.rs:30-32`), replace:
  ```rust
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
  ```
  with:
  ```rust
        configure_conn(&conn, None)?;
  ```
  (Task 6 changes `open` further to load the key; for now keep `None`.)

- In `open_no_migrate` (around `db.rs:62-64`) and `open_with_custom_migrations` (around `db.rs:81-83`), make the same replacement (`configure_conn(&conn, None)?;`). These paths remain plaintext-only by contract.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p snk-library open_with_key_round_trips`
Expected: PASS.

Run: `cargo test -p snk-library`
Expected: all existing db.rs tests still PASS (refactor is behavior-preserving for plaintext).

- [ ] **Step 5: Commit**

```bash
git add crates/snk-library/src/db.rs
git commit -m "feat(library): add open_with_key + configure_conn key-aware open mechanic

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: keystore module (OS-gated key storage) + key-aware `Db::open`

`keystore` is the only OS-specific surface. On win/mac it wraps `keyring` v3; elsewhere it is a stub returning `EncryptionUnsupported` so the crate compiles and `Db::open` orchestration links on Linux. `Db::open` now probes the header and, when encrypted, loads the key from the keychain.

**Files:**
- Create: `crates/snk-library/src/keystore.rs`
- Modify: `crates/snk-library/src/lib.rs` (add `pub mod keystore;`)
- Modify: `crates/snk-library/src/db.rs` (`Db::open` orchestration + cfg-gated test)

- [ ] **Step 1: Create keystore.rs with real impl + stub**

Create `crates/snk-library/src/keystore.rs`:

```rust
//! OS credential-store access for the library encryption key.
//!
//! This is the only OS-specific surface of the encryption feature. On Windows
//! and macOS it wraps `keyring` v3 (Windows Credential Manager / macOS
//! Keychain). On every other platform it is a stub that reports
//! `EncryptionUnsupported`, so the crate still compiles and links on Linux CI —
//! where the encryption code paths are never reached (header-probe sees only
//! plaintext DBs in tests).

use crate::{LibraryError, Result};

#[cfg(any(windows, target_os = "macos"))]
const SERVICE: &str = "snapper-keeper";
#[cfg(any(windows, target_os = "macos"))]
const ACCOUNT: &str = "library-db-key";

/// Generate a fresh random 32-byte key.
pub fn generate_key() -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key)
        .map_err(|e| LibraryError::Encrypt { reason: format!("rng failure: {e}") })?;
    Ok(key)
}

#[cfg(any(windows, target_os = "macos"))]
fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT)
        .map_err(|e| LibraryError::Keyring { reason: e.to_string() })
}

/// Store the key in the OS credential store, replacing any existing entry.
#[cfg(any(windows, target_os = "macos"))]
pub fn store_key(key: &[u8; 32]) -> Result<()> {
    entry()?
        .set_secret(key)
        .map_err(|e| LibraryError::Keyring { reason: e.to_string() })
}

/// Load the key, or `None` if no entry exists (the key-loss / not-yet-enabled case).
#[cfg(any(windows, target_os = "macos"))]
pub fn load_key() -> Result<Option<[u8; 32]>> {
    match entry()?.get_secret() {
        Ok(bytes) => {
            let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| LibraryError::Keyring {
                reason: format!("stored key has wrong length: {}", bytes.len()),
            })?;
            Ok(Some(arr))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(LibraryError::Keyring { reason: e.to_string() }),
    }
}

/// Delete the key entry. Missing entry is treated as success (idempotent).
#[cfg(any(windows, target_os = "macos"))]
pub fn delete_key() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(LibraryError::Keyring { reason: e.to_string() }),
    }
}

// ---- Stubs for unsupported platforms (e.g. Linux CI). ----

#[cfg(not(any(windows, target_os = "macos")))]
pub fn store_key(_key: &[u8; 32]) -> Result<()> {
    Err(unsupported())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn load_key() -> Result<Option<[u8; 32]>> {
    Err(unsupported())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn delete_key() -> Result<()> {
    Err(unsupported())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn unsupported() -> LibraryError {
    LibraryError::EncryptionUnsupported {
        reason: "no OS credential store on this platform".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_key_is_nonzero_and_varies() {
        let a = generate_key().unwrap();
        let b = generate_key().unwrap();
        assert_ne!(a, [0u8; 32], "key must not be all zeros");
        assert_ne!(a, b, "two generated keys must differ");
    }

    // Real keychain round-trip only runs where a credential store exists.
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn store_load_delete_round_trip() {
        let key = generate_key().unwrap();
        store_key(&key).unwrap();
        assert_eq!(load_key().unwrap(), Some(key));
        delete_key().unwrap();
        assert_eq!(load_key().unwrap(), None, "deleted key reads back as None");
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn storage_is_unsupported_off_platform() {
        assert!(matches!(
            load_key(),
            Err(LibraryError::EncryptionUnsupported { .. })
        ));
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/snk-library/src/lib.rs`, add to the module list (after `pub mod error;`):

```rust
pub mod keystore;
```

(`encrypt` is declared in Task 7, when its file exists — declaring it here would break the build because `encrypt.rs` does not exist yet.)

- [ ] **Step 3: Make `Db::open` key-aware**

In `crates/snk-library/src/db.rs`, replace the body of `pub fn open` (currently `db.rs:25-52`) with an orchestration that probes then delegates to `open_with_key`:

```rust
    pub fn open(path: &Path) -> Result<Self> {
        if is_encrypted(path)? {
            match crate::keystore::load_key()? {
                Some(key) => Self::open_with_key(path, Some(&key)),
                None => Err(crate::LibraryError::Locked {
                    detail: format!(
                        "database at {} is encrypted but no key is available in the OS keychain",
                        path.display()
                    ),
                }),
            }
        } else {
            Self::open_with_key(path, None)
        }
    }
```

The `// VERIFIED: privacy-md/local-only-storage` doc comment block above `open` stays — keep it attached to the function.

- [ ] **Step 4: Verify**

Run: `cargo test -p snk-library`
Expected: all PASS. On Linux, the new `Db::open` still only takes the plaintext branch in every existing test (their DBs are plaintext), so `keystore::load_key` is never called.

Run: `cargo clippy -p snk-library --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-library/src/keystore.rs crates/snk-library/src/lib.rs crates/snk-library/src/db.rs
git commit -m "feat(library): add OS keystore + key-aware Db::open with locked-state

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: `encrypt_in_place` migration mechanic

One-way plaintext→encrypted conversion using SQLCipher's `sqlcipher_export()`. The OS-specific key-store call is **injected as a closure** so the mechanic stays cross-platform-testable. Order (crash-safe): backup → export → store-key → atomic swap → purge plaintext backups.

**Files:**
- Create: `crates/snk-library/src/encrypt.rs`
- Modify: `crates/snk-library/src/lib.rs` (declare `pub mod encrypt;` if not already done in Task 6)

- [ ] **Step 1: Write the failing test**

Create `crates/snk-library/src/encrypt.rs` with the test first (impl added next step):

```rust
//! One-way plaintext → SQLCipher conversion of the library database.

use std::path::Path;

use rusqlite::Connection;

use crate::{LibraryError, Result};

// (impl inserted in Step 2)

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [9u8; 32];

    #[test]
    fn encrypt_in_place_encrypts_and_preserves_rows_and_purges_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapper-keeper.db");

        // Build a plaintext DB with a row.
        {
            let db = crate::Db::open(&path).unwrap();
            crate::tags::create(&db, "before", "#abc").unwrap();
        }
        assert!(!crate::db::is_encrypted(&path).unwrap());

        // Encrypt in place; the injected store-key closure records the key.
        let mut stored: Option<[u8; 32]> = None;
        encrypt_in_place(&path, &KEY, || {
            stored = Some(KEY);
            Ok(())
        })
        .expect("encrypt");

        assert_eq!(stored, Some(KEY), "store-key closure must be called");
        assert!(crate::db::is_encrypted(&path).unwrap(), "DB must be encrypted now");

        // Rows survive and are readable with the key.
        let db = crate::Db::open_with_key(&path, Some(&KEY)).unwrap();
        let tags = crate::tags::list(&db).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "before");

        // No plaintext file may remain in backups/.
        let backups = path.parent().unwrap().join("backups");
        if backups.exists() {
            for entry in std::fs::read_dir(&backups).unwrap() {
                let p = entry.unwrap().path();
                if p.extension().is_some_and(|e| e == "db") {
                    assert!(
                        crate::db::is_encrypted(&p).unwrap(),
                        "plaintext backup left behind: {}",
                        p.display()
                    );
                }
            }
        }
    }

    #[test]
    fn encrypt_in_place_aborts_and_preserves_plaintext_when_store_key_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapper-keeper.db");
        {
            let db = crate::Db::open(&path).unwrap();
            crate::tags::create(&db, "keepme", "#abc").unwrap();
        }

        let err = encrypt_in_place(&path, &KEY, || {
            Err(LibraryError::Keyring { reason: "boom".into() })
        })
        .expect_err("must fail when key storage fails");
        assert!(matches!(err, LibraryError::Keyring { .. }));

        // Original plaintext DB must be intact and readable.
        assert!(!crate::db::is_encrypted(&path).unwrap(), "DB must remain plaintext");
        let db = crate::Db::open(&path).unwrap();
        assert_eq!(crate::tags::list(&db).unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library encrypt_in_place`
Expected: FAIL — `cannot find function encrypt_in_place`.

- [ ] **Step 3: Implement `encrypt_in_place`**

In `crates/snk-library/src/encrypt.rs`, insert above the `#[cfg(test)]` block:

```rust
/// Convert the plaintext database at `path` into a SQLCipher-encrypted database
/// keyed with `key`. One-way. `store_key` is invoked (after the encrypted copy
/// is produced, before the swap) to persist the key in the OS keychain — it is
/// injected so this mechanic stays cross-platform-testable.
///
/// Order is chosen for crash-safety: the original plaintext file is untouched
/// until the atomic rename, so any failure before the swap leaves a readable
/// plaintext DB. Plaintext backups are purged only after a fully successful swap.
pub fn encrypt_in_place(
    path: &Path,
    key: &[u8; 32],
    store_key: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    let tmp_enc = sibling(path, "enc-tmp");
    let _ = std::fs::remove_file(&tmp_enc);

    // 1. Export plaintext main → keyed attached DB at tmp_enc.
    {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        conn.execute(
            "ATTACH DATABASE ?1 AS encrypted KEY ?2",
            rusqlite::params![tmp_enc.to_string_lossy(), format!("x'{hex}'")],
        )?;
        conn.query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(()))?;
        conn.execute("DETACH DATABASE encrypted", [])?;
    } // conn dropped → handles released (needed on Windows before the swap)

    // 2. Persist the key. On failure, abort and leave the plaintext DB intact.
    if let Err(e) = store_key() {
        let _ = std::fs::remove_file(&tmp_enc);
        return Err(e);
    }

    // 3. Atomic swap tmp_enc → path; clear stale WAL/SHM from the plaintext DB.
    let db_str = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(format!("{db_str}-wal"));
    let _ = std::fs::remove_file(format!("{db_str}-shm"));
    std::fs::rename(&tmp_enc, path).map_err(|e| LibraryError::Encrypt {
        reason: format!("atomic swap failed: {e}"),
    })?;

    // 4. Purge any plaintext copies left in backups/ (they would defeat encryption).
    purge_plaintext_backups(path);
    Ok(())
}

/// Build a sibling path next to `path` with an inserted suffix, e.g.
/// `…/snapper-keeper.db` + "enc-tmp" → `…/snapper-keeper.db.enc-tmp`.
fn sibling(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(format!(".{suffix}"));
    std::path::PathBuf::from(s)
}

/// Remove every `.db` file in the sibling `backups/` directory that is still
/// plaintext. Best-effort — failures are ignored (purge is a hardening step,
/// not correctness-critical). Encrypted backups (post-encryption migration
/// backups) are left alone.
fn purge_plaintext_backups(db_path: &Path) {
    let Some(dir) = db_path.parent().map(|p| p.join("backups")) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "db")
            && matches!(crate::db::is_encrypted(&p), Ok(false))
        {
            let _ = std::fs::remove_file(&p);
        }
    }
}
```

Add `pub mod encrypt;` to `crates/snk-library/src/lib.rs` now (next to `pub mod keystore;`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p snk-library encrypt_in_place`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-library/src/encrypt.rs crates/snk-library/src/lib.rs
git commit -m "feat(library): add encrypt_in_place one-way SQLCipher migration

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: `enable_encryption` + `encryption_status` commands (with the 3-file ACL dance)

`encryption_status` is a read-only probe (no authz, like `get_theme`). `enable_encryption` is security-sensitive and authz-gated to the `settings` window; it guards against double-enable, runs `encrypt_in_place` with the real keystore, and returns success (the frontend then restarts).

**Files:**
- Modify: `crates/snk-library/src/authz.rs` (add window set + test)
- Modify: `crates/snk-library/src/commands.rs` (two commands)
- Modify: `crates/snk-library/src/plugin.rs` (`invoke_handler!`)
- Modify: `crates/snk-library/build.rs` (`COMMANDS`)
- Modify: `crates/snk-library/permissions/default.toml`

- [ ] **Step 1: Add the authorized-window set + test**

In `crates/snk-library/src/authz.rs`, after `pub const SET_SETTING_WINDOWS` (line 22), add:

```rust
pub const ENABLE_ENCRYPTION_WINDOWS: &[&str] = &["settings"];
```

Add a test in `mod tests`:

```rust
    #[test]
    fn enable_encryption_allows_settings_only() {
        assert!(is_authorized("settings", ENABLE_ENCRYPTION_WINDOWS));
        assert!(!is_authorized("library", ENABLE_ENCRYPTION_WINDOWS));
        assert!(!is_authorized("clipboard-popup", ENABLE_ENCRYPTION_WINDOWS));
    }
```

Run: `cargo test -p snk-library enable_encryption_allows_settings_only`
Expected: PASS.

- [ ] **Step 2: Add the two commands**

In `crates/snk-library/src/commands.rs`, append (after `get_theme`):

```rust
/// Read-only: is the library database encrypted on disk? Backed by the header
/// probe, not the settings table (which is unreadable when encrypted). No authz
/// gate — purely informational, like `get_theme`. Returns a bare bool (no new
/// IPC struct needed).
#[tauri::command]
pub fn encryption_status<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
) -> Result<bool> {
    let db_path = state.root.join("snapper-keeper.db");
    crate::db::is_encrypted(&db_path)
}

/// Enable encryption-at-rest: a one-way plaintext → SQLCipher conversion keyed
/// by a fresh random key stored in the OS keychain. Authz-gated to the settings
/// window. The caller is expected to restart the app afterward (the live DB
/// connection still points at the now-replaced file).
#[tauri::command]
pub fn enable_encryption<R: Runtime>(
    state: State<'_, LibraryState>,
    window: tauri::WebviewWindow<R>,
) -> Result<()> {
    crate::authz::authorize(
        &window,
        "enable_encryption",
        crate::authz::ENABLE_ENCRYPTION_WINDOWS,
        "",
    )?;

    let db_path = state.root.join("snapper-keeper.db");
    if crate::db::is_encrypted(&db_path)? {
        return Ok(()); // already encrypted — idempotent no-op
    }

    let key = crate::keystore::generate_key()?;
    crate::encrypt::encrypt_in_place(&db_path, &key, || crate::keystore::store_key(&key))
}
```

- [ ] **Step 3: Register the commands (3-file ACL dance)**

(a) In `crates/snk-library/src/plugin.rs`, add to the `tauri::generate_handler!` list (after `crate::commands::get_theme,`):

```rust
            crate::commands::encryption_status,
            crate::commands::enable_encryption,
```

(b) In `crates/snk-library/build.rs`, add to `COMMANDS` (after `"get_theme",`):

```rust
    "encryption_status",
    "enable_encryption",
```

(c) In `crates/snk-library/permissions/default.toml`, add to the `permissions` array (after `"allow-get-theme",`):

```toml
    "allow-encryption-status",
    "allow-enable-encryption",
```

- [ ] **Step 4: Verify it builds and the status command works**

Run: `cargo build -p snk-library`
Expected: compiles; `build.rs` regenerates the per-command permission TOMLs under `permissions/autogenerated/`.

Run: `cargo test -p snk-library`
Expected: PASS. (No new generated TS binding — `encryption_status` returns a bare `bool`.)

- [ ] **Step 5: Commit**

```bash
git add crates/snk-library/src/authz.rs crates/snk-library/src/commands.rs crates/snk-library/src/plugin.rs crates/snk-library/build.rs crates/snk-library/permissions/default.toml
git commit -m "feat(library): add enable_encryption + encryption_status commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 9: Handle the `Locked` state in plugin setup (fail-fast)

When the DB is encrypted but the key is unavailable, fail fast with a clear, logged reason and a `library:locked` event — never auto-delete (a transient keychain miss must not destroy data).

**Files:**
- Modify: `crates/snk-library/src/plugin.rs:53-73` (setup match)

- [ ] **Step 1: Add a `Locked` arm to the open match**

In `crates/snk-library/src/plugin.rs`, in the `match Db::open(&db_path)` block, add an arm before the catch-all `Err(e) => …`:

```rust
                Err(crate::LibraryError::Locked { detail }) => {
                    tracing::error!(
                        path = %db_path.display(),
                        %detail,
                        "library is encrypted but its key is unavailable (keychain reset, \
                         locked, or moved machine). Not starting fresh automatically — a \
                         transient keychain miss must not destroy data. To start over, \
                         remove the database file."
                    );
                    let _ = app.emit("library:locked", detail.clone());
                    return Err(format!("library locked: {detail}").into());
                }
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build -p snk-library`
Expected: compiles (the `Emitter` trait is already imported — `use tauri::Emitter` is present at the top of plugin.rs).

Run: `cargo test -p snk-library`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/snk-library/src/plugin.rs
git commit -m "feat(library): fail fast with a clear reason when the library is locked

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 10: Frontend — Privacy settings section + bindings

**Files:**
- Modify: `packages/snk-library/src/index.ts` (bindings)
- Create: `app/src/windows/settings/PrivacySettings.tsx`
- Create: `app/src/windows/settings/PrivacySettings.test.tsx`
- Modify: `app/src/windows/settings/SettingsWindow.tsx` (mount it)

- [ ] **Step 1: Add the TS bindings**

In `packages/snk-library/src/index.ts`, add (near the other invoke wrappers):

```typescript
export function encryptionStatus(): Promise<boolean> {
  return invoke<boolean>('plugin:snk-library|encryption_status');
}

export function enableEncryption(): Promise<void> {
  return invoke<void>('plugin:snk-library|enable_encryption');
}
```

- [ ] **Step 2: Write the failing component test**

Create `app/src/windows/settings/PrivacySettings.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import { PrivacySettings } from './PrivacySettings';

const mockStatus = vi.fn();
vi.mock('@snk/library', () => ({
  encryptionStatus: () => mockStatus(),
  enableEncryption: vi.fn(),
}));
vi.mock('../../components/Modal', () => ({
  useModal: () => ({ confirm: vi.fn(), alert: vi.fn(), custom: vi.fn() }),
}));
vi.mock('@snk/updater', () => ({ restart: vi.fn() }));

function wrap(ui: React.ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={qc}>{ui}</QueryClientProvider>);
}

describe('PrivacySettings', () => {
  beforeEach(() => mockStatus.mockReset());

  it('shows the Encrypt button when not encrypted', async () => {
    mockStatus.mockResolvedValue(false);
    wrap(<PrivacySettings />);
    await waitFor(() => expect(screen.getByText(/Encrypt…/)).toBeInTheDocument());
  });

  it('shows the locked On state when encrypted', async () => {
    mockStatus.mockResolvedValue(true);
    wrap(<PrivacySettings />);
    await waitFor(() => expect(screen.getByText(/On/)).toBeInTheDocument());
    expect(screen.getByText(/cannot be undone/i)).toBeInTheDocument();
  });
});
```

Run: `cd app && pnpm vitest run src/windows/settings/PrivacySettings.test.tsx`
Expected: FAIL — cannot resolve `./PrivacySettings`.

- [ ] **Step 3: Implement the component**

Create `app/src/windows/settings/PrivacySettings.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query';

import { encryptionStatus, enableEncryption } from '@snk/library';
import { restart } from '@snk/updater';

import { SettingsSection } from '../../components/SettingsSection';
import { SettingRow } from '../../components/SettingRow';
import { Button } from '../../components/Button';
import { useModal } from '../../components/Modal';

export function PrivacySettings() {
  const modal = useModal();
  const statusQ = useQuery({
    queryKey: ['encryption-status'],
    queryFn: () => encryptionStatus(),
  });
  const enabled = statusQ.data ?? false;

  const onEncrypt = () => {
    modal.confirm({
      title: 'Enable encryption?',
      danger: true,
      confirmLabel: 'Encrypt my library',
      cancelLabel: 'Cancel',
      body: (
        <div className="space-y-2 text-sm">
          <p>
            Your library will be encrypted with a key stored in your OS keychain
            (Windows Credential Manager / macOS Keychain).
          </p>
          <p className="text-fg-muted">
            ⚠ If you reset your keychain or move to a new machine, this data
            cannot be recovered. There is no backdoor and no recovery key. The
            app will restart to finish.
          </p>
        </div>
      ),
      onConfirm: async () => {
        await enableEncryption();
        await restart();
      },
    });
  };

  return (
    <SettingsSection title="Privacy">
      <SettingRow
        label="Encrypt library"
        description={
          enabled
            ? 'Encryption is on; support requests cannot include the DB. This cannot be undone here.'
            : 'Encrypt the local database with a key stored in your OS keychain.'
        }
      >
        {enabled ? (
          <span className="text-sm text-fg-muted">🔒 On</span>
        ) : (
          <Button variant="secondary" onClick={onEncrypt} disabled={statusQ.isLoading}>
            Encrypt…
          </Button>
        )}
      </SettingRow>
    </SettingsSection>
  );
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd app && pnpm vitest run src/windows/settings/PrivacySettings.test.tsx`
Expected: both tests PASS.

- [ ] **Step 5: Mount it in the settings window**

In `app/src/windows/settings/SettingsWindow.tsx`, add the import (after the `UpdateSettings` import, line 14):

```tsx
import { PrivacySettings } from './PrivacySettings';
```

Add `<PrivacySettings />` to the render tree, immediately before `<UpdateSettings />` (line 175):

```tsx
        <PrivacySettings />
        <UpdateSettings />
```

- [ ] **Step 6: Verify typecheck + lint**

Run: `cd app && pnpm exec tsc --noEmit && pnpm exec eslint src/windows/settings/PrivacySettings.tsx`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add packages/snk-library/src/index.ts app/src/windows/settings/PrivacySettings.tsx app/src/windows/settings/PrivacySettings.test.tsx app/src/windows/settings/SettingsWindow.tsx
git commit -m "feat(settings): add Privacy section with one-way Encrypt library control

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 11: About-panel note, docs, and Apple export-compliance flag

**Files:**
- Modify: `app/src/windows/settings/AboutSection.tsx` (note when encrypted)
- Modify: `app/src-tauri/tauri.conf.json` (`ITSAppUsesNonExemptEncryption`)
- Modify: `PRIVACY.md:62-64`
- Modify: `README.md` (Strawberry Perl prerequisite)

- [ ] **Step 1: Add the encrypted note to the About panel**

In `app/src/windows/settings/AboutSection.tsx`, add the import (after line 21):

```tsx
import { encryptionStatus } from '@snk/library';
```

Inside `AboutSection`, after the existing `useQuery` blocks (around line 94), add:

```tsx
  const encQ = useQuery({
    queryKey: ['encryption-status'],
    queryFn: () => encryptionStatus(),
  });
```

In the returned JSX, add a row after the `License` row (before the closing `</SettingsSection>`):

```tsx
      {encQ.data && (
        <SettingRow label="Encryption">
          <span className="text-sm text-fg-muted">
            On — support requests cannot include the DB
          </span>
        </SettingRow>
      )}
```

- [ ] **Step 2: Set the Apple export-compliance flag**

In `app/src-tauri/tauri.conf.json`, locate the macOS bundle section (`bundle.macOS`). Add the Info.plist key so notarization/App-review doesn't stall on missing export-compliance. Under `bundle.macOS`, add (merge into the existing object):

```json
"infoPlist": {
  "ITSAppUsesNonExemptEncryption": false
}
```

If `bundle.macOS.infoPlist` already exists, add only the `ITSAppUsesNonExemptEncryption` key inside it. (`false` = the app uses only standard/exempt encryption for its own local data — no custom cryptography. Confirm this classification at release time.)

Run: `cd app && pnpm exec tauri build --help >/dev/null` is not needed; instead validate JSON:
Run: `node -e "JSON.parse(require('fs').readFileSync('app/src-tauri/tauri.conf.json','utf8'))"`
Expected: no output, exit 0 (valid JSON).

- [ ] **Step 3: Update PRIVACY.md**

In `PRIVACY.md`, replace the paragraph at lines 62-64:

```markdown
Data at rest is stored unencrypted in a local SQLite database, relying on your
operating system's user-account isolation. To remove everything, delete the
application data directory shown in Settings → About.
```

with:

```markdown
By default, data at rest is stored unencrypted in a local SQLite database,
relying on your operating system's user-account isolation. You can optionally
enable encryption in Settings → Privacy → "Encrypt library", which encrypts the
database with SQLCipher using a key stored in your OS keychain (Windows
Credential Manager / macOS Keychain). Note that **only the database is
encrypted; captured image files in the application data directory remain
unencrypted.** Encryption is one-way and cannot be disabled from within the app;
if your OS keychain is reset or you move to a new machine, encrypted data cannot
be recovered. To remove everything, delete the application data directory shown
in Settings → About.
```

- [ ] **Step 4: Document the Strawberry Perl prerequisite**

In `README.md`, find the Windows dev-prerequisites section (search for "Windows" near toolchain/setup). Add a bullet:

```markdown
- **Strawberry Perl** (Windows only) — the database layer builds vendored
  OpenSSL (for SQLCipher), whose configure step requires a full Perl. Install
  with `winget install --id StrawberryPerl.StrawberryPerl`. Git Bash's bundled
  msys Perl is **not** sufficient; build from PowerShell, or use
  `scripts/build-local.sh` which forces Strawberry Perl onto PATH.
```

(If the README has no per-OS prerequisites list, add this under the main setup/Build section.)

- [ ] **Step 5: Verify**

Run: `cd app && pnpm exec tsc --noEmit`
Expected: no errors.

Run: `node -e "JSON.parse(require('fs').readFileSync('app/src-tauri/tauri.conf.json','utf8'))"`
Expected: valid JSON.

- [ ] **Step 6: Commit**

```bash
git add app/src/windows/settings/AboutSection.tsx app/src-tauri/tauri.conf.json PRIVACY.md README.md
git commit -m "docs(privacy): About-panel note, export-compliance flag, encryption docs

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final verification

- [ ] **Full Rust suite:** `cargo test --workspace` → all PASS.
- [ ] **Clippy:** `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- [ ] **Frontend:** `cd app && pnpm vitest run && pnpm exec tsc --noEmit && pnpm exec eslint .` → all green.
- [ ] **Generated bindings are LF:** `git diff --check` shows no CRLF issues in `packages/**/generated/*.ts`.
- [ ] **Manual smoke (Windows interactive desktop, optional but recommended):** `pnpm tauri dev` → Settings → Privacy → Encrypt → confirm modal → app restarts → reopen Settings shows "🔒 On"; About shows the encryption note; the DB file under the app-data dir no longer starts with `SQLite format 3`.

---

## Spec coverage check

| Design section | Task(s) |
|---|---|
| §2 Key model & storage (random key, raw `PRAGMA key`, keyring v3) | 2, 5, 6 |
| §3 Open flow / header-probe | 4, 6 |
| §4 Enable flow (one-way, backup, export, swap, purge, restart) | 7, 8, 10 |
| §5 Error model + locked fail-fast | 3, 6, 9 |
| §6 UI (Privacy section, status hook, About note) | 10, 11 |
| §7 Build/deps (SQLCipher always-on, target-gated keyring, Perl fix, Apple flag) | 1, 2, 11 |
| §8 Testing (mechanics cross-platform, keystore OS-gated, purge assertion) | 4, 5, 6, 7, 10 |
| §9 Out of scope (image files plaintext → PRIVACY.md honesty) | 11 |
