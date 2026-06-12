use super::ScreenshotBackend;
use crate::grab::{GrabResult, WindowInfo};
use crate::Result;

pub struct ScreenCaptureKitBackend;

/// Returns true when the given app should be excluded from capture output
/// because it belongs to Snapper Keeper itself.
///
/// Matching priority:
/// 1. Bundle ID exact match against `own_bundle_id`.
/// 2. App name contains "snapper-keeper" (case-insensitive) — covers helper
///    processes and sub-bundles that share the same prefix but may report a
///    different bundle ID.
///
/// Called by `list_capturable_windows` and capture methods once SCK
/// enumeration is added in task 4. Marked `allow(dead_code)` while the
/// scaffold stubs are in place.
#[allow(dead_code)]
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

/// Returns the bundle identifier for the running Snapper Keeper app.
///
/// In a properly signed bundled app this is always `com.snapper-keeper.app`.
/// Hard-coded here so the exclusion logic works without an Objective-C
/// runtime call in unit tests; the SCK integration in task 4 will keep
/// using this value when constructing content filters.
fn current_bundle_id() -> Result<String> {
    Ok("com.snapper-keeper.app".to_string())
}

impl ScreenshotBackend for ScreenCaptureKitBackend {
    fn grab_primary_monitor(&self) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not yet implemented".into(),
        })
    }

    fn grab_monitor(&self, _monitor_id: u32) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not yet implemented".into(),
        })
    }

    fn grab_window(&self, _window_id: u32) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not yet implemented".into(),
        })
    }

    fn grab_region(
        &self,
        _monitor_id: u32,
        _x: u32,
        _y: u32,
        _w: u32,
        _h: u32,
    ) -> Result<GrabResult> {
        Err(crate::CaptureError::Os {
            message: "ScreenCaptureKit backend not yet implemented".into(),
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
            "com.snapper-keeper.app"
        ));
    }

    #[test]
    fn list_capturable_windows_returns_empty_vec_from_scaffold() {
        let backend = ScreenCaptureKitBackend;
        let windows = backend.list_capturable_windows().unwrap();
        assert!(windows.is_empty());
    }
}
