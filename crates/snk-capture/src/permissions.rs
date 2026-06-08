//! macOS Screen Recording (TCC) helpers for screen capture.
//!
//! On macOS Sequoia and later, `CGPreflightScreenCaptureAccess()` is
//! unreliable for unsigned / ad-hoc-signed binaries (debug builds) —
//! it always returns false even after the user has granted permission.
//! xcap uses ScreenCaptureKit, which handles the TCC permission prompt
//! natively when capture is first attempted.
//!
//! Our role here is limited to:
//!   1. Calling `CGRequestScreenCaptureAccess()` at plugin init so the
//!      app is registered with TCC and the system prompt appears on first
//!      launch, rather than silently returning black frames.
//!   2. Providing a deep-link to the Settings pane for the manual fallback.
//!
//! Off macOS, everything is a no-op.

use crate::Result;

/// Trigger the macOS Screen Recording permission prompt.
/// Should be called once at plugin init to register the app with TCC.
/// No-op on non-macOS.
pub fn request_screen_recording_access() {
    #[cfg(target_os = "macos")]
    {
        // Safety: CGRequestScreenCaptureAccess takes no arguments and has no
        // memory-safety concerns. Return value is intentionally ignored.
        unsafe {
            cg::CGRequestScreenCaptureAccess();
        }
    }
}

/// Open the OS settings pane where the user grants Screen Recording
/// permission. No-op off macOS.
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

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGRequestScreenCaptureAccess() -> c_uchar;
    }
}

