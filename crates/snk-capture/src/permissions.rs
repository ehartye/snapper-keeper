//! macOS Screen Recording (TCC) permission helpers for screen capture.
//!
//! ## Why TCC permission matters
//!
//! xcap uses CGWindowListCreateImage under the hood. Without the Screen
//! Recording TCC grant, that API silently returns black frames — no error,
//! no prompt — so captures appear blank and the region-overlay backdrop is
//! a solid black rectangle.
//!
//! ## Why CGPreflightScreenCaptureAccess requires a signed bundle
//!
//! TCC identifies apps by their code-signing identity. An unsigned binary
//! has no stable identity, so TCC cannot track a grant across launches and
//! CGPreflightScreenCaptureAccess() always returns false regardless of
//! what the user has toggled in System Settings.
//!
//! For capture validation, run the app as a properly signed .app bundle:
//!
//! ```text
//! pnpm dev:mac-capture
//! ```
//!
//! This builds a debug .app bundle, ad-hoc signs it with the stable bundle ID
//! `com.snapper-keeper.app`, and launches it via Launch Services.
//! Production builds are signed by the release pipeline and work correctly
//! without any extra steps.

use crate::Result;

/// Whether the process currently holds Screen Recording permission.
/// Returns true on non-macOS platforms (no TCC).
pub fn screen_recording_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Safe: CGPreflightScreenCaptureAccess takes no arguments, has no
        // side effects, and returns a Boolean (0/1). Requires a signed
        // binary with a stable identifier — see module doc.
        unsafe { cg::CGPreflightScreenCaptureAccess() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Trigger the macOS Screen Recording permission prompt so the app is
/// registered with TCC and appears in System Settings → Screen Recording.
/// No-op on non-macOS.
pub fn request_screen_recording_access() {
    #[cfg(target_os = "macos")]
    {
        // Safe: no arguments, no memory-safety concerns.
        // Return value intentionally ignored; use screen_recording_granted()
        // to re-check after the user acts.
        unsafe {
            cg::CGRequestScreenCaptureAccess();
        }
    }
}

/// Open System Settings → Privacy & Security → Screen Recording.
/// No-op off macOS.
pub fn open_screen_recording_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
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
        pub fn CGPreflightScreenCaptureAccess() -> c_uchar;
        pub fn CGRequestScreenCaptureAccess() -> c_uchar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_recording_granted_returns_bool() {
        // Just verify it doesn't panic. On CI (Linux) this always returns
        // true; on macOS run via `pnpm dev:mac-capture` (bundled app) for
        // a stable TCC identity that makes this return the real grant state.
        let _ = screen_recording_granted();
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_always_granted() {
        assert!(screen_recording_granted());
    }
}
