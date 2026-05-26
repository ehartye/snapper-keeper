# Hide Own Windows During Capture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:team-driven-development to implement this plan.

**Goal:** Hide all snapper-keeper windows before pixel grab and restore them after, so the user's own UI doesn't leak into captures. Closes #78.

**Architecture:**
- New `WindowManager` trait + `TauriWindowManager` impl in `crates/snk-capture/src/window_hider.rs`.
- `WindowVisibilityGuard` snapshots which windows were visible at the start, hides them, and restores them on `Drop` — guarantees restore even if the capture grab panics or errors out.
- 3 capture commands in `commands.rs` (`capture_full_screen`, `capture_window`, `capture_region`) read setting `capture.hide_own_windows` (default `true`) and create a guard before invoking the orchestrator's grab.
- The `capture-overlay` label is excluded from hide/restore — it's already hidden by the React frontend (`CaptureOverlay.tsx:51`) before the region-capture IPC fires, so the guard touching it would either no-op or race.
- A short `std::thread::sleep(Duration::from_millis(50))` after hide() before the grab lets the compositor catch up (mirrors the existing 150ms pattern in the overlay).
- React side: a Toggle row in SettingsWindow's Capture section, default `true`, wired through the existing `setSetting('capture.hide_own_windows', ...)` API.

**Tech Stack:** Rust 1.78+, Tauri 2 (`Window::hide`/`show`/`is_visible` + `AppHandle::webview_windows`), React 18 (Toggle from PR A).

**Spec:** Issue #78 verbatim:
- Setting `capture.hide_own_windows`, default true.
- Hide ALL snapper-keeper-owned windows BEFORE pixel grab, restore AFTER.
- Region select: overlay stays visible during user input (handled today by frontend), other windows hidden.
- Full screen / window capture: ALL hidden.
- Timed capture: same as full screen for each frame.
- Restore must happen even if capture throws.
- Files: `crates/snk-capture/src/{orchestrate.rs, commands.rs}` + Tauri AppHandle window management.

**Worktree:** `C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows/`
**Branch:** `feat/hide-own-windows` (off origin/main a4f909c)
**Baseline:** TS 265/265 + Rust workspace clean.

---

## Conventions

- Conventional Commits: `feat(capture):`, `feat(ui):`, `test(capture):`.
- Stage explicit paths.
- One task = one commit.
- TS strict + noUncheckedIndexedAccess.
- Rust 2021, rustfmt + clippy clean (`cargo fmt -p snk-capture -- --check`, `cargo clippy -p snk-capture -- -D warnings`).
- No comments unless WHY is non-obvious.

## Dependency graph

```
T1 (WindowManager trait + guard + unit tests)
   ↓
T2 (capture commands integrate the guard)
   ↓
T3 (React toggle in Settings)
   ↓
T4 (final verification: cargo test, pnpm test, lint, typecheck, build)
```

Linear. Single implementer or two implementers (Rust + React split).

---

## Task 1: `window_hider.rs` — trait, Tauri impl, guard, tests

**Files:**
- Create: `crates/snk-capture/src/window_hider.rs`
- Modify: `crates/snk-capture/src/lib.rs` — add `pub mod window_hider;`

**Step 1: Write the failing tests**

Append to a new file `crates/snk-capture/src/window_hider.rs` (initially just the test module so it compiles):

```rust
//! Hide/restore the app's own windows around a screen grab so the
//! captured pixels don't include snapper-keeper UI. Window enumeration
//! is abstracted behind `WindowManager` so the snapshot/restore logic
//! can be unit-tested without a real Tauri runtime.

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Records every hide/show call so tests can assert on ordering +
    /// final state without driving a real Tauri runtime.
    struct MockWindowManager {
        visibility: RefCell<HashMap<String, bool>>,
        calls: RefCell<Vec<String>>,
    }

    impl MockWindowManager {
        fn new(initial: &[(&str, bool)]) -> Self {
            let map = initial.iter().map(|(k, v)| (k.to_string(), *v)).collect();
            Self {
                visibility: RefCell::new(map),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl WindowManager for MockWindowManager {
        fn list_own_windows(&self) -> Vec<(String, bool)> {
            self.visibility
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect()
        }

        fn hide(&self, label: &str) {
            self.calls.borrow_mut().push(format!("hide:{label}"));
            self.visibility
                .borrow_mut()
                .insert(label.to_string(), false);
        }

        fn show(&self, label: &str) {
            self.calls.borrow_mut().push(format!("show:{label}"));
            self.visibility.borrow_mut().insert(label.to_string(), true);
        }
    }

    #[test]
    fn guard_hides_visible_windows_on_construction() {
        let mgr = MockWindowManager::new(&[("library", true), ("settings", true)]);
        let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
        // Both labels should now be hidden.
        let viz = mgr.visibility.borrow();
        assert_eq!(viz.get("library"), Some(&false));
        assert_eq!(viz.get("settings"), Some(&false));
    }

    #[test]
    fn guard_does_not_touch_already_hidden_windows() {
        let mgr = MockWindowManager::new(&[("library", true), ("popup", false)]);
        let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
        // Only library should have been hidden (popup was already hidden).
        let calls = mgr.calls();
        assert!(calls.contains(&"hide:library".to_string()));
        assert!(!calls.contains(&"hide:popup".to_string()));
    }

    #[test]
    fn guard_restores_visibility_on_drop() {
        let mgr = MockWindowManager::new(&[("library", true), ("settings", true)]);
        {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
            // Verify hidden during scope.
            let viz = mgr.visibility.borrow();
            assert_eq!(viz.get("library"), Some(&false));
        }
        // After drop, both should be visible again.
        let viz = mgr.visibility.borrow();
        assert_eq!(viz.get("library"), Some(&true));
        assert_eq!(viz.get("settings"), Some(&true));
    }

    #[test]
    fn guard_only_restores_what_it_hid() {
        let mgr = MockWindowManager::new(&[("library", true), ("popup", false)]);
        {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
        }
        let calls = mgr.calls();
        // popup was hidden BEFORE we ran — guard must not show it on drop.
        assert!(calls.contains(&"show:library".to_string()));
        assert!(!calls.contains(&"show:popup".to_string()));
    }

    #[test]
    fn guard_respects_exclude_labels() {
        let mgr = MockWindowManager::new(&[
            ("library", true),
            ("capture-overlay", true),
        ]);
        {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &["capture-overlay"]);
            let viz = mgr.visibility.borrow();
            // Overlay must NOT have been hidden.
            assert_eq!(viz.get("capture-overlay"), Some(&true));
            assert_eq!(viz.get("library"), Some(&false));
        }
        let calls = mgr.calls();
        assert!(!calls.iter().any(|c| c.contains("capture-overlay")));
    }

    #[test]
    fn guard_restores_even_after_panic_in_scope() {
        let mgr = MockWindowManager::new(&[("library", true)]);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = WindowVisibilityGuard::hide_all(&mgr, &[]);
            panic!("simulated grab failure");
        }));
        // After the panic unwound through Drop, library must be visible.
        let viz = mgr.visibility.borrow();
        assert_eq!(viz.get("library"), Some(&true));
    }
}
```

**Step 2: Run tests, verify they fail to compile**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
cargo test -p snk-capture window_hider 2>&1 | tail -10
```

Expected: compile errors — `WindowManager`, `WindowVisibilityGuard` not defined.

**Step 3: Implement the trait + guard in the same file (above the test module)**

Add to the top of `crates/snk-capture/src/window_hider.rs` (before the `#[cfg(test)]` block):

```rust
use tauri::{AppHandle, Manager, Runtime};
use tracing::warn;

/// Abstracts the per-window operations the guard needs, so tests can
/// drive snapshot/restore logic without a real Tauri runtime.
pub trait WindowManager {
    /// Returns (label, currently-visible) for every webview window
    /// the app owns.
    fn list_own_windows(&self) -> Vec<(String, bool)>;
    fn hide(&self, label: &str);
    fn show(&self, label: &str);
}

/// Tauri-backed implementation. Used in production.
pub struct TauriWindowManager<'a, R: Runtime> {
    app: &'a AppHandle<R>,
}

impl<'a, R: Runtime> TauriWindowManager<'a, R> {
    pub fn new(app: &'a AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<'a, R: Runtime> WindowManager for TauriWindowManager<'a, R> {
    fn list_own_windows(&self) -> Vec<(String, bool)> {
        self.app
            .webview_windows()
            .into_iter()
            .map(|(label, win)| {
                let visible = win.is_visible().unwrap_or(false);
                (label, visible)
            })
            .collect()
    }

    fn hide(&self, label: &str) {
        if let Some(win) = self.app.get_webview_window(label) {
            if let Err(e) = win.hide() {
                warn!(label, error = %e, "failed to hide window for capture");
            }
        }
    }

    fn show(&self, label: &str) {
        if let Some(win) = self.app.get_webview_window(label) {
            if let Err(e) = win.show() {
                warn!(label, error = %e, "failed to restore window after capture");
            }
        }
    }
}

/// RAII guard that hides a set of windows on construction and restores
/// them on `Drop`. Restoring in Drop guarantees the windows come back
/// even if the grab panics or returns an error.
pub struct WindowVisibilityGuard<'a, W: WindowManager> {
    manager: &'a W,
    // Only labels the guard actually hid; we restore exactly this set.
    hidden_labels: Vec<String>,
}

impl<'a, W: WindowManager> WindowVisibilityGuard<'a, W> {
    /// Snapshot current visibility, hide every visible window NOT in
    /// `exclude_labels`, return a guard that will restore them on drop.
    pub fn hide_all(manager: &'a W, exclude_labels: &[&str]) -> Self {
        let mut hidden = Vec::new();
        for (label, visible) in manager.list_own_windows() {
            if !visible {
                continue;
            }
            if exclude_labels.iter().any(|ex| *ex == label) {
                continue;
            }
            manager.hide(&label);
            hidden.push(label);
        }
        Self {
            manager,
            hidden_labels: hidden,
        }
    }
}

impl<'a, W: WindowManager> Drop for WindowVisibilityGuard<'a, W> {
    fn drop(&mut self) {
        for label in &self.hidden_labels {
            self.manager.show(label);
        }
    }
}
```

**Step 4: Modify `crates/snk-capture/src/lib.rs`**

Add the module declaration. Find the existing `pub mod` lines and add:

```rust
pub mod window_hider;
```

**Step 5: Run tests, verify pass**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
cargo test -p snk-capture window_hider 2>&1 | tail -10
```

Expected: 6 tests passing.

**Step 6: Lint + fmt**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
cargo fmt -p snk-capture -- --check
cargo clippy -p snk-capture -- -D warnings
```

Expected: both clean.

**Step 7: Commit**

```bash
git add crates/snk-capture/src/window_hider.rs crates/snk-capture/src/lib.rs
git commit -m "feat(capture): add WindowVisibilityGuard with Drop-based restore"
```

---

## Task 2: Integrate the guard into the 3 capture commands

**Files:**
- Modify: `crates/snk-capture/src/commands.rs`

**Context:** Each of the 3 commands (`capture_full_screen`, `capture_window`, `capture_region`) reads the setting, wraps the orchestrator call with the guard if true. The `capture-overlay` label is excluded so the guard doesn't race the frontend's existing self-hide for region capture.

**Step 1: Modify `crates/snk-capture/src/commands.rs`**

Replace the file with:

```rust
use std::time::Duration;

use snk_library::{plugin::LibraryState, Capture};
use tauri::{Emitter, Manager, Runtime, State};

use crate::grab::WindowInfo;
use crate::window_hider::{TauriWindowManager, WindowVisibilityGuard};
use crate::Result;

const HIDE_OWN_WINDOWS_KEY: &str = "capture.hide_own_windows";
/// Labels excluded from the visibility guard. The capture overlay
/// is already hidden by the React frontend before invoking
/// capture_region (see CaptureOverlay.tsx); we exclude it to avoid
/// racing the frontend's existing hide.
const EXCLUDE_LABELS: &[&str] = &["capture-overlay"];
/// Delay between hiding our windows and grabbing pixels. Lets the
/// compositor unmap the windows before the screen capture API reads
/// the framebuffer. 50ms matches what we've seen reliable in the
/// existing overlay self-hide path.
const HIDE_SETTLE_DELAY: Duration = Duration::from_millis(50);

fn should_hide_own_windows(db: &snk_library::Db) -> bool {
    snk_library::settings::get(db, HIDE_OWN_WINDOWS_KEY)
        .ok()
        .flatten()
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Run `f` with our own windows hidden if the setting is enabled. The
/// guard restores visibility on drop, so any panic/error in `f` still
/// leaves the user's windows back up. When the setting is false, `f`
/// runs unmodified.
fn with_hidden_own_windows<R: Runtime, T, F>(
    app: &tauri::AppHandle<R>,
    db: &snk_library::Db,
    f: F,
) -> T
where
    F: FnOnce() -> T,
{
    if !should_hide_own_windows(db) {
        return f();
    }
    let manager = TauriWindowManager::new(app);
    let _guard = WindowVisibilityGuard::hide_all(&manager, EXCLUDE_LABELS);
    std::thread::sleep(HIDE_SETTLE_DELAY);
    f()
}

#[tauri::command]
pub fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
) -> Result<Capture> {
    let capture = with_hidden_own_windows(&app, &state.db, || {
        crate::orchestrate::capture_full_screen(&state.db, &state.root)
    })?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn capture_window<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    window_id: u32,
) -> Result<Capture> {
    let capture = with_hidden_own_windows(&app, &state.db, || {
        crate::orchestrate::capture_window(&state.db, &state.root, window_id)
    })?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn capture_region<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    monitor_id: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<Capture> {
    let capture = with_hidden_own_windows(&app, &state.db, || {
        crate::orchestrate::capture_region(&state.db, &state.root, monitor_id, x, y, w, h)
    })?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    crate::grab::list_capturable_windows()
}

/// Mint a fresh cache-busting token for one preview write.
/// UUIDv7 is monotonic so two consecutive calls always differ.
fn mint_preview_token() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(serde::Serialize)]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub token: String,
}

#[tauri::command]
pub fn grab_screen_preview<R: Runtime>(app: tauri::AppHandle<R>) -> Result<ScreenPreview> {
    let result = crate::grab::grab_primary_monitor()?;
    // Preview file lives under `captures/` so it falls inside the
    // assetProtocol allow scope (`$APPDATA/captures/**`). Tightening the
    // scope in #84 broke the previous root-of-app-data location: the
    // overlay backdrop's `convertFileSrc(.preview.png)` URL failed CSP/
    // scope checks and fell through to a solid black background, which
    // visually presented as "overlay blocks the images" / blank capture.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| crate::CaptureError::Os {
            message: format!("app data dir: {e}"),
        })?
        .join("captures");
    let preview_path = dir.join(".preview.png");
    std::fs::create_dir_all(&dir).map_err(|e| crate::CaptureError::Os {
        message: format!("create dir: {e}"),
    })?;
    std::fs::write(&preview_path, &result.png_bytes).map_err(|e| crate::CaptureError::Os {
        message: format!("write preview: {e}"),
    })?;
    Ok(ScreenPreview {
        path: preview_path.to_string_lossy().into_owned(),
        width: result.width,
        height: result.height,
        token: mint_preview_token(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_preview_token_yields_unique_strings() {
        let a = mint_preview_token();
        let b = mint_preview_token();
        assert_ne!(a, b, "two calls must return different tokens");
        assert!(!a.is_empty());
        assert!(!b.is_empty());
    }

    #[test]
    fn should_hide_own_windows_defaults_to_true_when_setting_missing() {
        let dir = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&dir.path().join("test.db")).unwrap();
        assert!(should_hide_own_windows(&db));
    }

    #[test]
    fn should_hide_own_windows_reads_false_when_setting_false() {
        let dir = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&dir.path().join("test.db")).unwrap();
        snk_library::settings::set(
            &db,
            HIDE_OWN_WINDOWS_KEY,
            &serde_json::Value::Bool(false),
        )
        .unwrap();
        assert!(!should_hide_own_windows(&db));
    }

    #[test]
    fn should_hide_own_windows_reads_true_when_setting_true() {
        let dir = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&dir.path().join("test.db")).unwrap();
        snk_library::settings::set(
            &db,
            HIDE_OWN_WINDOWS_KEY,
            &serde_json::Value::Bool(true),
        )
        .unwrap();
        assert!(should_hide_own_windows(&db));
    }
}
```

**Step 2: Verify the new dev-deps — tempfile is already in snk-library dev-deps; the test module above uses it directly. snk-capture also already has tempfile in its dev-deps (verified in `orchestrate.rs` tests).**

**Step 3: Build + run the new tests**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
cargo test -p snk-capture 2>&1 | tail -15
```

Expected: all pre-existing tests + 6 new window_hider tests + 3 new settings-default tests + 1 mint_preview_token test = clean run, all green.

**Step 4: Lint + fmt**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
cargo fmt -p snk-capture -- --check
cargo clippy -p snk-capture -- -D warnings
```

Expected: both clean.

**Step 5: Commit**

```bash
git add crates/snk-capture/src/commands.rs
git commit -m "feat(capture): hide own windows around all 3 capture grabs (closes #78)"
```

---

## Task 3: React Settings toggle

**Files:**
- Modify: `app/src/windows/settings/SettingsWindow.tsx`
- Modify: `app/src/windows/settings/SettingsWindow.test.tsx`

**Step 1: Modify `SettingsWindow.tsx`**

Find the existing `useSetting` block that reads the other capture settings (around line 222-227, after the `useEffect`). Add:

```tsx
  const [hideOwnWindows, setHideOwnWindows] = useSetting('capture.hide_own_windows', true);
```

Find the Capture section's `<SettingsSection title="Capture">` block. After the existing rows (Format, Auto-copy, JPG quality conditional), add a new SettingRow:

```tsx
          <SettingRow
            label="Hide snapper-keeper windows during capture"
            description="Prevents the app's own windows from appearing in screen captures."
          >
            <Toggle value={hideOwnWindows as boolean} onChange={setHideOwnWindows} />
          </SettingRow>
```

**Step 2: Add a test in `SettingsWindow.test.tsx`**

Add a new test alongside the existing `toggling Auto-copy persists the new boolean` test:

```tsx
it('toggling Hide-own-windows persists the new boolean', async () => {
  mockedInvoke.mockImplementation((cmd: string, args: unknown) => {
    if (cmd === 'plugin:snk-library|get_setting') {
      const key = (args as { key: string }).key;
      if (key === 'capture.hide_own_windows') return Promise.resolve(true);
      return Promise.resolve(null);
    }
    if (cmd === 'plugin:snk-updater|get_update_status') return Promise.resolve({ kind: 'idle' });
    if (cmd === 'plugin:snk-updater|get_last_check_at') return Promise.resolve(null);
    return Promise.resolve(undefined);
  });
  renderWithQuery(<ModalProvider><SettingsWindow /></ModalProvider>);
  await waitFor(() => expect(mockedInvoke).toHaveBeenCalled());

  const rowLabel = await screen.findByText(/Hide snapper-keeper/i);
  const row = rowLabel.closest('div.flex')!;
  const toggle = row.querySelector('button');
  expect(toggle).toBeTruthy();

  await act(async () => fireEvent.click(toggle!));
  await waitFor(() => {
    expect(invoke).toHaveBeenCalledWith('plugin:snk-library|set_setting', {
      key: 'capture.hide_own_windows',
      value: false,
    });
  });
});
```

**Step 3: Run the SettingsWindow tests**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
pnpm --filter @snk/app test -- --run src/windows/settings/SettingsWindow.test.tsx
```

Expected: 6 tests passing (5 prior + 1 new).

**Step 4: Lint + typecheck**

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
```

Expected: both clean.

**Step 5: Commit**

```bash
git add app/src/windows/settings/SettingsWindow.tsx app/src/windows/settings/SettingsWindow.test.tsx
git commit -m "feat(ui): add Hide-own-windows toggle in Settings → Capture"
```

---

## Task 4: Final verification

**Files:** None modified.

```bash
cd C:/Users/ehart/repos/snapper-keeper-worktrees/feat-hide-own-windows
pnpm -r --filter "@snk/*" --filter @snk/app test
pnpm --filter @snk/app lint
pnpm --filter @snk/app typecheck
pnpm --filter @snk/app build
cargo fmt -- --check
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
cargo test --workspace --exclude snapper-keeper-app
```

Expected: all clean. Report exact test counts (TS: 266 = 265 + 1 new; Rust: workspace + 10 new = pre-existing count + 10 new tests from snk-capture).

---

## Self-review notes

1. **Spec coverage:** Setting key + default — T2 + T3. Hide before grab — T2 `with_hidden_own_windows`. Restore after — T1 `Drop` impl. Restore on panic — T1 test `guard_restores_even_after_panic_in_scope`. Behavior matrix: region select overlay excluded via `EXCLUDE_LABELS`; full screen / window / region all wrap the orchestrator call uniformly. Timed capture: covered automatically because timed capture invokes `capture_full_screen` per frame.
2. **Placeholders:** none.
3. **Naming consistency:** `WindowManager` + `TauriWindowManager` + `WindowVisibilityGuard` + `hide_all` + `should_hide_own_windows` + `with_hidden_own_windows` — every name is used consistently across T1 + T2.
4. **Buildability:** Linear dependencies; each task has runnable commands with expected output.

## Plan-as-source-of-truth reminder

Real bugs → SendMessage `team-lead` BEFORE applying. Per memory `[[feedback_plan_as_source_of_truth]]`.
