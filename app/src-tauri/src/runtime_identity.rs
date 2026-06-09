//! Runtime classifier: distinguishes bundled-app runs from raw dev runs.
//!
//! On macOS, `tauri dev` runs the unpackaged binary from the cargo target
//! directory. Screen Recording permission is granted per bundle ID, so an
//! unpackaged binary will be denied even if the bundled app has been approved.
//! This classifier lets the UI surface actionable guidance in that case.

use serde::Serialize;

/// Whether the app is running from a bundled installer or directly from
/// the cargo build output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureRuntimeStatus {
    /// Running inside an `.app` bundle (macOS) or equivalent platform
    /// installer package.
    BundledApp,
    /// Running directly from the cargo build output (e.g. `tauri dev`
    /// from a terminal). Screen Recording and similar OS permissions
    /// may not be granted for this unpackaged binary.
    RawDev,
}

/// Classify the runtime from `exe_path`.
///
/// Separated from [`classify_capture_runtime`] so tests can inject specific
/// path shapes without depending on the real process executable path.
pub fn classify_from_path(exe_path: &std::path::Path) -> CaptureRuntimeStatus {
    if cfg!(target_os = "macos") && exe_path.to_string_lossy().contains(".app/Contents/MacOS") {
        CaptureRuntimeStatus::BundledApp
    } else {
        CaptureRuntimeStatus::RawDev
    }
}

/// Return the runtime classification for the current process.
pub fn classify_capture_runtime() -> CaptureRuntimeStatus {
    let exe = std::env::current_exe().unwrap_or_default();
    classify_from_path(&exe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(target_os = "macos")]
    #[test]
    fn bundled_app_path_is_classified_correctly() {
        let path = Path::new("/Applications/Snapper Keeper.app/Contents/MacOS/snapper-keeper-app");
        assert_eq!(classify_from_path(path), CaptureRuntimeStatus::BundledApp);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn raw_dev_path_is_classified_correctly() {
        let path = Path::new("/Users/user/repos/snapper-keeper/target/debug/snapper-keeper-app");
        assert_eq!(classify_from_path(path), CaptureRuntimeStatus::RawDev);
    }
}
