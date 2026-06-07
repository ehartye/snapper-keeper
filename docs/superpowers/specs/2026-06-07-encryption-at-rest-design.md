# Opt-in encryption-at-rest (SQLCipher) — design

**Issue:** [#160](https://github.com/ehartye/snapper-keeper/issues/160) — *[privacy] Offer SQLCipher opt-in for local data at rest*
**Date:** 2026-06-07
**Status:** Approved (design); ready for implementation plan

## Summary

Add an **off-by-default** option (Settings → Privacy → "Encrypt library") that encrypts
the local SQLite database with SQLCipher. The encryption key is a random 32-byte key
stored in the OS credential store (Windows Credential Manager / macOS Keychain); there
is no user passphrase — encryption is transparent and tied to the OS user account.
Enabling encryption is a **one-way** operation (no in-app decrypt-back in v1). If the OS
keychain is lost, the data is unrecoverable by design (warn-only, no recovery key).

Both build risks are already retired by throwaway spikes (2026-06-06/07):
SQLCipher compiles and encrypts on all three OSes via
`rusqlite` `bundled-sqlcipher-vendored-openssl`, and `keyring` v3.6.3 stores/loads a
32-byte key cleanly on Windows Credential Manager (byte-identical round-trip,
clean `NoEntry` on miss).

## Decisions settled

| Decision | Choice | Rationale |
|---|---|---|
| Default | **Off** | Issue TT5 — preserves the "send me your DB file" support workflow; ~zero day-one value, so no reason to default-on. |
| Key form | **Random 32-byte key, no passphrase** | Issue specifies a "keychain-derived key". Raw key via `PRAGMA key = "x'…'"` uses the bytes directly as the AES key (skips PBKDF2). Transparent UX. |
| Key storage | **OS keychain** via `keyring` v3 | Windows Credential Manager / macOS Keychain. Spike-proven. |
| Key-loss policy | **Warn-only, no recovery** | Smallest surface; honest about the tradeoff. A side-project off-by-default feature; recovery keys/passphrases are YAGNI for v1. |
| Reversibility | **One-way (no decrypt-back in v1)** | Matches the issue. One migration path to test, not two; avoids re-exposing plaintext on disk. |
| Encryption-state detection | **Header-probe** (DB file is ground truth) | Self-describing, zero extra persisted state, cleanly detects the key-loss case. The "is encryption on" bit cannot live in the `settings` table (it's inside the encrypted DB). |
| Re-key mechanics | **Restart after enable** | App holds a live open connection; restart avoids juggling Windows file handles + WAL mid-swap for a once-ever action. |
| Scope | **DB only** | Capture image files on disk remain plaintext in v1; PRIVACY.md will say so explicitly. |

## Why `keyring` v3, not v4

`keyring` 4.0.1 is a restructured meta/CLI package and bumped its MSRV to **1.88**, above
this workspace's **1.81** floor. v3.6.3 is the clean single-crate library API
(`windows-native` / `apple-native` features, MSRV 1.75). Pin v3.

## Architecture

The design splits along a boundary that is **both a security boundary and a testability
boundary**:

- **Key mechanics (cross-platform, key-as-parameter):** header-probe, `PRAGMA key` on
  open, and the plaintext→encrypted export. Each takes the 32-byte key as an explicit
  argument, so they are fully unit-testable on Linux CI with no keychain.
- **Key storage (OS-specific, keychain):** a thin `keystore` wrapper over `keyring` v3,
  compiled only for Windows/macOS. This is the only OS-gated, CI-unexercised part.

### Files

All changes are inside `snk-library` (the sole owner of persistence) plus the frontend
Settings window.

| File | Change |
|---|---|
| `crates/snk-library/src/db.rs` | `open*` paths route through a shared `configure_conn(conn, key: Option<&[u8;32]>)`; add `is_encrypted(path)` header-probe. |
| `crates/snk-library/src/keystore.rs` *(new)* | `generate_key`, `store_key`, `load_key`, `delete_key` over `keyring` v3; `#[cfg(any(windows, target_os = "macos"))]`, stub-errors (`EncryptionUnsupported`) elsewhere. |
| `crates/snk-library/src/encrypt.rs` *(new)* | one-time `encrypt_in_place(path, key)` using SQLCipher `sqlcipher_export()`, backup + atomic swap + plaintext-backup purge. |
| `crates/snk-library/src/commands.rs` | `enable_encryption`, `encryption_status` commands (+ the 3-file ACL dance: `invoke_handler!`, `build.rs` `COMMANDS`, `permissions/default.toml`). |
| `crates/snk-library/src/error.rs` | `Locked`, `Keyring`, `Encrypt`, `EncryptionUnsupported` variants. |
| `crates/snk-library/src/plugin.rs` | Catch `Locked` in `setup()` → emit `library:locked` instead of bricking. |
| `app/src/windows/settings/SettingsWindow.tsx` (+ new `PrivacySettings.tsx`) | New Privacy section; `useEncryptionStatus` hook. |
| `app/src/windows/settings/AboutSection.tsx` | "Encryption is on…" note when enabled. |
| `Cargo.toml` (workspace) | `rusqlite` feature `bundled` → `bundled-sqlcipher-vendored-openssl`. |
| `crates/snk-library/Cargo.toml` | Add `keyring` v3 as a target-gated dep (win/mac only); add `getrandom` as a direct dep. |
| `scripts/build-local.sh` | Force real Perl on Windows (prerequisite — see Build). |
| `PRIVACY.md` | Update at-rest wording; state image files are not encrypted. |

## Key model & storage

- **Random 32-byte key** from `getrandom` (already in-tree via `uuid` v7). Passed to
  SQLCipher as `PRAGMA key = "x'<64-hex>'"` — the raw-key form (bytes used directly as
  the AES key; no PBKDF2, no passphrase).
- Stored as **bytes** via `keyring::Entry::new("snapper-keeper", "library-db-key")
  .set_secret(&key)`. Loaded via `get_secret()`; a missing entry returns
  `keyring::Error::NoEntry`.
- `keyring` is declared as a **target-gated dependency** in `crates/snk-library/Cargo.toml`,
  not the workspace root, so Linux CI never compiles it (no no-backend mock ambiguity):
  ```toml
  [target.'cfg(any(windows, target_os = "macos"))'.dependencies]
  keyring = { version = "3", features = ["windows-native", "apple-native"] }
  ```
  This matches the `keystore.rs` cfg-gate exactly. No Linux backend — not a target.

## Open flow (header-probe)

`Db::open(path)` becomes encryption-aware:

```text
bytes16 = first 16 bytes of path        # empty/missing file → treat as plaintext-new
if bytes16 == b"SQLite format 3\0":     # plaintext DB (ground truth)
    open plaintext
    configure_conn(conn, None)
    migrate                              # unchanged behavior
else:                                    # encrypted DB
    key = keystore::load_key()?
    match key:
        None    -> return LibraryError::Locked      # key-loss state
        Some(k) -> open
                   configure_conn(conn, Some(k))
                   verify readable (SELECT count(*) FROM sqlite_master)
                   migrate
```

`configure_conn(conn, key)` runs `PRAGMA key` **first** (mandatory for SQLCipher — must
precede any other statement on the connection), then the existing
`journal_mode=WAL` / `synchronous=NORMAL` / `foreign_keys=ON`. All three open paths
(`open`, `open_no_migrate`, `open_with_custom_migrations`) route through it — this
satisfies the issue's "PRAGMA key on every open path" in one place. The keychain is
consulted **only for key bytes**, never as the encryption-state signal.

`is_encrypted(path)`: read the first 16 bytes; `b"SQLite format 3\0"` ⇒ plaintext;
empty or non-existent ⇒ plaintext-new; anything else ⇒ encrypted. (A SQLCipher database
has no plaintext header — spike-confirmed.)

## Enable-encryption flow (one-way, restart-based)

Triggered by `enable_encryption` from the Privacy confirm-modal. Refuses if
`is_encrypted(path)` is already true (idempotent guard). Ordered for crash-safety:

1. `key = generate_key()` — not yet stored.
2. **Backup** the plaintext DB to a *temp* location (checkpoint `wal_checkpoint(TRUNCATE)`
   + copy) — for rollback only, **not** the shared `backups/` dir.
3. **Export**: on a connection to the plaintext DB,
   `ATTACH DATABASE '<tmp.enc>' AS enc KEY "x'<hex>'"; SELECT sqlcipher_export('enc'); DETACH enc;`
   → writes a fully-encrypted copy to `<tmp.enc>`.
4. **Store key** in the keychain.
5. **Atomic swap** `<tmp.enc>` → real DB path: close all handles, remove `-wal`/`-shm`,
   rename on the same volume.
6. **Purge plaintext backups**: securely delete the temp backup **and any pre-existing
   plaintext files in `backups/`** — otherwise we encrypt the live DB but leave plaintext
   copies on disk (a real leak). Post-encryption, future migration backups are byte-copies
   of the encrypted DB, so they are themselves encrypted — no further concern.
7. Return success → **frontend relaunches** the app (`@tauri-apps/plugin-process`
   `relaunch()`).

**Why restart, not in-place re-key:** the app holds a live `Arc<Db>` with an open
connection; swapping the file under it means juggling Windows file handles + WAL
mid-flight. Restart is a once-ever operation for a rare user action and is dramatically
safer. In-place re-key is a deliberately-deferred alternative.

**Failure handling:** any failure before the swap (step 5) leaves the original plaintext
DB untouched (we never replaced it); clean up `<tmp.enc>` and any stray keychain entry.
A crash between steps 4 and 5 leaves a plaintext DB plus a stray keychain key, which the
header-probe ignores (file is plaintext) → self-heals on next open.

## Error model & the locked state

New `LibraryError` variants. The serde discriminator tag is `"kind"`, so variant field
names must **not** be `kind` — use `reason`/`detail`:

- `Locked { detail }` — encrypted on disk, key unavailable (key-loss).
- `Keyring { reason }` — credential-store I/O failure.
- `Encrypt { reason }` — export/swap failure during enable.
- `EncryptionUnsupported { reason }` — `enable_encryption` invoked on a non-win/mac build.

`plugin.rs` `setup()` catches `Locked` (parallel to its existing
`Migration { recoverable: true }` handling): logs a clear diagnosis, emits
`library:locked`, and surfaces the typed error rather than crashing silently.

**v1 ships fail-fast, not auto-recovery.** A keychain read can fail *transiently*
(e.g. a locked macOS Keychain before the user authenticates), and auto-"starting fresh"
on a transient miss would destroy a live session's data. So v1 does **not** auto-delete
or auto-replace the encrypted DB; it stops with a clear `Locked` reason and documents
manual remediation (remove the DB file to start over). A guided "Start fresh" recovery
flow is a deferred enhancement — the user confirmed either approach is acceptable.

## UI (Settings → Privacy, new section)

- **Disabled state:** row `Encrypt library` + `[ Encrypt… ]` button → opens the warn-only
  confirm modal → on confirm, `enable_encryption` → on success, relaunch.

  ```text
  Enable Encryption?
  ──────────────────────
  Your library will be encrypted with a key stored in your OS keychain
  (Windows Credential Manager / macOS Keychain).

  ⚠  If you reset your keychain or move to a new machine, this data
     CANNOT be recovered. There is no backdoor and no recovery key.

  [ Cancel ]            [ Encrypt my library ]
  ```

- **Enabled state:** row `Encrypt library  🔒 On` + subtext *"Encryption is on; support
  requests cannot include the DB. This cannot be undone here."* No re-toggle (one-way).
- **About panel:** the same note when enabled (issue requirement).
- State source: `useEncryptionStatus()` → `encryption_status` command → Rust header-probe.
  Mirrors the existing `useAutostart` precedent (`SettingsWindow.tsx`) for state that
  cannot live in the `settings` table.

## Build / dependency / platform

- **`rusqlite`: `bundled` → `bundled-sqlcipher-vendored-openssl`**, workspace-wide,
  **always compiled in**. Runtime opt-in requires the library to always be present, and a
  SQLCipher build opens plaintext DBs natively (spike-proven), so off-by-default users are
  unaffected. *Not* Cargo-feature-gated — gating it would make runtime opt-in impossible.
- **Windows dev-setup (blocking prerequisite):** `scripts/build-local.sh` runs under Git
  bash, where msys Perl shadows Strawberry Perl and breaks the vendored-OpenSSL
  `Configure` (`Can't locate Locale/Maketext/Simple.pm`). Force a real Perl on Windows
  (prepend Strawberry to `PATH` / set `$PERL`) **before** flipping the dep, otherwise the
  `e2e-process-smoke (windows)` CI job breaks. The `build-app` job is unaffected (its
  PowerShell shell finds Strawberry on the system PATH). README dev-setup must document
  the Strawberry Perl prerequisite.
- **Apple export compliance:** adding encryption requires declaring
  `ITSAppUsesNonExemptEncryption` in the macOS bundle (via `tauri.conf.json`). A
  release-checklist item to classify the exemption (standard crypto for the app's own
  data); paperwork, not code.
- New deps: `keyring` v3 (win/mac features); `getrandom` used directly for key generation.

## Testing

The mechanics/storage split is what makes this testable without a keychain in CI.

**Cross-platform (runs on Linux CI, key passed explicitly):**
- encrypt-then-reopen round-trip (data survives, byte-identical rows);
- `is_encrypted` header-probe correctness (plaintext vs encrypted vs empty/missing);
- locked-state returned when the key is withheld on an encrypted DB;
- plaintext DB stays readable on a SQLCipher build (off-by-default users);
- enable-flow backup/rollback on an injected failure (original plaintext intact);
- idempotent-enable guard (refuses when already encrypted);
- **plaintext-backup-purge** assertion (no plaintext file left in `backups/` post-encrypt).

**OS-gated (`#[cfg(any(windows, target_os = "macos"))]`):**
- `keystore` round-trip + `NoEntry` on miss (the spike, promoted to a real test).

**e2e smoke:**
- the `build-local.sh` Perl fix is verified by the Windows `e2e-process-smoke` job
  building green.

## Out of scope (YAGNI for v1)

- Decrypt-back-to-plaintext (one-way only).
- Guided "Start fresh" locked-state recovery UI (v1 fails fast with a documented
  manual remediation; auto-recovery is unsafe on transient keychain failures).
- Recovery keys / passphrases (warn-only key-loss policy).
- Key rotation.
- Per-table or field-level encryption.
- **Encrypting capture image files** — image blobs live as separate files in the
  app-data dir and remain plaintext in v1. PRIVACY.md will state the database is
  encrypted but image files are not, so the claim stays honest.

## References

- Issue #160; split from #60.
- `migrate.rs` `create_backup()` / `Db::open` restore-on-failure — the backup primitive
  reused by the enable flow.
- Spikes (2026-06-06/07): SQLCipher build de-risk (all 3 OSes); `keyring` v3 keychain
  round-trip on Windows.
- CLAUDE.md gotchas: serde `"kind"` tag; the 3-file Tauri-command ACL dance;
  Windows OpenSSL/Perl trap; `<500`-line file guideline.
