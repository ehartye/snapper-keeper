# ScreenCaptureKit Screenshot Architecture Implementation Plan

> **Execution:** Use @chewie:hyperdrive (parallel team) or @chewie:execute-plan (sequential solo) to execute this plan.

**Goal:** Replace the current macOS screenshot backend with a ScreenCaptureKit-based implementation behind a new cross-platform screenshot abstraction, while preserving Windows/Linux through adapters and keeping the bundled macOS runtime as the authoritative screenshot-validation path.

**Architecture:** Introduce a screenshot backend contract inside `snk-capture`, move the current non-macOS `xcap` path behind an adapter, and add a macOS ScreenCaptureKit backend that owns display/window enumeration, full-screen capture, region preview/final capture, and own-window exclusion. Keep runtime identity and developer workflow concerns in the app shell, but make `snk-capture` return clear typed errors for unsupported runtime, missing permission, and backend capture failure.

**Tech Stack:** Rust 1.81, Tauri 2, ScreenCaptureKit (via Rust bindings), existing `xcap` adapter on Windows/Linux, React 18, TypeScript/Vitest, Bash

---

### Task 1: Introduce the screenshot backend contract and façade

**Files:**
- Create: `crates/snk-capture/src/backend/mod.rs`
- Modify: `crates/snk-capture/src/grab.rs`
- Modify: `crates/snk-capture/src/lib.rs`
- Test: `crates/snk-capture/src/grab.rs`

- [ ] **Step 1: Write the failing contract-level tests**

Append these tests to `crates/snk-capture/src/grab.rs`:

```rust
#[cfg(test)]
mod backend_contract_tests {
    use super::*;

    #[test]
    fn window_infos_round_trip_backend_shape() {
        let windows = vec![
            WindowInfo {
                id: 7,
                app_name: "Finder".into(),
                title: "Downloads".into(),
                width: 1200,
                height: 800,
            },
            WindowInfo {
                id: 9,
                app_name: "Preview".into(),
                title: "image.png".into(),
                width: 640,
                height: 480,
            },
        ];

        assert_eq!(windows[0].id, 7);
        assert_eq!(windows[0].app_name, "Finder");
        assert_eq!(windows[1].title, "image.png");
    }

    #[test]
    fn resolve_requested_monitor_position_prefers_index_over_id_match() {
        let ids = vec![1, 2];
        assert_eq!(resolve_requested_monitor_position(&ids, 1), Some(1));
    }
}
```

- [ ] **Step 2: Run the focused test command**

Run:

```bash
cargo test --package snk-capture grab
```

Expected: PASS for the copied contract assertions, but no backend abstraction exists yet — this is the “anchor” run before the refactor.

- [ ] **Step 3: Add the backend module and route `grab.rs` through it**

Create `crates/snk-capture/src/backend/mod.rs`:

```rust
use crate::grab::{GrabResult, WindowInfo};
use crate::Result;

pub trait ScreenshotBackend {
    fn capture_primary_monitor(&self) -> Result<GrabResult>;
    fn capture_monitor(&self, monitor_id: u32) -> Result<GrabResult>;
    fn capture_window(&self, window_id: u32) -> Result<GrabResult>;
    fn capture_region(&self, monitor_id: u32, x: u32, y: u32, w: u32, h: u32)
        -> Result<GrabResult>;
    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>>;
}

#[cfg(target_os = "macos")]
mod macos_sck;
#[cfg(not(target_os = "macos"))]
mod xcap_adapter;

#[cfg(target_os = "macos")]
pub fn platform_backend() -> &'static dyn ScreenshotBackend {
    &macos_sck::BACKEND
}

#[cfg(not(target_os = "macos"))]
pub fn platform_backend() -> &'static dyn ScreenshotBackend {
    &xcap_adapter::BACKEND
}
```

In `crates/snk-capture/src/lib.rs`, add:

```rust
pub mod backend;
```

In `crates/snk-capture/src/grab.rs`, replace the top-level public entrypoints with façade calls:

```rust
pub fn grab_primary_monitor() -> Result<GrabResult> {
    crate::backend::platform_backend().capture_primary_monitor()
}

pub fn grab_monitor(monitor_id: u32) -> Result<GrabResult> {
    crate::backend::platform_backend().capture_monitor(monitor_id)
}

pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    crate::backend::platform_backend().list_capturable_windows()
}

pub fn grab_window(window_id: u32) -> Result<GrabResult> {
    crate::backend::platform_backend().capture_window(window_id)
}

pub fn grab_region(monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult> {
    crate::backend::platform_backend().capture_region(monitor_id, x, y, w, h)
}
```

Keep `resolve_requested_monitor_position`, `clamp_region`, and `encode_rgba_to_png` in `grab.rs` as reusable helpers for adapters.

- [ ] **Step 4: Run the package tests again**

Run:

```bash
cargo test --package snk-capture grab
```

Expected: FAIL because neither backend module exists yet.

- [ ] **Step 5: Commit the contract refactor**

```bash
git add crates/snk-capture/src/backend/mod.rs crates/snk-capture/src/grab.rs crates/snk-capture/src/lib.rs
git commit -m "feat(capture): add screenshot backend contract"
```

### Task 2: Move the current Windows/Linux capture path behind an xcap adapter

**Files:**
- Create: `crates/snk-capture/src/backend/xcap_adapter.rs`
- Modify: `crates/snk-capture/src/grab.rs`
- Test: `crates/snk-capture/src/grab.rs`

- [ ] **Step 1: Write a backend adapter smoke test**

Append this non-macOS-only test to `crates/snk-capture/src/grab.rs`:

```rust
#[cfg(all(test, not(target_os = "macos")))]
mod xcap_adapter_tests {
    use super::*;

    #[test]
    fn non_macos_contract_still_exposes_window_listing() {
        let _ = crate::backend::platform_backend()
            .list_capturable_windows()
            .expect("window listing should stay wired through the backend contract");
    }
}
```

- [ ] **Step 2: Run it and verify it fails**

Run:

```bash
cargo test --package snk-capture xcap_adapter_tests
```

Expected: FAIL because `xcap_adapter` does not exist.

- [ ] **Step 3: Implement `xcap_adapter.rs` by moving the current non-macOS logic**

Create `crates/snk-capture/src/backend/xcap_adapter.rs`:

```rust
use std::io::Cursor;

use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use xcap::{Monitor, Window};

use super::ScreenshotBackend;
use crate::grab::{clamp_region, encode_rgba_to_png, resolve_requested_monitor_position, GrabResult, WindowInfo};
use crate::Result;

pub static BACKEND: XcapBackend = XcapBackend;

pub struct XcapBackend;

fn select_monitor(monitor_id: Option<u32>) -> Result<Monitor> {
    let mut monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(crate::CaptureError::NoMonitors);
    }

    if let Some(id) = monitor_id {
        let monitor_ids: Vec<u32> = monitors
            .iter()
            .map(|m| m.id().unwrap_or(u32::MAX))
            .collect();
        if let Some(pos) = resolve_requested_monitor_position(&monitor_ids, id) {
            return Ok(monitors.swap_remove(pos));
        }
    }

    if let Some(pos) = monitors
        .iter()
        .position(|m| m.is_primary().unwrap_or(false))
    {
        return Ok(monitors.swap_remove(pos));
    }

    monitors.pop().ok_or(crate::CaptureError::NoMonitors)
}

impl ScreenshotBackend for XcapBackend {
    fn capture_primary_monitor(&self) -> Result<GrabResult> {
        let primary = select_monitor(None)?;
        let image = primary.capture_image()?;
        let (w, h) = (image.width(), image.height());
        let name = primary.name().unwrap_or_default();
        let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
        PngEncoder::new(&mut buf).write_image(image.as_raw(), w, h, ColorType::Rgba8.into())?;
        Ok(GrabResult {
            png_bytes: buf.into_inner(),
            width: w,
            height: h,
            monitor_name: name,
        })
    }

    fn capture_monitor(&self, monitor_id: u32) -> Result<GrabResult> {
        let monitor = select_monitor(Some(monitor_id))?;
        let image = monitor.capture_image()?;
        let (w, h) = (image.width(), image.height());
        let name = monitor.name().unwrap_or_default();
        let png_bytes = encode_rgba_to_png(image.as_raw(), w, h)?;
        Ok(GrabResult {
            png_bytes,
            width: w,
            height: h,
            monitor_name: name,
        })
    }

    fn capture_window(&self, window_id: u32) -> Result<GrabResult> {
        let windows = Window::all()?;
        let target = windows
            .into_iter()
            .find(|w| w.id().unwrap_or(0) == window_id)
            .ok_or(crate::CaptureError::WindowNotFound { id: window_id })?;

        let monitor_name = target
            .current_monitor()
            .ok()
            .and_then(|m| m.name().ok())
            .unwrap_or_default();
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

    fn capture_region(
        &self,
        monitor_id: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Result<GrabResult> {
        let monitor = select_monitor(Some(monitor_id))?;
        let monitor_name = monitor.name().unwrap_or_default();
        let full_image = monitor.capture_image()?;
        let (x, y, w, h) = clamp_region(full_image.width(), full_image.height(), x, y, w, h)
            .ok_or_else(|| crate::CaptureError::Os {
                message: "region has zero area".into(),
            })?;
        let cropped = image::imageops::crop_imm(&full_image, x, y, w, h).to_image();
        let (cw, ch) = (cropped.width(), cropped.height());
        let png_bytes = encode_rgba_to_png(cropped.as_raw(), cw, ch)?;
        Ok(GrabResult {
            png_bytes,
            width: cw,
            height: ch,
            monitor_name,
        })
    }

    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>> {
        let windows = Window::all()?;
        Ok(windows
            .into_iter()
            .filter(|w| {
                !w.is_minimized().unwrap_or(true)
                    && w.width().unwrap_or(0) > 0
                    && w.height().unwrap_or(0) > 0
            })
            .map(|w| WindowInfo {
                id: w.id().unwrap_or(0),
                app_name: w.app_name().unwrap_or_default(),
                title: w.title().unwrap_or_default(),
                width: w.width().unwrap_or(0),
                height: w.height().unwrap_or(0),
            })
            .collect())
    }
}
```

- [ ] **Step 4: Run the non-macOS adapter tests**

Run:

```bash
cargo test --package snk-capture
```

Expected: PASS on the full `snk-capture` package test suite.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-capture/src/backend/xcap_adapter.rs crates/snk-capture/src/grab.rs
git commit -m "refactor(capture): move non-macOS screenshots behind xcap adapter"
```

### Task 3: Add the macOS ScreenCaptureKit catalog and own-content exclusion helpers

**Files:**
- Modify: `crates/snk-capture/Cargo.toml`
- Create: `crates/snk-capture/src/backend/macos_sck.rs`
- Test: `crates/snk-capture/src/backend/macos_sck.rs`

- [ ] **Step 1: Write the failing helper tests**

Start `crates/snk-capture/src/backend/macos_sck.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_bundle_id_match_is_excluded() {
        assert!(should_exclude_own_content(
            Some("com.snapper-keeper.app"),
            "Snapper Keeper",
            "com.snapper-keeper.app",
        ));
    }

    #[test]
    fn app_name_fallback_is_case_insensitive() {
        assert!(should_exclude_own_content(
            None,
            "Snapper-Keeper Helper",
            "com.snapper-keeper.app",
        ));
    }

    #[test]
    fn unrelated_window_is_not_excluded() {
        assert!(!should_exclude_own_content(
            Some("com.apple.finder"),
            "Finder",
            "com.snapper-keeper.app",
        ));
    }
}
```

- [ ] **Step 2: Run the targeted test**

Run:

```bash
cargo test --package snk-capture own_bundle_id_match_is_excluded
```

Expected: FAIL because `macos_sck.rs` does not exist yet.

- [ ] **Step 3: Add target-specific ScreenCaptureKit dependencies**

Update `crates/snk-capture/Cargo.toml`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
block2 = "0.6"
dispatch2 = "0.3"
objc2 = "0.6"
objc2-core-graphics = "0.3"
objc2-foundation = "0.3"
objc2-screen-capture-kit = "0.3"
```

- [ ] **Step 4: Implement the macOS catalog and exclusion helpers**

Replace `crates/snk-capture/src/backend/macos_sck.rs` with:

```rust
use super::ScreenshotBackend;
use crate::grab::{GrabResult, WindowInfo};
use crate::Result;

pub static BACKEND: ScreenCaptureKitBackend = ScreenCaptureKitBackend;

pub struct ScreenCaptureKitBackend;

fn should_exclude_own_content(
    bundle_id: Option<&str>,
    app_name: &str,
    own_bundle_id: &str,
) -> bool {
    if let Some(bundle_id) = bundle_id {
        if bundle_id == own_bundle_id {
            return true;
        }
    }
    app_name.to_ascii_lowercase().contains("snapper-keeper")
}

fn current_bundle_id() -> Result<String> {
    Ok("com.snapper-keeper.app".to_string())
}

impl ScreenshotBackend for ScreenCaptureKitBackend {
    fn capture_primary_monitor(&self) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not implemented yet".into(),
        })
    }

    fn capture_monitor(&self, _monitor_id: u32) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not implemented yet".into(),
        })
    }

    fn capture_window(&self, _window_id: u32) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not implemented yet".into(),
        })
    }

    fn capture_region(
        &self,
        _monitor_id: u32,
        _x: u32,
        _y: u32,
        _w: u32,
        _h: u32,
    ) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not implemented yet".into(),
        })
    }

    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>> {
        let _own_bundle_id = current_bundle_id()?;
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_bundle_id_match_is_excluded() {
        assert!(should_exclude_own_content(
            Some("com.snapper-keeper.app"),
            "Snapper Keeper",
            "com.snapper-keeper.app",
        ));
    }

    #[test]
    fn app_name_fallback_is_case_insensitive() {
        assert!(should_exclude_own_content(
            None,
            "Snapper-Keeper Helper",
            "com.snapper-keeper.app",
        ));
    }

    #[test]
    fn unrelated_window_is_not_excluded() {
        assert!(!should_exclude_own_content(
            Some("com.apple.finder"),
            "Finder",
            "com.snapper-keeper.app",
        ));
    }
}
```

- [ ] **Step 5: Run the package tests**

Run:

```bash
cargo test --package snk-capture
```

Expected: PASS for the helper tests, while the actual backend methods still fail at runtime if invoked.

- [ ] **Step 6: Commit**

```bash
git add crates/snk-capture/Cargo.toml crates/snk-capture/src/backend/macos_sck.rs
git commit -m "feat(capture): add ScreenCaptureKit catalog and exclusion helpers"
```

### Task 4: Implement ScreenCaptureKit screenshot capture for all macOS surfaces

**Files:**
- Modify: `crates/snk-capture/src/backend/macos_sck.rs`
- Modify: `crates/snk-capture/src/permissions.rs`
- Test: `crates/snk-capture/src/backend/macos_sck.rs`

- [ ] **Step 1: Write the failing contract test for macOS own-window exclusion**

Append this test to `crates/snk-capture/src/backend/macos_sck.rs`:

```rust
    #[test]
    fn own_window_titles_are_removed_from_capturable_window_list() {
        let windows = vec![
            ("com.snapper-keeper.app", "snapper-keeper", 1u32),
            ("com.apple.finder", "Downloads", 2u32),
        ];

        let visible_ids: Vec<u32> = windows
            .into_iter()
            .filter(|(bundle_id, app_name, _)| {
                !should_exclude_own_content(Some(bundle_id), app_name, "com.snapper-keeper.app")
            })
            .map(|(_, _, id)| id)
            .collect();

        assert_eq!(visible_ids, vec![2]);
    }
```

- [ ] **Step 2: Run the test to verify the helper contract stays green**

Run:

```bash
cargo test --package snk-capture own_window_titles_are_removed_from_capturable_window_list
```

Expected: PASS. This anchors the exclusion rule before wiring live SCK capture.

- [ ] **Step 3: Replace the placeholder macOS backend with real ScreenCaptureKit capture**

In `crates/snk-capture/src/backend/macos_sck.rs`, replace the placeholder methods with real ScreenCaptureKit-backed implementations using this structure:

```rust
use std::path::PathBuf;
use std::sync::mpsc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::ClassType;
use objc2_core_graphics::{CGRect, CGPoint, CGSize};
use objc2_foundation::{NSArray, NSError, NSString, NSURL};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCRunningApplication, SCScreenshotConfiguration,
    SCScreenshotManager, SCShareableContent, SCWindow,
};

fn current_bundle_id() -> &'static str {
    "com.snapper-keeper.app"
}

fn temp_capture_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("snk-sck-{label}-{}.png", uuid::Uuid::now_v7()))
}

fn read_capture_png(path: &std::path::Path) -> Result<(Vec<u8>, u32, u32)> {
    let png_bytes = std::fs::read(path).map_err(|e| crate::CaptureError::Os {
        message: format!("read ScreenCaptureKit screenshot {}: {e}", path.display()),
    })?;
    let image = image::load_from_memory(&png_bytes)?;
    Ok((png_bytes, image.width(), image.height()))
}

fn shareable_content_sync() -> Result<Retained<SCShareableContent>> {
    let (tx, rx) = mpsc::sync_channel(1);
    let block = RcBlock::new(move |content: *mut SCShareableContent, error: *mut NSError| {
        let payload = if error.is_null() {
            Ok(unsafe { Retained::retain(content).expect("shareable content") })
        } else {
            Err(unsafe { (*error).localizedDescription().to_string() })
        };
        let _ = tx.send(payload);
    });

    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
            true,
            true,
            &block,
        );
    }

    rx.recv()
        .map_err(|e| crate::CaptureError::Os {
            message: format!("shareable content channel: {e}"),
        })?
        .map_err(|detail| crate::CaptureError::Os { message: detail })
}

fn excluded_applications(
    content: &SCShareableContent,
    own_bundle_id: &str,
) -> Retained<NSArray<SCRunningApplication>> {
    let apps = unsafe { content.applications() };
    let mut excluded = Vec::<Retained<SCRunningApplication>>::new();
    for idx in 0..apps.count() {
        let app = unsafe { apps.objectAtIndex(idx) };
        let bundle_id = unsafe { app.bundleIdentifier() }.to_string();
        let app_name = unsafe { app.applicationName() }.to_string();
        if should_exclude_own_content(Some(&bundle_id), &app_name, own_bundle_id) {
            excluded.push(app);
        }
    }
    let refs: Vec<&SCRunningApplication> = excluded.iter().map(|app| app.as_ref()).collect();
    NSArray::from_slice(&refs)
}

fn display_filter(
    display: &SCDisplay,
    content: &SCShareableContent,
) -> Retained<SCContentFilter> {
    let excluded_apps = excluded_applications(content, current_bundle_id());
    let excepting_windows = NSArray::from_slice::<&SCWindow>(&[]);
    unsafe {
        SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
            SCContentFilter::alloc(),
            display,
            &excluded_apps,
            &excepting_windows,
        )
    }
}

fn screenshot_output_sync(filter: &SCContentFilter, config: &SCScreenshotConfiguration) -> Result<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    let block = RcBlock::new(
        move |output: *mut objc2_screen_capture_kit::SCScreenshotOutput, error: *mut NSError| {
            let payload = if error.is_null() && !output.is_null() {
                Ok(())
            } else if !error.is_null() {
                Err(unsafe { (*error).localizedDescription().to_string() })
            } else {
                Err("ScreenCaptureKit returned a null screenshot output".to_string())
            };
            let _ = tx.send(payload);
        },
    );

    unsafe {
        SCScreenshotManager::captureScreenshotWithFilter_configuration_completionHandler(
            filter,
            config,
            Some(&block),
        );
    }

    rx.recv()
        .map_err(|e| crate::CaptureError::Os {
            message: format!("screenshot channel: {e}"),
        })?
        .map_err(|detail| crate::CaptureError::Os { message: detail })
}

fn config_for_rect(path: &std::path::Path, rect: CGRect, scale: f32) -> Retained<SCScreenshotConfiguration> {
    let cfg = unsafe { SCScreenshotConfiguration::new() };
    let url_str = NSString::from_str(&path.to_string_lossy());
    let url = unsafe { NSURL::fileURLWithPath(&url_str) };
    unsafe {
        cfg.setShowsCursor(false);
        cfg.setSourceRect(rect);
        cfg.setWidth((rect.size.width * scale as f64).round() as isize);
        cfg.setHeight((rect.size.height * scale as f64).round() as isize);
        cfg.setFileURL(Some(&url));
    }
    cfg
}

impl ScreenshotBackend for ScreenCaptureKitBackend {
    fn capture_primary_monitor(&self) -> Result<GrabResult> {
        self.capture_monitor(0)
    }

    fn capture_monitor(&self, monitor_id: u32) -> Result<GrabResult> {
        let content = shareable_content_sync()?;
        let displays = unsafe { content.displays() };
        let display = unsafe { displays.objectAtIndex(monitor_id as usize) };
        let filter = display_filter(&display, &content);
        let info = unsafe { SCShareableContent::infoForFilter(&filter) };
        let rect = unsafe { info.contentRect() };
        let scale = unsafe { info.pointPixelScale() };
        let path = temp_capture_path("display");
        let cfg = config_for_rect(&path, rect, scale);
        screenshot_output_sync(&filter, &cfg)?;
        let (png_bytes, width, height) = read_capture_png(&path)?;
        let _ = std::fs::remove_file(&path);
        Ok(GrabResult {
            png_bytes,
            width,
            height,
            monitor_name: format!("display-{monitor_id}"),
        })
    }

    fn capture_window(&self, window_id: u32) -> Result<GrabResult> {
        let content = shareable_content_sync()?;
        let windows = unsafe { content.windows() };
        let window = (0..windows.count())
            .map(|idx| unsafe { windows.objectAtIndex(idx) })
            .find(|window| unsafe { window.windowID() } == window_id)
            .ok_or(crate::CaptureError::WindowNotFound { id: window_id })?;
        let filter = unsafe {
            SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &window)
        };
        let info = unsafe { SCShareableContent::infoForFilter(&filter) };
        let rect = unsafe { info.contentRect() };
        let scale = unsafe { info.pointPixelScale() };
        let path = temp_capture_path("window");
        let cfg = config_for_rect(&path, rect, scale);
        screenshot_output_sync(&filter, &cfg)?;
        let (png_bytes, width, height) = read_capture_png(&path)?;
        let _ = std::fs::remove_file(&path);
        Ok(GrabResult {
            png_bytes,
            width,
            height,
            monitor_name: "window".to_string(),
        })
    }

    fn capture_region(&self, monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult> {
        let content = shareable_content_sync()?;
        let displays = unsafe { content.displays() };
        let display = unsafe { displays.objectAtIndex(monitor_id as usize) };
        let filter = display_filter(&display, &content);
        let info = unsafe { SCShareableContent::infoForFilter(&filter) };
        let display_rect = unsafe { info.contentRect() };
        let scale = unsafe { info.pointPixelScale() };
        let region_rect = CGRect::new(
            CGPoint::new(display_rect.origin.x + x as f64, display_rect.origin.y + y as f64),
            CGSize::new(w as f64, h as f64),
        );
        let path = temp_capture_path("region");
        let cfg = config_for_rect(&path, region_rect, scale);
        screenshot_output_sync(&filter, &cfg)?;
        let (png_bytes, width, height) = read_capture_png(&path)?;
        let _ = std::fs::remove_file(&path);
        Ok(GrabResult {
            png_bytes,
            width,
            height,
            monitor_name: format!("display-{monitor_id}"),
        })
    }

    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>> {
        let content = shareable_content_sync()?;
        let windows = unsafe { content.windows() };
        let mut out = Vec::new();
        for idx in 0..windows.count() {
            let window = unsafe { windows.objectAtIndex(idx) };
            let Some(app) = (unsafe { window.owningApplication() }) else {
                continue;
            };
            let bundle_id = unsafe { app.bundleIdentifier() }.to_string();
            let app_name = unsafe { app.applicationName() }.to_string();
            if should_exclude_own_content(Some(&bundle_id), &app_name, current_bundle_id()) {
                continue;
            }
            let title = unsafe { window.title() }
                .map(|s| s.to_string())
                .unwrap_or_default();
            let frame = unsafe { window.frame() };
            out.push(WindowInfo {
                id: unsafe { window.windowID() },
                app_name,
                title,
                width: frame.size.width.round() as u32,
                height: frame.size.height.round() as u32,
            });
        }
        Ok(out)
    }
}
```

Also update `crates/snk-capture/src/permissions.rs` so the macOS permission comment refers to the ScreenCaptureKit-backed screenshot path instead of `xcap` / `CGWindowListCreateImage`.

- [ ] **Step 4: Run the package tests**

Run:

```bash
cargo test --package snk-capture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-capture/src/backend/macos_sck.rs crates/snk-capture/src/permissions.rs
git commit -m "feat(capture): add ScreenCaptureKit screenshot backend"
```

### Task 5: Wire runtime, errors, and frontend UX to the new screenshot architecture

**Files:**
- Modify: `crates/snk-capture/src/error.rs`
- Modify: `crates/snk-capture/src/plugin.rs`
- Modify: `packages/snk-capture/src/index.ts`
- Modify: `packages/snk-capture/src/generated/errors.ts`
- Modify: `app/src/windows/library/LibraryWindow.tsx`
- Modify: `app/src/windows/library/LibraryWindow.test.tsx`
- Modify: `README.md`

- [ ] **Step 1: Write the failing frontend test for the runtime guidance path**

Append this test to `app/src/windows/library/LibraryWindow.test.tsx`:

```tsx
  it('keeps routing raw-dev screenshot failures to the bundled runtime guidance', async () => {
    mockedInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'plugin:snk-capture|capture_full_screen') {
        return Promise.reject({ kind: 'screen-recording-permission-denied' });
      }
      if (cmd === 'capture_runtime_status') {
        return Promise.resolve('raw-dev');
      }
      return Promise.resolve([]);
    });

    renderLibraryWindow();

    await act(async () => {
      fireEvent.click(screen.getByText(/Snap!/i));
    });

    expect(await screen.findByText(/pnpm dev:mac-capture/i)).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run the focused frontend test**

Run:

```bash
pnpm --filter @snk/app test -- app/src/windows/library/LibraryWindow.test.tsx
```

Expected: PASS if the existing guidance still holds; if it fails after backend changes, fix the UX here before proceeding.

- [ ] **Step 3: Update error / binding / docs surfaces**

In `crates/snk-capture/src/error.rs`, add any new typed error variants needed for runtime or ScreenCaptureKit-specific failures, following the existing serde/tagged-enum pattern:

```rust
    #[error("unsupported capture runtime: {detail}")]
    UnsupportedRuntime { detail: String },

    #[error("capture backend unavailable: {detail}")]
    BackendUnavailable { detail: String },
```

In `packages/snk-capture/src/generated/errors.ts`, mirror those variants in the generated union if `ts-rs` export changes the file.

In `app/src/windows/library/LibraryWindow.tsx`, keep the raw-dev bundled-runtime guidance path, but ensure any new backend/runtime errors still route through a single macOS screenshot guidance UX rather than leaking low-level backend terminology to the user.

In `README.md`, update the capture architecture description from:

```md
snk-capture/      xcap grabs + orchestrator (region, window, timed, fullscreen)
```

to:

```md
snk-capture/      screenshot backend abstraction + macOS ScreenCaptureKit + Windows/Linux adapters
```

- [ ] **Step 4: Run app and package tests**

Run:

```bash
pnpm --filter @snk/app test -- app/src/windows/library/LibraryWindow.test.tsx
pnpm --filter @snk/capture test
```

Expected: PASS on both commands.

- [ ] **Step 5: Commit**

```bash
git add crates/snk-capture/src/error.rs crates/snk-capture/src/plugin.rs packages/snk-capture/src/index.ts packages/snk-capture/src/generated/errors.ts app/src/windows/library/LibraryWindow.tsx app/src/windows/library/LibraryWindow.test.tsx README.md
git commit -m "feat(app): align macOS screenshot runtime and error UX"
```

### Task 6: Final validation, docs sync, and bundled-runtime smoke

**Files:**
- Modify: `docs/chewie/specs/2026-06-11-screencapturekit-screenshot-architecture-design.md` (only if implementation reveals a spec correction)
- Test: `crates/snk-capture/src/backend/macos_sck.rs`

- [ ] **Step 1: Regenerate bindings if needed and verify no generated drift**

Run:

```bash
cargo test --workspace --exclude snapper-keeper-app export_bindings -- --include-ignored
git diff -- packages/
```

Expected: either no diff, or only the expected `packages/snk-capture/src/generated/errors.ts` change from Task 5.

- [ ] **Step 2: Run the full repo validation suite**

Run:

```bash
pnpm lint
pnpm typecheck
pnpm test
cargo test --workspace --exclude snapper-keeper-app --exclude snk-updater
cargo fmt -- --check
cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings
```

Expected: all commands pass.

- [ ] **Step 3: Run the bundled macOS screenshot-validation workflow**

Run:

```bash
pnpm dev:mac-capture
```

Manual validation checklist:

1. Full-screen capture works.
2. Region preview renders correctly.
3. Final region capture works.
4. Window capture works.
5. With “Hide snapper-keeper windows during capture” enabled, Snapper Keeper content does **not** appear in any screenshot output.
6. If you intentionally run raw `pnpm --filter @snk/app tauri dev`, the app still routes screenshot runtime problems back to `pnpm dev:mac-capture`.

- [ ] **Step 4: Commit**

```bash
git add docs/chewie/specs/2026-06-11-screencapturekit-screenshot-architecture-design.md
git commit -m "docs(capture): sync ScreenCaptureKit screenshot architecture"
```
