# Sensitive-clipboard exclusion — design

**Status:** approved · ready for implementation plan
**Date:** 2026-05-24
**Resolves:** [#22](https://github.com/ehartye/snapper-keeper/issues/22) (blocker)
**Touches:** [#38](https://github.com/ehartye/snapper-keeper/issues/38) (incidentally improves SKIP_NEXT race surface on Windows), [#57](https://github.com/ehartye/snapper-keeper/issues/57) (extracts `worker_step` for the clipboard plugin)
**Dependencies:** none (lands independently; #25 logging would make observability durable but is not a hard prerequisite)

## 1. Problem

Snapper-keeper's clipboard watcher (`crates/snk-clipboard/src/watcher.rs`) polls the OS clipboard every 500 ms and writes every observed text/image to `clipboard_items` in SQLite — no filtering. Password-manager copies, OTP codes, and any other secret that touches the clipboard ends up in plaintext forever (or until the 200-item unpinned eviction fires).

Five surfaces in the repo reference the feature ("sensitive-content filtering", "sensitive-flag", "app blocklist") and zero implement it:

| Surface | Claim | Reality |
|---|---|---|
| Design spec §2 / §4.2 | "Clipboard management with … sensitive-content filtering" | not implemented |
| Design spec §8.4 | Documents the macOS + Windows OS-level detection mechanism | not wired |
| Schema `V002__clipboard_items.sql` | `sensitive INTEGER NOT NULL DEFAULT 0` column | never written |
| `crates/snk-library/src/settings.rs` | Orphaned `clipboard.app_blocklist` test fixture | no consumer |
| README | Doesn't mention it (issue #22 body was wrong about this) | n/a |

PRIVACY.md doesn't currently claim sensitive-clipboard exclusion, so the doc-reconciliation surface is lighter than the issue body suggested.

## 2. Goals & non-goals

### Goals (this PR)

- Honor the OS-level sensitive-content flags on macOS and Windows (per design spec §8.4) so password-manager copies are never persisted.
- User-managed app-blocklist as a long-tail safety net for apps that don't set the OS flag (matches what Maccy / Ditto / Pastebot / etc. all ship).
- Drop the dead `sensitive` schema column (V005 migration) — content is filtered at the watcher, so the column is unreachable.
- Capture `source_app` on every recorded clipboard event (the existing schema columns are currently NULL on every row).
- Switch Windows from poll-based to event-driven clipboard observation (`AddClipboardFormatListener` + `WM_CLIPBOARDUPDATE`) so source-app attribution is reliable.

### Non-goals

- **No hardcoded default blocklist.** Every team's password tooling is different; no major clipboard manager hardcodes a default (Maccy, Ditto, Pastebot, Paste, ClipboardFusion, 1Clipboard, Windows Win+V all start with an empty list). The orphaned default of `[1Password, KeePass, Bitwarden]` is dropped.
- **No replacement of `arboard`.** Content reads stay on arboard. Native OS calls are added only for flag inspection + source-app detection (~150 LoC).
- **No SQLCipher / at-rest encryption** for items the user does choose to record. That belongs in [#60](https://github.com/ehartye/snapper-keeper/issues/60).
- **No new Tauri IPC commands** beyond a single `snk-clipboard|detect_frontmost_app` for the Settings UI's "Add from frontmost app" convenience action. Storage uses the existing `set_setting` / `get_setting` commands.
- **No fuzzing / property tests on the blocklist JSON.** The value comes from our own UI; we fail open on malformed JSON.

## 3. Architecture

### 3.1 Module layout

```
crates/snk-clipboard/src/
├── watcher.rs        (existing — refactored to call the new modules)
├── sensitivity.rs    (new)
├── source_app.rs     (new)
├── blocklist.rs      (new)
└── platform/
    ├── mod.rs        (new — re-exports the active OS impl)
    ├── windows.rs    (new — windows-rs native calls)
    └── macos.rs      (new — objc2-app-kit native calls)
```

Native OS code lives under `platform/` so the public `sensitivity.rs` / `source_app.rs` modules stay clean trait-front-ends. The same pattern as `snk-capture/src/grab.rs` + per-OS xcap calls.

### 3.2 Per-event decision flow

```
clipboard change observed
  ↓
sensitivity::is_sensitive()  ──── true ────→  skip; bump last_hash; emit DEBUG; done
  ↓ false
source_app::current() → Option<SourceApp>
  ↓
blocklist::matches(&source_app)  ──── true ────→  skip; bump last_hash; emit INFO; done
  ↓ false
arboard reads text/image (existing path)
  ↓
clipboard::insert(... source_app: source_app.map(|s| s.identifier))
```

`last_hash` is updated on skip branches so the same skipped event isn't re-evaluated on the next poll cycle (macOS) or re-fired listener (Windows).

### 3.3 Windows event mechanism

Replace the polling thread with a hidden message-only window that registers `AddClipboardFormatListener` and processes `WM_CLIPBOARDUPDATE`. Each message triggers the per-event decision flow synchronously.

Benefits:

- Source-app attribution is reliable — frontmost-app is captured in the message handler, not after a 500 ms gap when the user may have alt-tabbed.
- Lower CPU than polling (idle clipboard = zero work).
- Incidentally narrows the race surface that [#38](https://github.com/ehartye/snapper-keeper/issues/38) flagged for the `SKIP_NEXT` AtomicBool. (Full hash-set replacement of `SKIP_NEXT` stays #38's scope; not changed here.)

Fallback: if `AddClipboardFormatListener` registration fails at startup, fall back to the existing 500 ms polling loop with the sensitivity / blocklist checks still active. Source-app attribution will be flakier in fallback mode; logged at `error` level so the operator notices.

### 3.4 macOS event mechanism

NSPasteboard has no change-notification API; keep polling. Tighten the interval from 500 ms to 100 ms — this narrows the source-app attribution window meaningfully without measurable CPU cost (each cycle is a single `changeCount` read; only a flag-changed cycle triggers the full pipeline).

### 3.5 Architectural invariants preserved

- All persistence flows through `snk-library` — the new modules don't touch SQLite directly. `blocklist::matches` reads the setting through `snk-library::settings::get`.
- No plugin imports another plugin's internals — `source_app::current()` is implemented in `snk-clipboard`'s own platform/ modules, not borrowed from `snk-capture`'s `foreground.rs` (similar but different signatures + different invocation timing).
- Failure modes degrade rather than crash (Section 6).

## 4. Components

### 4.1 `sensitivity.rs`

```rust
pub fn is_sensitive() -> bool { platform::is_sensitive() }
```

**Windows impl** — query the clipboard for any of these registered formats; sensitive if any are present:

| Format | Mechanism |
|---|---|
| `CFSTR_EXCLUDECLIPBOARDCONTENTFROMMONITORING` | `IsClipboardFormatAvailable` |
| `"CanIncludeInClipboardHistory"` registered format | Read value via `GetClipboardData`; sensitive when `value == 0` |
| `"CanUploadToCloudClipboard"` registered format | Same shape; sensitive when `value == 0` |

Native crate: `windows` (already in the workspace transitively via `tauri`). Specifically `Win32::System::DataExchange::*`.

**macOS impl** — call `NSPasteboard::general().types()` and check membership for:

- `org.nspasteboard.ConcealedType`
- `org.nspasteboard.TransientType`
- `org.nspasteboard.AutoGeneratedType`

Native crate: `objc2-app-kit` (new dev/runtime dep, ~1.5 MB).

### 4.2 `source_app.rs`

```rust
pub struct SourceApp {
    pub identifier: String,
    pub display_name: String,
    pub kind: SourceAppKind,
}

pub enum SourceAppKind { MacosBundleId, WindowsExe }

pub fn current() -> Option<SourceApp> { platform::current_source_app() }
```

**Windows** — pulled inside the `WM_CLIPBOARDUPDATE` handler:

1. `GetForegroundWindow()` → `HWND`
2. `GetWindowThreadProcessId()` → PID
3. `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `HANDLE`
4. `QueryFullProcessImageNameW()` → exe path
5. `identifier` = basename of exe (`1Password.exe`), case-folded to lowercase for comparison consistency
6. `display_name` = `GetFileVersionInfoW` → `FileDescription` field if present, else basename

**macOS** — `NSWorkspace::shared().frontmostApplication()`:

- `identifier` = `bundleIdentifier()` (e.g. `com.1password.1password8`)
- `display_name` = `localizedName()`

Returns `None` if the foreground app can't be resolved (rare — happens during fast app switches). Watcher treats `None` as "unknown source" and records the row with `source_app = NULL`.

### 4.3 `blocklist.rs`

```rust
pub fn matches(db: &Db, source: &SourceApp) -> bool
```

Reads the setting `clipboard.app_blocklist` via `snk_library::settings::get`. Setting value is a JSON array:

```jsonc
[
  {
    "identifier": "com.1password.1password8",
    "display_name": "1Password 8",
    "kind": "macos_bundle_id"
  },
  {
    "identifier": "1password.exe",
    "display_name": "1Password",
    "kind": "windows_exe"
  }
]
```

Match rules:

- `kind` must equal the source's `kind`. Cross-OS entries are inert (displayed in Settings UI but never match a live event).
- `identifier` comparison is case-insensitive for `windows_exe` entries (Windows filesystem norm), case-sensitive for `macos_bundle_id` (Apple norm).
- If the setting is unset or contains malformed JSON, return `false` (fail open — broken setting shouldn't break clipboard history).

### 4.4 `watcher.rs` refactor

Extract the per-event decision into a pure function so it's unit-testable (matches issue [#57](https://github.com/ehartye/snapper-keeper/issues/57)'s recommendation, scoped to this plugin):

```rust
pub(crate) fn worker_step(
    event: ClipboardEvent,
    state: &mut WatcherState,
    db: &Db,
    library_root: &Path,
) -> StepResult
```

`WatcherState` carries `last_hash: Option<String>`. `ClipboardEvent` carries the change observation. `StepResult` is `Saved(id) | Deduped | Skipped(SkipReason)` where `SkipReason` is `SensitiveFlag | AppBlocked(SourceApp) | NotChanged`.

Windows `WM_CLIPBOARDUPDATE` handler and macOS polling loop both become thin shells that build a `ClipboardEvent` and call `worker_step`. The hard-to-test OS-binding stays minimal; the decision logic gets full unit coverage.

### 4.5 Settings UI — Settings → Clipboard → Excluded apps

New section in the existing Settings window:

```
┌─ Excluded apps ────────────────────────────────────────┐
│ Clipboard events from these apps are never recorded.   │
│ OS-level "concealed" flags are always honored          │
│ regardless of this list.                                │
│                                                         │
│   1Password 8           com.1password.1password8   [×] │
│   KeePassXC             org.keepassxc.keepassxc    [×] │
│   ──────────────────────────────────────────────────── │
│   [ Add app… ]   [ Add from frontmost app ]            │
└─────────────────────────────────────────────────────────┘
```

- **Add app…** — modal with a text input for `identifier`, dropdown for `kind`, optional `display_name`. Required for adding apps not currently running (work password manager on a different machine, etc.).
- **Add from frontmost app** — calls a new `snk-clipboard|detect_frontmost_app` IPC command that wraps `source_app::current()`. Shows the detected identifier in a confirmation modal so users can verify before adding (avoids accidentally blocking `Code.exe` when VS Code is foreground).
- **Remove** (×) — soft, no confirmation. List re-renders.

Storage: writes the full updated array via the existing `set_setting` command. No new commands beyond `detect_frontmost_app`.

Live re-read: the watcher fetches the blocklist fresh on each event (one indexed setting lookup, ~1 ms). Toggling the list is effective immediately without restart.

Cross-OS roaming: entries created on the other OS are displayed but inert. The `kind` field is shown so users understand why an entry isn't matching. No filtering / hiding — keeping them visible matches user intent if they roam the library back later.

## 5. Data flow

### 5.1 Schema migration V005

`crates/snk-library/migrations/V005__drop_clipboard_sensitive.sql`:

```sql
ALTER TABLE clipboard_items DROP COLUMN sensitive;
```

The column has been `NOT NULL DEFAULT 0` since V002 and no query reads or writes it. Safe to drop. SQLite's `DROP COLUMN` is supported since 3.35 (Mar 2021); we ship a much newer version.

### 5.2 Setting shape

`clipboard.app_blocklist` setting in the `settings` table:

```json
[
  {
    "identifier": "string",
    "display_name": "string",
    "kind": "macos_bundle_id" | "windows_exe"
  }
]
```

Stored as a JSON string in the `value` column (existing pattern from `settings.rs`).

### 5.3 Existing `source_app` column finally populated

The `clipboard_items.source_app` column has been `TEXT` (nullable) since V002 but every row has `NULL`. With this PR, rows where `source_app::current()` returns `Some(...)` get the identifier stored. Rows with `None` continue to store `NULL`. The library window can surface this in a follow-up; this PR doesn't change any read sites.

## 6. Error handling

| Failure | Response | Severity |
|---|---|---|
| Native sensitivity check throws / returns error | Treat as `sensitive=true` (fail closed) — content is NOT recorded | `warn` log |
| `source_app::current()` returns `None` | Record the row with `source_app=NULL`; skip the blocklist check | `debug` log |
| `clipboard.app_blocklist` JSON malformed | Treat as empty (fail open) | `warn` log once per session |
| `AddClipboardFormatListener` registration fails at startup (Windows) | Fall back to 500 ms polling with sensitivity check still active | `error` log |
| OS API returns transient error mid-event | Skip this clipboard event entirely; do not update `last_hash` | `warn` log |
| `set_setting` from UI fails (DB locked, etc.) | UI surfaces an inline error; list re-renders to its pre-edit state | TS-side toast |

Fail-closed (treat as sensitive) and fail-open (treat blocklist as empty) are deliberately different defaults — the privacy posture dictates "if we can't verify safety, don't record"; the blocklist is a long-tail safety net that breaking shouldn't break the core feature.

## 7. Testing

### 7.1 Unit tests (no real OS clipboard)

- **`sensitivity.rs`** — per-OS tests behind `cfg(target_os)`; native calls behind a `SensitivityProbe` trait so a `FakeProbe` can drive every match arm: present-but-zero, present-and-one, absent, registration-error.
- **`source_app.rs`** — pure-logic tests on `SourceApp` value object: display-name fallback rules, identifier normalization for Windows exe basenames.
- **`blocklist.rs`** — DB-backed tests via `test_support::fresh_db()` (PR #81): unset setting, empty array, exact match, case-insensitive Windows match, case-sensitive macOS match, cross-OS inert match, malformed JSON.
- **`watcher.rs`** — `worker_step` exhaustive matrix: each `SkipReason` is reachable; `last_hash` updated on skips; `SKIP_NEXT` interaction preserved.

### 7.2 Integration tests (real OS clipboard)

`crates/snk-clipboard/tests/sensitivity_integration.rs`:

- Write a known value WITH the concealed flag set; assert `is_sensitive()` returns true.
- Write a known value WITHOUT any flag; assert `is_sensitive()` returns false.
- Both gated by `serial_test::serial(clipboard)` (matches PR #80's env-mutation serialization pattern).
- `cfg(target_os = "windows")` and `cfg(target_os = "macos")` variants; Linux test prints `SKIP:`.

Runs in CI on `macos-latest` and `windows-latest`.

### 7.3 Migration tests

- `v005_drops_sensitive_column` — after `to_latest()`, `PRAGMA table_info(clipboard_items)` has no `sensitive` row.
- `v004_to_v005_preserves_clipboard_data` — seed at V004 with rows that have `sensitive=0` and `sensitive=1`, migrate, assert rows still selectable.
- `migration_count_matches_latest_applied_version` (existing, PR #80) catches drift automatically.

### 7.4 UI tests (vitest + happy-dom)

For the Settings → Clipboard panel:

- `renders_empty_list_when_setting_unset`
- `renders_entries_from_setting_value`
- `add_app_modal_submits_new_entry_via_set_setting`
- `add_from_frontmost_calls_detect_frontmost_app_and_prefills_modal`
- `remove_button_persists_updated_list`
- `duplicate_identifier_blocks_add_with_inline_error`

### 7.5 What's deliberately NOT tested

- XSS on identifier inputs — covered by the general renderer-safety pattern from PR #83 (React text-child rendering).
- E2E (per-OS interactive desktop test) — issue #47 not yet landed; real-clipboard integration tests above are the closest analog.
- Fuzz/property tests on JSON parsing — values come from our own validated UI; fail-open behavior makes this low-leverage.

## 8. Out of scope

- SQLCipher / at-rest encryption (#60)
- The full `SKIP_NEXT` AtomicBool → hash-set replacement (#38) — narrowed but not eliminated by event-driven Windows path
- File-based logging (#25) — observability planned via `tracing` macros; durable when #25 lands
- Library-window UI for the `source_app` column now being populated — display work is a follow-up
- Cross-device sync of the blocklist (no servers; aligns with project posture)
- Time-based sensitivity (e.g. "block clipboard from 1Password for 30s after a copy") — Maccy and Ditto don't ship this; YAGNI

## 9. Open questions

None. All decision points resolved during brainstorming. Implementation plan can proceed.
