use std::path::PathBuf;
use std::sync::mpsc;

use block2::RcBlock;
use image::load_from_memory;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSError, NSURL};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCRunningApplication, SCScreenshotConfiguration,
    SCScreenshotManager, SCScreenshotOutput, SCShareableContent, SCWindow,
};

use super::ScreenshotBackend;
use crate::grab::{GrabResult, WindowInfo};
use crate::{CaptureError, Result};

pub struct ScreenCaptureKitBackend;

/// Returns true when the given app should be excluded from capture output
/// because it belongs to Snapper Keeper itself.
///
/// Matching priority:
/// 1. Bundle ID exact match against `own_bundle_id`.
/// 2. App name contains "snapper-keeper" (case-insensitive) — covers helper
///    processes and sub-bundles that share the same prefix but may report a
///    different bundle ID.
fn should_exclude_own_content(
    bundle_id: Option<&str>,
    app_name: &str,
    own_bundle_id: &str,
) -> bool {
    if let Some(id) = bundle_id {
        if id == own_bundle_id {
            return true;
        }
    }
    app_name.to_ascii_lowercase().contains("snapper-keeper")
}

fn current_bundle_id() -> &'static str {
    "com.snapper-keeper.app"
}

fn temp_capture_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("snk-sck-{label}-{}.png", uuid::Uuid::now_v7()))
}

fn read_capture_png(path: &std::path::Path) -> Result<(Vec<u8>, u32, u32)> {
    let png_bytes = std::fs::read(path).map_err(|e| CaptureError::Os {
        message: format!("read ScreenCaptureKit screenshot {}: {e}", path.display()),
    })?;
    let image = load_from_memory(&png_bytes)?;
    Ok((png_bytes, image.width(), image.height()))
}

fn os_error(message: impl Into<String>) -> CaptureError {
    CaptureError::Os {
        message: message.into(),
    }
}

fn retain_or_error<T: objc2::Message>(ptr: *mut T, kind: &str) -> Result<Retained<T>> {
    unsafe { Retained::retain(ptr) }
        .ok_or_else(|| os_error(format!("{kind} callback returned null")))
}

fn resolve_requested_display_position(
    displays: &[Retained<SCDisplay>],
    requested: u32,
) -> Option<usize> {
    let index = requested as usize;
    if index < displays.len() {
        return Some(index);
    }
    displays
        .iter()
        .position(|display| unsafe { display.displayID() } == requested)
}

fn shareable_content_sync() -> Result<Retained<SCShareableContent>> {
    let (tx, rx) = mpsc::sync_channel(1);
    let block = RcBlock::new(
        move |content: *mut SCShareableContent, error: *mut NSError| {
            let payload = if !error.is_null() {
                let message = retain_or_error(error, "shareable content error")
                    .map(|e| e.localizedDescription().to_string())
                    .unwrap_or_else(|err| err.to_string());
                Err(message)
            } else {
                retain_or_error(content, "shareable content").map_err(|err| err.to_string())
            };
            let _ = tx.send(payload);
        },
    );

    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
            true,
            true,
            &block,
        );
    }

    rx.recv()
        .map_err(|e| os_error(format!("shareable content channel: {e}")))?
        .map_err(os_error)
}

fn screenshot_output_sync(
    filter: &SCContentFilter,
    config: &SCScreenshotConfiguration,
) -> Result<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    let block = RcBlock::new(
        move |output: *mut SCScreenshotOutput, error: *mut NSError| {
            let payload = if !error.is_null() {
                let message = retain_or_error(error, "screenshot error")
                    .map(|e| e.localizedDescription().to_string())
                    .unwrap_or_else(|err| err.to_string());
                Err(message)
            } else if output.is_null() {
                Err("ScreenCaptureKit returned a null screenshot output".to_string())
            } else {
                Ok(())
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
        .map_err(|e| os_error(format!("screenshot channel: {e}")))?
        .map_err(os_error)
}

fn empty_windows_array() -> Retained<NSArray<SCWindow>> {
    NSArray::<SCWindow>::from_slice(&[])
}

fn excluded_applications(
    content: &SCShareableContent,
    own_bundle_id: &str,
) -> Retained<NSArray<SCRunningApplication>> {
    let apps = unsafe { content.applications() }.to_vec();
    let excluded = apps
        .into_iter()
        .filter(|app| {
            let bundle_id = unsafe { app.bundleIdentifier() }.to_string();
            let app_name = unsafe { app.applicationName() }.to_string();
            should_exclude_own_content(Some(&bundle_id), &app_name, own_bundle_id)
        })
        .collect::<Vec<_>>();
    NSArray::from_retained_slice(&excluded)
}

fn display_filter(display: &SCDisplay, content: &SCShareableContent) -> Retained<SCContentFilter> {
    let excluded_apps = excluded_applications(content, current_bundle_id());
    let excepting_windows = empty_windows_array();
    unsafe {
        SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
            SCContentFilter::alloc(),
            display,
            &excluded_apps,
            &excepting_windows,
        )
    }
}

fn config_for_rect(
    path: &std::path::Path,
    rect: CGRect,
    scale: f32,
) -> Result<Retained<SCScreenshotConfiguration>> {
    let cfg = unsafe { SCScreenshotConfiguration::new() };
    let url = NSURL::from_file_path(path)
        .ok_or_else(|| os_error(format!("build file URL for {}", path.display())))?;
    unsafe {
        cfg.setShowsCursor(false);
        cfg.setSourceRect(rect);
        cfg.setWidth((rect.size.width * scale as f64).round() as isize);
        cfg.setHeight((rect.size.height * scale as f64).round() as isize);
        cfg.setFileURL(Some(&url));
    }
    Ok(cfg)
}

fn display_label(display: &SCDisplay) -> String {
    format!("display-{}", unsafe { display.displayID() })
}

fn capture_display_rect(
    display: &SCDisplay,
    content: &SCShareableContent,
    rect: CGRect,
    label: &str,
) -> Result<GrabResult> {
    let filter = display_filter(display, content);
    let info = unsafe { SCShareableContent::infoForFilter(&filter) };
    let scale = unsafe { info.pointPixelScale() };
    let path = temp_capture_path(label);
    let cfg = config_for_rect(&path, rect, scale)?;
    screenshot_output_sync(&filter, &cfg)?;
    let (png_bytes, width, height) = read_capture_png(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(GrabResult {
        png_bytes,
        width,
        height,
        monitor_name: display_label(display),
    })
}

impl ScreenshotBackend for ScreenCaptureKitBackend {
    fn grab_primary_monitor(&self) -> Result<GrabResult> {
        self.grab_monitor(0)
    }

    fn grab_monitor(&self, monitor_id: u32) -> Result<GrabResult> {
        let content = shareable_content_sync()?;
        let displays = unsafe { content.displays() }.to_vec();
        if displays.is_empty() {
            return Err(CaptureError::NoMonitors);
        }
        let pos = resolve_requested_display_position(&displays, monitor_id)
            .ok_or(CaptureError::NoMonitors)?;
        let display = &displays[pos];
        let info = unsafe {
            let filter = display_filter(display, &content);
            SCShareableContent::infoForFilter(&filter)
        };
        let rect = unsafe { info.contentRect() };
        capture_display_rect(display, &content, rect, "display")
    }

    fn grab_window(&self, window_id: u32) -> Result<GrabResult> {
        let content = shareable_content_sync()?;
        let windows = unsafe { content.windows() }.to_vec();
        let window = windows
            .into_iter()
            .find(|window| unsafe { window.windowID() } == window_id)
            .ok_or(CaptureError::WindowNotFound { id: window_id })?;

        let filter = unsafe {
            SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &window)
        };
        let info = unsafe { SCShareableContent::infoForFilter(&filter) };
        let rect = unsafe { info.contentRect() };
        let scale = unsafe { info.pointPixelScale() };
        let path = temp_capture_path("window");
        let cfg = config_for_rect(&path, rect, scale)?;
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

    fn grab_region(&self, monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult> {
        let content = shareable_content_sync()?;
        let displays = unsafe { content.displays() }.to_vec();
        if displays.is_empty() {
            return Err(CaptureError::NoMonitors);
        }
        let pos = resolve_requested_display_position(&displays, monitor_id)
            .ok_or(CaptureError::NoMonitors)?;
        let display = &displays[pos];
        let filter = display_filter(display, &content);
        let info = unsafe { SCShareableContent::infoForFilter(&filter) };
        let display_rect = unsafe { info.contentRect() };
        let region_rect = CGRect {
            origin: CGPoint {
                x: display_rect.origin.x + x as f64,
                y: display_rect.origin.y + y as f64,
            },
            size: CGSize {
                width: w as f64,
                height: h as f64,
            },
        };
        capture_display_rect(display, &content, region_rect, "region")
    }

    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>> {
        let content = shareable_content_sync()?;
        let windows = unsafe { content.windows() }.to_vec();
        let own_bundle_id = current_bundle_id();
        let mut out = Vec::new();

        for window in windows {
            let Some(app) = (unsafe { window.owningApplication() }) else {
                continue;
            };
            let bundle_id = unsafe { app.bundleIdentifier() }.to_string();
            let app_name = unsafe { app.applicationName() }.to_string();
            if should_exclude_own_content(Some(&bundle_id), &app_name, own_bundle_id) {
                continue;
            }
            if !unsafe { window.isOnScreen() } {
                continue;
            }
            let frame = unsafe { window.frame() };
            if frame.size.width <= 0.0 || frame.size.height <= 0.0 {
                continue;
            }
            let title = unsafe { window.title() }
                .map(|s| s.to_string())
                .unwrap_or_default();
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

    #[test]
    fn bundle_id_mismatch_with_unrelated_name_is_not_excluded() {
        assert!(!should_exclude_own_content(
            Some("com.example.other"),
            "Other App",
            "com.snapper-keeper.app",
        ));
    }

    #[test]
    fn no_bundle_id_unrelated_name_is_not_excluded() {
        assert!(!should_exclude_own_content(
            None,
            "Safari",
            "com.snapper-keeper.app",
        ));
    }

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
}
