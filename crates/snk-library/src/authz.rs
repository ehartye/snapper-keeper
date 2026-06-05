//! Defense-in-depth authorization + audit logging for destructive IPC
//! commands.
//!
//! Per-window capabilities (`app/src-tauri/capabilities/*.json`) are the first
//! gate — Tauri's ACL rejects a command the calling window was never granted.
//! This module re-asserts the caller window *inside* the handler, so a future
//! capability misconfiguration can't silently widen access, and writes every
//! destructive attempt — allowed or denied — to the security-events.log.
//!
//! The allowed-window sets below MIRROR the capability grants. Keep them in
//! sync with `app/src-tauri/capabilities/*.json`.

use tauri::{Runtime, WebviewWindow};

/// Tracing target routing events into `security-events.log`. MUST stay in sync
/// with `SECURITY_LOG_TARGET` in `app/src-tauri/src/logging.rs` (the plugins
/// can't import the app binary, so the string is duplicated by necessity).
pub const SECURITY_LOG_TARGET: &str = "snk::security";

pub const HARD_DELETE_CAPTURE_WINDOWS: &[&str] = &["library"];
pub const PURGE_TRASH_WINDOWS: &[&str] = &["library"];
pub const SET_SETTING_WINDOWS: &[&str] = &["library", "settings"];
pub const PASTE_ITEM_WINDOWS: &[&str] = &["clipboard-popup"];

/// Returned when a window invokes a destructive command it is not allowed to.
/// Each crate converts this into its own typed `Unauthorized` error variant
/// via a `From` impl, so the denial crosses the IPC boundary as a typed error.
#[derive(Debug, Clone)]
pub struct AuthzDenied {
    pub window: String,
    pub command: String,
}

/// Pure authorization decision, split out so it can be unit-tested without a
/// live Tauri window.
pub fn is_authorized(label: &str, allowed: &[&str]) -> bool {
    allowed.contains(&label)
}

/// Authorize `window` to invoke `command`, auditing the attempt either way.
///
/// `arg_summary` must be a NON-sensitive identifier (capture id, clipboard
/// item id, setting key) — never raw clipboard / OCR / setting *values*, which
/// must never reach the log (PRIVACY.md guarantees the log folder contains no
/// clipboard/OCR text).
pub fn authorize<R: Runtime>(
    window: &WebviewWindow<R>,
    command: &str,
    allowed: &[&str],
    arg_summary: &str,
) -> Result<(), AuthzDenied> {
    let label = window.label();
    if is_authorized(label, allowed) {
        tracing::info!(
            target: SECURITY_LOG_TARGET,
            event = "destructive_invoked",
            window = label,
            command = command,
            arg = arg_summary,
            "destructive command authorized"
        );
        Ok(())
    } else {
        tracing::warn!(
            target: SECURITY_LOG_TARGET,
            event = "destructive_denied",
            window = label,
            command = command,
            arg = arg_summary,
            allowed = ?allowed,
            "destructive command denied: caller window not in allowed set"
        );
        Err(AuthzDenied {
            window: label.to_string(),
            command: command.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_window_authorized_for_hard_delete_and_purge() {
        assert!(is_authorized("library", HARD_DELETE_CAPTURE_WINDOWS));
        assert!(is_authorized("library", PURGE_TRASH_WINDOWS));
    }

    #[test]
    fn popup_and_overlay_denied_for_capture_destruction() {
        for win in ["clipboard-popup", "capture-overlay", "annotate"] {
            assert!(!is_authorized(win, HARD_DELETE_CAPTURE_WINDOWS));
            assert!(!is_authorized(win, PURGE_TRASH_WINDOWS));
        }
    }

    #[test]
    fn set_setting_allows_library_and_settings_only() {
        assert!(is_authorized("library", SET_SETTING_WINDOWS));
        assert!(is_authorized("settings", SET_SETTING_WINDOWS));
        assert!(!is_authorized("clipboard-popup", SET_SETTING_WINDOWS));
        assert!(!is_authorized("capture-overlay", SET_SETTING_WINDOWS));
    }

    #[test]
    fn paste_item_allows_only_popup() {
        assert!(is_authorized("clipboard-popup", PASTE_ITEM_WINDOWS));
        assert!(!is_authorized("library", PASTE_ITEM_WINDOWS));
        assert!(!is_authorized("settings", PASTE_ITEM_WINDOWS));
    }

    #[test]
    fn unknown_window_is_denied_everywhere() {
        for allowed in [
            HARD_DELETE_CAPTURE_WINDOWS,
            PURGE_TRASH_WINDOWS,
            SET_SETTING_WINDOWS,
            PASTE_ITEM_WINDOWS,
        ] {
            assert!(!is_authorized("totally-unknown", allowed));
            assert!(!is_authorized("", allowed));
        }
    }
}
