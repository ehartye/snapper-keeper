//! macOS Screen Recording (TCC) permission checks for screen capture.
//!
//! xcap calls CGDisplayCreateImage/CGWindowListCreateImage under the hood.
//! Without the Screen Recording grant, those APIs return black frames — no
//! error, no prompt — so captures appear blank and the region overlay shows
//! a solid black backdrop.  We check up-front with
//! CGPreflightScreenCaptureAccess and surface a typed error the UI can act
//! on, plus a deep-link to the System Settings pane.
//!
//! Off macOS, screen capture needs no such grant, so everything here reports
//! "granted" and the settings deep-link is a no-op.

use serde::Serialize;
use ts_rs::TS;

use crate::Result;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(
    export,
    export_to = "../../../packages/snk-capture/src/generated/permission-status.ts"
)]
pub struct CapturePermissionStatus {
    pub screen_recording_granted: bool,
}

/// Whether the process currently holds Screen Recording permission.
/// macOS gates this behind the TCC grant; other platforms always allow it.
pub fn screen_recording_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Safety: CGPreflightScreenCaptureAccess takes no arguments, has no
        // side effects, and returns a Boolean (0/1).
        unsafe { cg::CGPreflightScreenCaptureAccess() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Trigger the macOS Screen Recording permission prompt / System Settings
/// deep-link.  No-op on non-macOS.
pub fn request_screen_recording_access() {
    #[cfg(target_os = "macos")]
    {
        // Safety: CGRequestScreenCaptureAccess takes no arguments and has no
        // memory-safety concerns.  Return value is intentionally ignored; we
        // use screen_recording_granted() to re-check after the user acts.
        unsafe {
            cg::CGRequestScreenCaptureAccess();
        }
    }
}

pub fn status() -> CapturePermissionStatus {
    CapturePermissionStatus {
        screen_recording_granted: screen_recording_granted(),
    }
}

/// Open the OS settings pane where the user grants Screen Recording
/// permission.  No-op off macOS.
pub fn open_screen_recording_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            )
            .spawn()
            .map_err(|e| crate::CaptureError::Os {
                message: format!("open screen recording settings: {e}"),
            })?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod cg {
    use std::os::raw::c_uchar;

    // CGPreflightScreenCaptureAccess / CGRequestScreenCaptureAccess live in
    // the CoreGraphics framework (available since macOS 10.15).
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGPreflightScreenCaptureAccess() -> c_uchar;
        pub fn CGRequestScreenCaptureAccess() -> c_uchar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_matches_screen_recording_granted() {
        assert_eq!(status().screen_recording_granted, screen_recording_granted());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_always_granted() {
        assert!(screen_recording_granted());
    }
}
