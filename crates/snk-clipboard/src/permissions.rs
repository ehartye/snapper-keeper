//! macOS Accessibility (TCC) permission checks for synthetic paste.
//!
//! Driving Cmd+V via `CGEvent` (see `paste.rs`) requires the process to hold
//! the macOS Accessibility permission. Without it the OS silently drops the
//! synthetic event — no error, no prompt — so auto-paste appears dead. We
//! check up-front and surface a typed error the popup can act on, plus a
//! deep-link to the System Settings pane where the user grants it.
//!
//! Off macOS, synthetic paste needs no such grant, so everything here reports
//! "granted" and the settings deep-link is a no-op.

use serde::Serialize;
use ts_rs::TS;

use crate::Result;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(
    export,
    export_to = "../../../packages/snk-clipboard/src/generated/permission-status.ts"
)]
pub struct PermissionStatus {
    pub accessibility_granted: bool,
}

/// Whether synthetic paste is permitted right now. macOS gates this behind the
/// Accessibility (TCC) grant; other platforms always allow it.
pub fn accessibility_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Safety: `AXIsProcessTrusted` takes no arguments, has no side effects,
        // and returns a `Boolean` (0/1). Always sound to call.
        unsafe { ax::AXIsProcessTrusted() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn status() -> PermissionStatus {
    PermissionStatus {
        accessibility_granted: accessibility_granted(),
    }
}

/// Open the OS settings pane where the user grants the permission auto-paste
/// needs. No-op off macOS.
pub fn open_accessibility_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Absolute path rather than relying on PATH, which a packaged app
        // can't assume and which the environment could influence.
        std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| crate::ClipboardError::Access {
                message: format!("open accessibility settings: {e}"),
            })?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod ax {
    use std::os::raw::c_uchar;

    // `AXIsProcessTrusted` lives in the HIServices framework, re-exported by
    // the ApplicationServices umbrella.
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        pub fn AXIsProcessTrusted() -> c_uchar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_matches_accessibility_granted() {
        assert_eq!(status().accessibility_granted, accessibility_granted());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_always_granted() {
        assert!(accessibility_granted());
    }
}
