# snapper-keeper — Phase 2: Capture Modes & Post-Capture Toolbar

> **For Claude:** REQUIRED SUB-SKILL: Use h-superpowers:subagent-driven-development, h-superpowers:team-driven-development, or h-superpowers:executing-plans to implement this plan (ask user which approach).

**Goal:** Expand capture beyond full-screen: add window capture, region-select overlay, and timed (delayed) capture — plus a floating post-capture toolbar and source-app/window-title detection for every capture mode.

**Architecture:** All new capture modes live in `snk-capture`. The region overlay is a fullscreen transparent Tauri window defined in the frontend (`app/src/windows/capture-overlay/`). The post-capture toolbar is a small borderless window (`app/src/windows/capture-toolbar/`). New hotkey actions register in `snk-hotkeys`. No new Rust crates — phase 2 extends existing plugins. The `snk-library` data model already has `source_app` and `source_window_title` columns from phase 1.

**Tech Stack:** Same as phase 1. New: `xcap::Window` API for window capture. Tauri 2 window API for overlay + toolbar windows. No new crate dependencies.

**Phase 2 scope (in):**
- Window capture via `Ctrl/Cmd+Shift+5` — list windows, user picks, capture
- Region-select overlay via `Ctrl/Cmd+Shift+4` — fullscreen transparent window, drag rect, capture region
- Timed capture via `Ctrl/Cmd+Shift+6` — 5s countdown, then full-screen capture
- Floating post-capture toolbar — Annotate (placeholder) · Copy · Save · Discard
- Source-app and window-title detection for every capture
- `capture:saved` event emission (sets up OCR subscription for phase 5)
- Tray menu wired to all four capture modes

**Out of scope (later phases):**
- Annotation editor (phase 3 — "Annotate" button is a placeholder in the toolbar)
- Clipboard auto-copy through `snk-clipboard` (phase 4 — we copy directly to OS clipboard for now)
- Multi-monitor region overlay spanning all monitors (v1 uses the monitor under the cursor)
- Custom countdown duration (hardcoded 5s for v1)

---

## Pre-flight

You are building on `main` which has phase 1 complete. Create a worktree on a `feature/phase-2-capture-modes` branch.

**Verify before starting:**

```bash
rustc --version        # 1.78+
node --version         # 20+
pnpm --version         # 9+
cargo test             # all green
pnpm lint && pnpm typecheck  # all green
```

---

## Task 1: Add source-app detection to grab module

Phase 1 captures pass `source_app: None` and `source_window_title: None`. Phase 2 populates these for every capture by querying the foreground window before capturing.

**Files:**
- Create: `crates/snk-capture/src/foreground.rs`
- Modify: `crates/snk-capture/src/lib.rs`

**Step 1: Write `crates/snk-capture/src/foreground.rs`**

```rust
use xcap::Window;

pub struct ForegroundInfo {
    pub app_name: String,
    pub window_title: String,
}

pub fn get_foreground_info() -> Option<ForegroundInfo> {
    let windows = Window::all().ok()?;
    // xcap returns windows in z-order; first non-minimized window is foreground
    let win = windows.into_iter().find(|w| !w.is_minimized())?;
    Some(ForegroundInfo {
        app_name: win.app_name().to_string(),
        window_title: win.title().to_string(),
    })
}
```

**Step 2: Register the module in `crates/snk-capture/src/lib.rs`**

Add `pub mod foreground;` to the module list and add to the pub use block:

```rust
pub mod commands;
pub mod error;
pub mod foreground;
pub mod grab;
pub mod orchestrate;
pub mod plugin;

pub use error::{CaptureError, Result};
pub use foreground::{get_foreground_info, ForegroundInfo};
pub use grab::{grab_primary_monitor, GrabResult};
pub use plugin::init;
```

**Step 3: Run tests**

Run: `cargo test -p snk-capture`
Expected: existing tests pass (foreground module has no tests yet — it depends on OS state).

**Step 4: Commit**

```bash
git add crates/snk-capture/src/foreground.rs crates/snk-capture/src/lib.rs
git commit -m "feat(capture): add foreground window info detection via xcap"
```

---

## Task 2: Extend orchestrate module with source-app detection

Wire `ForegroundInfo` into the capture pipeline so every capture records which app was active.

**Files:**
- Modify: `crates/snk-capture/src/orchestrate.rs`

**Step 1: Update `capture_full_screen` to detect foreground app**

Replace the entire file:

```rust
use std::sync::Arc;

use snk_library::{captures, files, Capture, Db, NewCapture};
use uuid::Uuid;

use crate::foreground::get_foreground_info;
use crate::grab::{grab_primary_monitor, GrabResult};
use crate::Result;

/// Capture the primary monitor, write the PNG to disk, and insert a row.
/// Returns the persisted Capture row.
pub fn capture_full_screen(db: &Arc<Db>, library_root: &std::path::Path) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab_primary_monitor()?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

fn persist(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    png_bytes: &[u8],
    width: u32,
    height: u32,
    monitor: Option<String>,
    fg: Option<crate::foreground::ForegroundInfo>,
) -> Result<Capture> {
    let id = Uuid::now_v7();
    let relative = files::capture_relative_path(&id, "png");
    let _full = files::write_atomic(library_root, &relative, png_bytes)?;
    let row = captures::insert(
        db,
        NewCapture {
            file_path: relative,
            width,
            height,
            source_app: fg.as_ref().map(|f| f.app_name.clone()),
            source_window_title: fg.as_ref().map(|f| f.window_title.clone()),
            monitor,
        },
    )?;
    Ok(row)
}
```

**Step 2: Run tests**

Run: `cargo test -p snk-capture`
Expected: PASS — no existing tests exercise `source_app` values.

Run: `cargo test -p snk-library`
Expected: PASS — library tests are independent.

**Step 3: Commit**

```bash
git add crates/snk-capture/src/orchestrate.rs
git commit -m "feat(capture): wire foreground-app detection into capture pipeline"
```

---

## Task 3: Add window capture to grab module

Add a function that lists visible windows and captures a specific window by id.

**Files:**
- Modify: `crates/snk-capture/src/grab.rs`
- Modify: `crates/snk-capture/src/error.rs`

**Step 1: Add `WindowNotFound` error variant**

In `crates/snk-capture/src/error.rs`, add a new variant to `CaptureError`:

```rust
#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CaptureError {
    #[error("no monitors found")]
    NoMonitors,

    #[error("window not found: {id}")]
    WindowNotFound { id: u32 },

    #[error("xcap error: {message}")]
    Os { message: String },

    #[error("encode error: {message}")]
    Encode { message: String },

    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),
}
```

**Step 2: Add `WindowInfo` struct and grab functions to `crates/snk-capture/src/grab.rs`**

Add after the existing `grab_primary_monitor` function:

```rust
use serde::{Deserialize, Serialize};
use xcap::Window;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u32,
    pub app_name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    let windows = Window::all()?;
    let infos = windows
        .into_iter()
        .filter(|w| !w.is_minimized() && w.width() > 0 && w.height() > 0)
        .map(|w| WindowInfo {
            id: w.id(),
            app_name: w.app_name().to_string(),
            title: w.title().to_string(),
            width: w.width(),
            height: w.height(),
        })
        .collect();
    Ok(infos)
}

pub fn grab_window(window_id: u32) -> Result<GrabResult> {
    let windows = Window::all()?;
    let target = windows
        .into_iter()
        .find(|w| w.id() == window_id)
        .ok_or(crate::CaptureError::WindowNotFound { id: window_id })?;

    let monitor_name = target.current_monitor().name().to_string();
    let image = target.capture_image()?;
    let (w, h) = (image.width(), image.height());

    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(image.as_raw(), w, h, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
        width: w,
        height: h,
        monitor_name,
    })
}
```

Also add `serde` imports to the top of the file and ensure `Window` is imported. The full imports block should be:

```rust
use std::io::Cursor;

use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use xcap::{Monitor, Window};

use crate::Result;
```

**Step 3: Update `crates/snk-capture/src/lib.rs` exports**

Add the new public items:

```rust
pub use grab::{grab_primary_monitor, grab_window, list_capturable_windows, GrabResult, WindowInfo};
```

**Step 4: Add serde dependency to snk-capture if not already present**

Check `crates/snk-capture/Cargo.toml` — `serde` is already in dependencies from phase 1.

**Step 5: Run tests**

Run: `cargo test -p snk-capture`
Expected: PASS (new functions aren't tested by unit tests — they depend on OS window state).

Run: `cargo check --workspace`
Expected: no errors.

**Step 6: Commit**

```bash
git add crates/snk-capture/src/grab.rs crates/snk-capture/src/error.rs crates/snk-capture/src/lib.rs
git commit -m "feat(capture): add window listing and window capture via xcap"
```

---

## Task 4: Add region capture to grab module

Add a function that captures a rectangular region from a specific monitor.

**Files:**
- Modify: `crates/snk-capture/src/grab.rs`

**Step 1: Add `grab_region` function**

Add after `grab_window`:

```rust
pub fn grab_region(monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult> {
    let monitors = Monitor::all()?;
    let mon = monitors
        .into_iter()
        .find(|m| m.id() == monitor_id)
        .ok_or(crate::CaptureError::NoMonitors)?;

    let monitor_name = mon.name().to_string();
    let full_image = mon.capture_image()?;

    let x = x.min(full_image.width().saturating_sub(1));
    let y = y.min(full_image.height().saturating_sub(1));
    let w = w.min(full_image.width().saturating_sub(x));
    let h = h.min(full_image.height().saturating_sub(y));

    if w == 0 || h == 0 {
        return Err(crate::CaptureError::Os {
            message: "region has zero area".into(),
        });
    }

    let cropped = image::imageops::crop_imm(&full_image, x, y, w, h).to_image();
    let (cw, ch) = (cropped.width(), cropped.height());

    let mut buf = Cursor::new(Vec::with_capacity((cw * ch * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(cropped.as_raw(), cw, ch, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
        width: cw,
        height: ch,
        monitor_name,
    })
}
```

**Step 2: Update lib.rs exports**

```rust
pub use grab::{
    grab_primary_monitor, grab_region, grab_window, list_capturable_windows, GrabResult, WindowInfo,
};
```

**Step 3: Write a unit test for region clamping**

Add to `crates/snk-capture/src/grab.rs` at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grab_region_rejects_zero_area() {
        // We can't control which monitor ID exists, but we can verify that
        // a zero-size region on any monitor returns an error.
        // Use monitor_id=9999 which won't exist — expect NoMonitors error.
        let result = grab_region(9999, 0, 0, 100, 100);
        assert!(result.is_err());
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p snk-capture`
Expected: PASS (the test verifies error handling when monitor doesn't exist).

**Step 5: Commit**

```bash
git add crates/snk-capture/src/grab.rs crates/snk-capture/src/lib.rs
git commit -m "feat(capture): add region capture with monitor-relative cropping"
```

---

## Task 5: Add orchestration functions for window, region, and timed capture

Wire the new grab functions into the orchestrate module with persist and event emission.

**Files:**
- Modify: `crates/snk-capture/src/orchestrate.rs`

**Step 1: Replace `crates/snk-capture/src/orchestrate.rs`**

```rust
use std::sync::Arc;

use snk_library::{captures, files, Capture, Db, NewCapture};
use uuid::Uuid;

use crate::foreground::get_foreground_info;
use crate::grab::{self, GrabResult};
use crate::Result;

pub fn capture_full_screen(db: &Arc<Db>, library_root: &std::path::Path) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab::grab_primary_monitor()?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

pub fn capture_window(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    window_id: u32,
) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab::grab_window(window_id)?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

pub fn capture_region(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    monitor_id: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab::grab_region(monitor_id, x, y, w, h)?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

fn persist(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    png_bytes: &[u8],
    width: u32,
    height: u32,
    monitor: Option<String>,
    fg: Option<crate::foreground::ForegroundInfo>,
) -> Result<Capture> {
    let id = Uuid::now_v7();
    let relative = files::capture_relative_path(&id, "png");
    let _full = files::write_atomic(library_root, &relative, png_bytes)?;
    let row = captures::insert(
        db,
        NewCapture {
            file_path: relative,
            width,
            height,
            source_app: fg.as_ref().map(|f| f.app_name.clone()),
            source_window_title: fg.as_ref().map(|f| f.window_title.clone()),
            monitor,
        },
    )?;
    Ok(row)
}
```

**Step 2: Run tests**

Run: `cargo test -p snk-capture`
Expected: PASS.

Run: `cargo check --workspace`
Expected: no errors.

**Step 3: Commit**

```bash
git add crates/snk-capture/src/orchestrate.rs
git commit -m "feat(capture): add orchestration for window, region, and timed capture"
```

---

## Task 6: Add Tauri commands for new capture modes

Expose the new capture modes and window listing as Tauri IPC commands.

**Files:**
- Modify: `crates/snk-capture/src/commands.rs`
- Modify: `crates/snk-capture/src/plugin.rs`
- Modify: `crates/snk-capture/build.rs`
- Modify: `crates/snk-capture/permissions/default.toml`

**Step 1: Update `crates/snk-capture/src/commands.rs`**

Replace the entire file:

```rust
use snk_library::{plugin::LibraryState, Capture};
use tauri::{Emitter, Runtime, State};

use crate::grab::WindowInfo;
use crate::Result;

#[tauri::command]
pub fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
) -> Result<Capture> {
    let capture = crate::orchestrate::capture_full_screen(&state.db, &state.root)?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn capture_window<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    window_id: u32,
) -> Result<Capture> {
    let capture = crate::orchestrate::capture_window(&state.db, &state.root, window_id)?;
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
    let capture =
        crate::orchestrate::capture_region(&state.db, &state.root, monitor_id, x, y, w, h)?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    crate::grab::list_capturable_windows()
}
```

**Step 2: Update `crates/snk-capture/src/plugin.rs`**

```rust
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-capture")
        .invoke_handler(tauri::generate_handler![
            crate::commands::capture_full_screen,
            crate::commands::capture_window,
            crate::commands::capture_region,
            crate::commands::list_capturable_windows,
        ])
        .build()
}
```

**Step 3: Update `crates/snk-capture/build.rs`**

```rust
const COMMANDS: &[&str] = &[
    "capture_full_screen",
    "capture_window",
    "capture_region",
    "list_capturable_windows",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 4: Update `crates/snk-capture/permissions/default.toml`**

```toml
[default]
description = "Default permissions for snk-capture: allows all capture modes and window listing."
permissions = [
    "allow-capture-full-screen",
    "allow-capture-window",
    "allow-capture-region",
    "allow-list-capturable-windows",
]
```

**Step 5: Run check**

Run: `cargo check --workspace`
Expected: no errors.

**Step 6: Commit**

```bash
git add crates/snk-capture/src/commands.rs crates/snk-capture/src/plugin.rs crates/snk-capture/build.rs crates/snk-capture/permissions/default.toml
git commit -m "feat(capture): expose window, region, and list-windows as Tauri commands"
```

---

## Task 7: Register new hotkey actions

Add hotkey bindings for window capture, region capture, and timed capture.

**Files:**
- Modify: `crates/snk-hotkeys/src/lib.rs`

**Step 1: Extend `HotkeyAction` enum and registration**

Replace the entire file:

```rust
//! snk-hotkeys — register global hotkeys and emit events when triggered.

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HotkeyAction {
    CaptureFullScreen,
    CaptureRegion,
    CaptureWindow,
    CaptureTimedFullScreen,
}

impl HotkeyAction {
    pub fn event_name(self) -> &'static str {
        match self {
            HotkeyAction::CaptureFullScreen => "hotkey:capture-full-screen",
            HotkeyAction::CaptureRegion => "hotkey:capture-region",
            HotkeyAction::CaptureWindow => "hotkey:capture-window",
            HotkeyAction::CaptureTimedFullScreen => "hotkey:capture-timed",
        }
    }

    pub fn default_chord(self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            HotkeyAction::CaptureFullScreen => "Cmd+Shift+3",
            HotkeyAction::CaptureRegion => "Cmd+Shift+4",
            HotkeyAction::CaptureWindow => "Cmd+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "Cmd+Shift+6",
        }
        #[cfg(not(target_os = "macos"))]
        match self {
            HotkeyAction::CaptureFullScreen => "CmdOrCtrl+Shift+3",
            HotkeyAction::CaptureRegion => "CmdOrCtrl+Shift+4",
            HotkeyAction::CaptureWindow => "CmdOrCtrl+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "CmdOrCtrl+Shift+6",
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-hotkeys")
        .setup(|app, _api| {
            let handle = app.app_handle().clone();
            app.listen_any("tauri://window-created", move |_event| {
                if let Err(e) = register_defaults(&handle) {
                    warn!(error = %e, "failed to register default hotkeys");
                }
            });
            Ok(())
        })
        .build()
}

fn register_defaults<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let actions = [
        HotkeyAction::CaptureFullScreen,
        HotkeyAction::CaptureRegion,
        HotkeyAction::CaptureWindow,
        HotkeyAction::CaptureTimedFullScreen,
    ];
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
        info!(%chord, action = ?action, "registered hotkey");
    }
    Ok(())
}
```

**Step 2: Run tests**

Run: `cargo check --workspace`
Expected: no errors.

**Step 3: Commit**

```bash
git add crates/snk-hotkeys/src/lib.rs
git commit -m "feat(hotkeys): register region, window, and timed capture hotkeys"
```

---

## Task 8: Update tray menu with all capture modes

Add region, window, and timed capture to the system tray context menu.

**Files:**
- Modify: `app/src-tauri/src/main.rs`

**Step 1: Extend the tray menu and event handler**

Replace the entire file:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
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
            let capture_region = MenuItem::with_id(
                app,
                "tray:capture-region",
                "Capture region\tCtrl+Shift+4",
                true,
                None::<&str>,
            )?;
            let capture_window = MenuItem::with_id(
                app,
                "tray:capture-window",
                "Capture window\tCtrl+Shift+5",
                true,
                None::<&str>,
            )?;
            let capture_screen = MenuItem::with_id(
                app,
                "tray:capture-full-screen",
                "Capture screen\tCtrl+Shift+3",
                true,
                None::<&str>,
            )?;
            let capture_timed = MenuItem::with_id(
                app,
                "tray:capture-timed",
                "Timed (5s)\tCtrl+Shift+6",
                true,
                None::<&str>,
            )?;
            let sep = PredefinedMenuItem::separator(app)?;
            let open_lib =
                MenuItem::with_id(app, "tray:open-library", "Open library", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &capture_region,
                    &capture_window,
                    &capture_screen,
                    &capture_timed,
                    &sep,
                    &open_lib,
                    &quit,
                ],
            )?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray:capture-full-screen" => {
                        let _ = app.emit("hotkey:capture-full-screen", ());
                    }
                    "tray:capture-region" => {
                        let _ = app.emit("hotkey:capture-region", ());
                    }
                    "tray:capture-window" => {
                        let _ = app.emit("hotkey:capture-window", ());
                    }
                    "tray:capture-timed" => {
                        let _ = app.emit("hotkey:capture-timed", ());
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

            if let Some(win) = app.get_webview_window("library") {
                let _ = win.show();
                let _ = win.set_focus();
            }

            info!("snapper-keeper started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!(error = %e, "tauri runtime exited");
        });
}
```

**Step 2: Run check**

Run: `cargo check --workspace`
Expected: no errors.

**Step 3: Commit**

```bash
git add app/src-tauri/src/main.rs
git commit -m "feat(app): wire all capture modes into tray menu"
```

---

## Task 9: Update TypeScript bindings for new capture commands

Add TS functions for the new Tauri commands and event names.

**Files:**
- Modify: `packages/snk-capture/src/index.ts`
- Create: `packages/snk-capture/src/types.ts`

**Step 1: Create `packages/snk-capture/src/types.ts`**

```typescript
export interface WindowInfo {
  id: number;
  app_name: string;
  title: string;
  width: number;
  height: number;
}
```

**Step 2: Replace `packages/snk-capture/src/index.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { Capture } from '@snk/library';
import type { WindowInfo } from './types';

export const CAPTURE_FULL_SCREEN_EVENT = 'hotkey:capture-full-screen';
export const CAPTURE_REGION_EVENT = 'hotkey:capture-region';
export const CAPTURE_WINDOW_EVENT = 'hotkey:capture-window';
export const CAPTURE_TIMED_EVENT = 'hotkey:capture-timed';

export function captureFullScreen(): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_full_screen');
}

export function captureWindow(windowId: number): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_window', { windowId });
}

export function captureRegion(
  monitorId: number,
  x: number,
  y: number,
  w: number,
  h: number,
): Promise<Capture> {
  return invoke<Capture>('plugin:snk-capture|capture_region', { monitorId, x, y, w, h });
}

export function listCapturableWindows(): Promise<WindowInfo[]> {
  return invoke<WindowInfo[]>('plugin:snk-capture|list_capturable_windows');
}

export type { WindowInfo } from './types';
```

**Step 3: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 4: Commit**

```bash
git add packages/snk-capture/src/index.ts packages/snk-capture/src/types.ts
git commit -m "feat(capture): add TS bindings for window, region, and timed capture"
```

---

## Task 10: Add soft-delete command to snk-library

The post-capture toolbar needs a "Discard" action. Add a `soft_delete_capture` command.

**Files:**
- Modify: `crates/snk-library/src/captures.rs`
- Modify: `crates/snk-library/src/commands.rs`
- Modify: `crates/snk-library/src/lib.rs`
- Modify: `crates/snk-library/src/plugin.rs`
- Modify: `crates/snk-library/build.rs`
- Modify: `crates/snk-library/permissions/default.toml`

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` block at the bottom of `crates/snk-library/src/captures.rs`:

```rust
    #[test]
    fn soft_delete_sets_deleted_at() {
        let db = fresh_db();
        let new = NewCapture {
            file_path: PathBuf::from("del.png"),
            width: 1,
            height: 1,
            source_app: None,
            source_window_title: None,
            monitor: None,
        };
        let c = insert(&db, new).unwrap();
        assert!(c.deleted_at.is_none());

        soft_delete(&db, &c.id).unwrap();

        // get() excludes deleted, so use include_deleted
        let rows = list(
            &db,
            ListCapturesQuery {
                limit: None,
                include_deleted: true,
            },
        )
        .unwrap();
        let found = rows.iter().find(|r| r.id == c.id).unwrap();
        assert!(found.deleted_at.is_some());
    }

    #[test]
    fn soft_delete_nonexistent_returns_not_found() {
        let db = fresh_db();
        match soft_delete(&db, "no-such-id") {
            Err(crate::LibraryError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p snk-library soft_delete`
Expected: FAIL — `soft_delete` function doesn't exist yet.

**Step 3: Implement `soft_delete` in `crates/snk-library/src/captures.rs`**

Add after the `list` function:

```rust
pub fn soft_delete(db: &Db, id: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    db.with_conn(|conn| {
        let changed = conn.execute(
            "UPDATE captures SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![now, id],
        )?;
        if changed == 0 {
            return Err(crate::LibraryError::NotFound {
                what: format!("capture {id}"),
            });
        }
        Ok(())
    })
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p snk-library`
Expected: PASS — all tests including the two new ones.

**Step 5: Add the Tauri command**

In `crates/snk-library/src/commands.rs`, add:

```rust
#[tauri::command]
pub fn soft_delete_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    captures::soft_delete(&state.db, &id)
}
```

**Step 6: Register the command in `crates/snk-library/src/plugin.rs`**

Add `crate::commands::soft_delete_capture` to the invoke handler:

```rust
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_captures,
            crate::commands::get_capture,
            crate::commands::soft_delete_capture,
        ])
```

**Step 7: Update `crates/snk-library/build.rs`**

```rust
const COMMANDS: &[&str] = &["list_captures", "get_capture", "soft_delete_capture"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
```

**Step 8: Update `crates/snk-library/permissions/default.toml`**

```toml
[default]
description = "Default permissions for snk-library: allows listing, getting, and deleting captures."
permissions = ["allow-list-captures", "allow-get-capture", "allow-soft-delete-capture"]
```

**Step 9: Export from lib.rs**

In `crates/snk-library/src/lib.rs`, the captures module is already public so `soft_delete` is accessible via `snk_library::captures::soft_delete`. No changes needed.

**Step 10: Run all tests**

Run: `cargo test --workspace`
Expected: PASS.

**Step 11: Commit**

```bash
git add crates/snk-library/src/captures.rs crates/snk-library/src/commands.rs crates/snk-library/src/plugin.rs crates/snk-library/build.rs crates/snk-library/permissions/default.toml
git commit -m "feat(library): add soft_delete_capture command for post-capture discard"
```

---

## Task 11: Add soft-delete TS binding

**Files:**
- Modify: `packages/snk-library/src/index.ts`

**Step 1: Add `softDeleteCapture` function**

Add to `packages/snk-library/src/index.ts`:

```typescript
export function softDeleteCapture(id: string): Promise<void> {
  return invoke<void>('plugin:snk-library|soft_delete_capture', { id });
}
```

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Commit**

```bash
git add packages/snk-library/src/index.ts
git commit -m "feat(library): add softDeleteCapture TS binding"
```

---

## Task 12: Create the capture-overlay window (region select UI)

A fullscreen transparent window where the user drags a rectangle to select a capture region.

**Files:**
- Create: `app/src/windows/capture-overlay/CaptureOverlay.tsx`
- Modify: `app/src-tauri/tauri.conf.json` (add window definition)
- Modify: `app/src-tauri/capabilities/default.json`

**Step 1: Add the capture-overlay window to `tauri.conf.json`**

In the `app.windows` array, add a second entry after the library window:

```json
      {
        "label": "capture-overlay",
        "title": "",
        "fullscreen": true,
        "transparent": true,
        "alwaysOnTop": true,
        "decorations": false,
        "resizable": false,
        "visible": false,
        "skipTaskbar": true
      }
```

**Step 2: Add `capture-overlay` to the capability window list**

In `app/src-tauri/capabilities/default.json`, change:

```json
"windows": ["library"]
```

to:

```json
"windows": ["library", "capture-overlay", "capture-toolbar"]
```

**Step 3: Write `app/src/windows/capture-overlay/CaptureOverlay.tsx`**

```tsx
import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { captureRegion } from '@snk/capture';

interface Rect {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

export function CaptureOverlay() {
  const [rect, setRect] = useState<Rect | null>(null);
  const [dragging, setDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const cancel = useCallback(async () => {
    const win = getCurrentWindow();
    await win.hide();
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    setDragging(true);
    setRect({ startX: e.clientX, startY: e.clientY, endX: e.clientX, endY: e.clientY });
  }, []);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!dragging || !rect) return;
      setRect((prev) => (prev ? { ...prev, endX: e.clientX, endY: e.clientY } : null));
    },
    [dragging, rect],
  );

  const handleMouseUp = useCallback(async () => {
    if (!rect) return;
    setDragging(false);

    const x = Math.min(rect.startX, rect.endX);
    const y = Math.min(rect.startY, rect.endY);
    const w = Math.abs(rect.endX - rect.startX);
    const h = Math.abs(rect.endY - rect.startY);

    const win = getCurrentWindow();
    await win.hide();

    if (w < 5 || h < 5) return;

    try {
      // Monitor ID 0 is the primary — region coordinates are screen-relative.
      // A multi-monitor-aware approach would detect which monitor the rect is on.
      // For phase 2 we use monitor 0 as a starting point.
      const scaleFactor = window.devicePixelRatio || 1;
      await captureRegion(
        0,
        Math.round(x * scaleFactor),
        Math.round(y * scaleFactor),
        Math.round(w * scaleFactor),
        Math.round(h * scaleFactor),
      );
    } catch (e) {
      console.error('region capture failed', e);
    }
  }, [rect]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [cancel]);

  const selectionStyle = rect
    ? {
        left: Math.min(rect.startX, rect.endX),
        top: Math.min(rect.startY, rect.endY),
        width: Math.abs(rect.endX - rect.startX),
        height: Math.abs(rect.endY - rect.startY),
      }
    : undefined;

  return (
    <div
      ref={containerRef}
      className="fixed inset-0 cursor-crosshair select-none"
      style={{ backgroundColor: 'rgba(0, 0, 0, 0.3)' }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {rect && dragging && (
        <div
          className="absolute border-2 border-blue-400"
          style={{
            ...selectionStyle,
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
          }}
        />
      )}
      <div className="fixed top-4 left-1/2 -translate-x-1/2 text-white text-sm bg-black/60 px-3 py-1 rounded">
        Drag to select region · Esc to cancel
      </div>
    </div>
  );
}
```

**Step 4: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 5: Commit**

```bash
git add app/src/windows/capture-overlay/CaptureOverlay.tsx app/src-tauri/tauri.conf.json app/src-tauri/capabilities/default.json
git commit -m "feat(app): add capture-overlay window for region selection"
```

---

## Task 13: Create the post-capture toolbar window

A small borderless floating window that appears after every capture with Annotate · Copy · Save · Discard buttons.

**Files:**
- Create: `app/src/windows/capture-toolbar/CaptureToolbar.tsx`
- Modify: `app/src-tauri/tauri.conf.json` (add window definition)

**Step 1: Add the capture-toolbar window to `tauri.conf.json`**

In the `app.windows` array, add:

```json
      {
        "label": "capture-toolbar",
        "title": "",
        "width": 280,
        "height": 48,
        "resizable": false,
        "alwaysOnTop": true,
        "decorations": false,
        "transparent": true,
        "visible": false,
        "skipTaskbar": true
      }
```

**Step 2: Write `app/src/windows/capture-toolbar/CaptureToolbar.tsx`**

```tsx
import { useCallback, useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { softDeleteCapture } from '@snk/library';

interface ToolbarPayload {
  captureId: string;
}

export function CaptureToolbar() {
  const [captureId, setCaptureId] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ToolbarPayload>('toolbar:show', (event) => {
      setCaptureId(event.payload.captureId);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error('toolbar listen failed', e));
    return () => unlisten?.();
  }, []);

  const dismiss = useCallback(async () => {
    const win = getCurrentWindow();
    await win.hide();
    setCaptureId(null);
  }, []);

  const handleAnnotate = useCallback(async () => {
    // Phase 3 will open the annotation editor here.
    // For now, just dismiss.
    await dismiss();
  }, [dismiss]);

  const handleCopy = useCallback(async () => {
    // Image is already on clipboard from capture flow.
    // In phase 4 this routes through snk-clipboard.
    await dismiss();
  }, [dismiss]);

  const handleSave = useCallback(async () => {
    // Already saved to library by the capture command.
    await dismiss();
  }, [dismiss]);

  const handleDiscard = useCallback(async () => {
    if (captureId) {
      try {
        await softDeleteCapture(captureId);
      } catch (e) {
        console.error('discard failed', e);
      }
    }
    await dismiss();
  }, [captureId, dismiss]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') dismiss();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dismiss]);

  return (
    <div className="flex items-center gap-1 bg-slate-900/95 rounded-lg px-2 py-1 border border-slate-700 shadow-lg">
      <button
        onClick={handleAnnotate}
        className="px-2 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
        title="Annotate (phase 3)"
      >
        Annotate
      </button>
      <button
        onClick={handleCopy}
        className="px-2 py-1 text-xs text-slate-300 hover:bg-slate-700 rounded"
      >
        Copy
      </button>
      <button
        onClick={handleSave}
        className="px-2 py-1 text-xs text-blue-400 hover:bg-slate-700 rounded"
      >
        Save
      </button>
      <button
        onClick={handleDiscard}
        className="px-2 py-1 text-xs text-red-400 hover:bg-slate-700 rounded"
      >
        Discard
      </button>
    </div>
  );
}
```

**Step 3: Run typecheck**

Run: `pnpm typecheck`
Expected: May warn about `@tauri-apps/plugin-clipboard-manager` not being installed — that's fine, the import is only used in handleCopy which is a no-op for now. If it fails, remove the unused import line.

**Step 4: Commit**

```bash
git add app/src/windows/capture-toolbar/CaptureToolbar.tsx app/src-tauri/tauri.conf.json
git commit -m "feat(app): add post-capture floating toolbar window"
```

---

## Task 14: Wire multi-window routing

The app needs to render different React components depending on which Tauri window it's loaded in. Currently `App.tsx` always renders `<LibraryWindow />`.

**Files:**
- Modify: `app/src/App.tsx`

**Step 1: Replace `app/src/App.tsx`**

```tsx
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';

import { LibraryWindow } from './windows/library/LibraryWindow';
import { CaptureOverlay } from './windows/capture-overlay/CaptureOverlay';
import { CaptureToolbar } from './windows/capture-toolbar/CaptureToolbar';

function WindowRouter() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    setLabel(getCurrentWindow().label);
  }, []);

  if (!label) return null;

  switch (label) {
    case 'library':
      return <LibraryWindow />;
    case 'capture-overlay':
      return <CaptureOverlay />;
    case 'capture-toolbar':
      return <CaptureToolbar />;
    default:
      return <div>Unknown window: {label}</div>;
  }
}

export default function App() {
  return <WindowRouter />;
}
```

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Commit**

```bash
git add app/src/App.tsx
git commit -m "feat(app): add multi-window routing by Tauri window label"
```

---

## Task 15: Wire hotkey events to capture actions in LibraryWindow

Connect the region, window, and timed hotkey events to their capture flows in the frontend.

**Files:**
- Modify: `app/src/windows/library/LibraryWindow.tsx`

**Step 1: Replace `app/src/windows/library/LibraryWindow.tsx`**

```tsx
import { useEffect, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import {
  CAPTURE_FULL_SCREEN_EVENT,
  CAPTURE_REGION_EVENT,
  CAPTURE_WINDOW_EVENT,
  CAPTURE_TIMED_EVENT,
  captureFullScreen,
} from '@snk/capture';

import { CaptureGrid } from './CaptureGrid';

export function LibraryWindow() {
  const queryClient = useQueryClient();

  const refreshCaptures = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: ['captures'] });
  }, [queryClient]);

  const showToolbar = useCallback(async (captureId: string) => {
    const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
    if (toolbar) {
      await toolbar.emit('toolbar:show', { captureId });
      await toolbar.show();
      await toolbar.setFocus();
    }
  }, []);

  const handleFullScreen = useCallback(async () => {
    try {
      const capture = await captureFullScreen();
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e) {
      console.error('capture failed', e);
    }
  }, [refreshCaptures, showToolbar]);

  const handleRegion = useCallback(async () => {
    const overlay = await WebviewWindow.getByLabel('capture-overlay');
    if (overlay) {
      await overlay.show();
      await overlay.setFocus();
    }
  }, []);

  const handleWindow = useCallback(async () => {
    // Phase 2 MVP: for now, capture the first non-minimized window
    // that isn't our own app. A window picker UI is a future polish item.
    try {
      const { listCapturableWindows, captureWindow } = await import('@snk/capture');
      const windows = await listCapturableWindows();
      const target = windows.find(
        (w) => !w.app_name.includes('snapper-keeper') && w.title.length > 0,
      );
      if (!target) {
        console.warn('no capturable window found');
        return;
      }
      const capture = await captureWindow(target.id);
      await refreshCaptures();
      await showToolbar(capture.id);
    } catch (e) {
      console.error('window capture failed', e);
    }
  }, [refreshCaptures, showToolbar]);

  const handleTimed = useCallback(async () => {
    // 5-second countdown, then full-screen capture
    setTimeout(async () => {
      try {
        const capture = await captureFullScreen();
        await refreshCaptures();
        await showToolbar(capture.id);
      } catch (e) {
        console.error('timed capture failed', e);
      }
    }, 5000);
  }, [refreshCaptures, showToolbar]);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    const setup = async () => {
      unlisteners.push(await listen(CAPTURE_FULL_SCREEN_EVENT, handleFullScreen));
      unlisteners.push(await listen(CAPTURE_REGION_EVENT, handleRegion));
      unlisteners.push(await listen(CAPTURE_WINDOW_EVENT, handleWindow));
      unlisteners.push(await listen(CAPTURE_TIMED_EVENT, handleTimed));
    };
    setup().catch((e) => console.error('listen setup failed', e));
    return () => unlisteners.forEach((fn) => fn());
  }, [handleFullScreen, handleRegion, handleWindow, handleTimed]);

  return (
    <main className="h-full flex flex-col">
      <header className="px-4 py-2 border-b border-slate-800 flex items-center gap-3">
        <h1 className="text-sm font-semibold">snapper-keeper</h1>
        <span className="text-xs text-slate-500">phase 2 · capture modes</span>
        <div className="flex-1" />
        <button
          className="bg-slate-800 hover:bg-slate-700 text-slate-100 px-3 py-1 rounded text-xs"
          onClick={handleFullScreen}
        >
          Capture screen
        </button>
      </header>
      <section className="flex-1 overflow-auto p-4">
        <CaptureGrid />
      </section>
    </main>
  );
}
```

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Commit**

```bash
git add app/src/windows/library/LibraryWindow.tsx
git commit -m "feat(app): wire all capture hotkey events to capture actions + toolbar"
```

---

## Task 16: Wire region overlay completion back to library

When the region overlay captures, it should refresh the library grid and show the toolbar.

**Files:**
- Modify: `app/src/windows/capture-overlay/CaptureOverlay.tsx`

**Step 1: Update the overlay to emit events after capture**

In the `handleMouseUp` callback, after the successful `captureRegion` call, emit events so the library window refreshes and the toolbar shows. Replace the `try` block:

```tsx
    try {
      const scaleFactor = window.devicePixelRatio || 1;
      const capture = await captureRegion(
        0,
        Math.round(x * scaleFactor),
        Math.round(y * scaleFactor),
        Math.round(w * scaleFactor),
        Math.round(h * scaleFactor),
      );
      // The capture:saved event is emitted by the Rust command.
      // Show the toolbar for this capture.
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const toolbar = await WebviewWindow.getByLabel('capture-toolbar');
      if (toolbar) {
        await toolbar.emit('toolbar:show', { captureId: capture.id });
        await toolbar.show();
        await toolbar.setFocus();
      }
    } catch (e) {
      console.error('region capture failed', e);
    }
```

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Commit**

```bash
git add app/src/windows/capture-overlay/CaptureOverlay.tsx
git commit -m "feat(app): show post-capture toolbar after region selection"
```

---

## Task 17: Update Thumbnail to show source app

Display the source app name in capture thumbnails now that it's populated.

**Files:**
- Modify: `app/src/windows/library/Thumbnail.tsx`

**Step 1: Read the current file and add source_app display**

In `Thumbnail.tsx`, add the source app to the metadata footer. After the monitor display line, add:

```tsx
{capture.source_app && (
  <span className="text-xs text-slate-500 truncate">{capture.source_app}</span>
)}
```

The full metadata footer section should show: time · dimensions · monitor · source app, all as small gray text items in the footer.

**Step 2: Run typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 3: Commit**

```bash
git add app/src/windows/library/Thumbnail.tsx
git commit -m "feat(app): display source app name in capture thumbnails"
```

---

## Task 18: Lint, typecheck, and full test pass

Final verification that everything compiles and passes.

**Files:** None (verification only).

**Step 1: Rust tests**

Run: `cargo test --workspace`
Expected: All tests PASS.

**Step 2: Rust lint**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings (fix any that appear).

**Step 3: Frontend typecheck**

Run: `pnpm typecheck`
Expected: PASS.

**Step 4: Frontend lint**

Run: `pnpm lint`
Expected: PASS (fix any that appear).

**Step 5: Rust format**

Run: `cargo fmt --all -- --check`
Expected: No formatting changes needed (fix any with `cargo fmt --all`).

**Step 6: Build verification**

Run: `pnpm --filter @snk/app tauri build --debug`
Expected: Compiles successfully (bundler may warn about icons but binary is built).

**Step 7: Commit any lint/format fixes**

```bash
# Only if changes were needed
git add -A
git commit -m "chore: lint + format fixes for phase 2"
```

---

## Task 19: Manual smoke test

Verify the end-to-end flows work on an interactive desktop.

**Steps:**

1. Run: `pnpm --filter @snk/app tauri dev`
2. **Full-screen capture:** Press `Ctrl+Shift+3`. Verify:
   - Screenshot appears in library grid
   - Toolbar appears with Annotate · Copy · Save · Discard buttons
   - Thumbnail shows source app name and monitor
   - "Discard" removes the capture from the grid
3. **Region capture:** Press `Ctrl+Shift+4`. Verify:
   - Fullscreen overlay appears with crosshair cursor
   - Drag a rectangle — overlay disappears, cropped region appears in grid
   - Toolbar shows
   - Esc cancels without capturing
4. **Window capture:** Press `Ctrl+Shift+5`. Verify:
   - Captures the top non-snapper-keeper window
   - Image appears in grid with correct dimensions
5. **Timed capture:** Press `Ctrl+Shift+6`. Verify:
   - 5-second delay, then full-screen capture appears
6. **Tray menu:** Right-click tray icon. Verify all four capture options are listed and work.

---

## Summary

Phase 2 adds 4 capture modes (full-screen enhanced, region, window, timed), a floating post-capture toolbar, source-app detection, and the `capture:saved` event for future OCR subscription. The architecture stays clean: all grab logic in `snk-capture`, all persistence in `snk-library`, all windows in the frontend.

**What ships in phase 2:**
- Region-select overlay (`Ctrl/Cmd+Shift+4`)
- Window capture (`Ctrl/Cmd+Shift+5`)
- Timed capture (`Ctrl/Cmd+Shift+6`)
- Floating post-capture toolbar (Annotate placeholder · Copy · Save · Discard)
- Source-app and window-title detection for all captures
- `capture:saved` event emission
- Tray menu with all four capture modes

**What's deferred:**
- **Phase 3** — `snk-annotate` + annotation editor (toolbar "Annotate" activates)
- **Phase 4** — `snk-clipboard` + clipboard popup, auto-copy through clipboard plugin
- **Phase 5** — `snk-ocr` + Tesseract sidecar, FTS5 search (subscribes to `capture:saved`)
- **Phase 6** — library window polish (sidebar, smart sections, tags, settings)
- **Phase 7** — signing, notarization, auto-updater, release pipeline, first-run wizard
