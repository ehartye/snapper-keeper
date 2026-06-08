use std::time::Duration;

use snk_library::{plugin::LibraryState, Capture};
use tauri::{Emitter, Manager, Runtime, State};

use crate::grab::WindowInfo;
use crate::permissions::CapturePermissionStatus;
use crate::window_hider::{TauriWindowManager, WindowVisibilityGuard};
use crate::Result;

const HIDE_OWN_WINDOWS_KEY: &str = "capture.hide_own_windows";
/// Labels excluded from the visibility guard. The capture overlay
/// is already hidden by the React frontend before invoking
/// capture_region (see CaptureOverlay.tsx); we exclude it to avoid
/// racing the frontend's existing hide.
const EXCLUDE_LABELS: &[&str] = &["capture-overlay"];
/// Delay between hiding our windows and grabbing pixels. Lets the
/// compositor unmap the windows before xcap reads the framebuffer.
/// 50ms left captured windows ghosting on real hardware — the hide
/// had returned but the OS compositor hadn't finished re-painting
/// the underlying content. 150ms matches the proven-reliable
/// self-hide delay the overlay uses in `CaptureOverlay.tsx` and
/// stops the ghost reliably.
const HIDE_SETTLE_DELAY: Duration = Duration::from_millis(150);

fn should_hide_own_windows(db: &snk_library::Db) -> bool {
    snk_library::settings::get(db, HIDE_OWN_WINDOWS_KEY)
        .ok()
        .flatten()
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Run `f` with our own windows hidden if the setting is enabled. The
/// guard restores visibility on drop, so any panic/error in `f` still
/// leaves the user's windows back up. When the setting is false, `f`
/// runs unmodified.
fn with_hidden_own_windows<R: Runtime, T, F>(
    app: &tauri::AppHandle<R>,
    db: &snk_library::Db,
    f: F,
) -> T
where
    F: FnOnce() -> T,
{
    if !should_hide_own_windows(db) {
        return f();
    }
    let manager = TauriWindowManager::new(app);
    let _guard = WindowVisibilityGuard::hide_all(&manager, EXCLUDE_LABELS);
    std::thread::sleep(HIDE_SETTLE_DELAY);
    f()
}

/// Return `Err(ScreenRecordingPermissionDenied)` if the OS has not granted
/// Screen Recording permission, and trigger the system prompt so the user
/// can navigate to Settings.  On non-macOS this is always `Ok(())`.
fn require_screen_recording() -> Result<()> {
    if !crate::permissions::screen_recording_granted() {
        crate::permissions::request_screen_recording_access();
        return Err(crate::CaptureError::ScreenRecordingPermissionDenied);
    }
    Ok(())
}

#[tauri::command]
pub fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
) -> Result<Capture> {
    require_screen_recording()?;
    let capture = with_hidden_own_windows(&app, &state.db, || {
        crate::orchestrate::capture_full_screen(&state.db, &state.root)
    })?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn capture_window<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    window_id: u32,
) -> Result<Capture> {
    require_screen_recording()?;
    let capture = with_hidden_own_windows(&app, &state.db, || {
        crate::orchestrate::capture_window(&state.db, &state.root, window_id)
    })?;
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
    require_screen_recording()?;
    let capture = with_hidden_own_windows(&app, &state.db, || {
        crate::orchestrate::capture_region(&state.db, &state.root, monitor_id, x, y, w, h)
    })?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    crate::grab::list_capturable_windows()
}

/// Mint a fresh cache-busting token for one preview write.
/// UUIDv7 is monotonic so two consecutive calls always differ.
fn mint_preview_token() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(serde::Serialize)]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub token: String,
}

#[tauri::command]
pub fn grab_screen_preview<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    monitor_id: Option<u32>,
) -> Result<ScreenPreview> {
    require_screen_recording()?;
    // Hide own windows around the grab — same pattern as the three
    // capture commands above. Without this the preview backdrop the
    // region overlay shows would include the library/settings/etc.,
    // making it impossible to draw a region over content underneath.
    let result = with_hidden_own_windows(&app, &state.db, || {
        if let Some(monitor_id) = monitor_id {
            crate::grab::grab_monitor(monitor_id)
        } else {
            crate::grab::grab_primary_monitor()
        }
    })?;
    // Preview file lives under `captures/` so it falls inside the
    // assetProtocol allow scope (`$APPDATA/captures/**`). Tightening the
    // scope in #84 broke the previous root-of-app-data location: the
    // overlay backdrop's `convertFileSrc(.preview.png)` URL failed CSP/
    // scope checks and fell through to a solid black background, which
    // visually presented as "overlay blocks the images" / blank capture.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| crate::CaptureError::Os {
            message: format!("app data dir: {e}"),
        })?
        .join("captures");
    let preview_path = dir.join(".preview.png");
    std::fs::create_dir_all(&dir).map_err(|e| crate::CaptureError::Os {
        message: format!("create dir: {e}"),
    })?;
    std::fs::write(&preview_path, &result.png_bytes).map_err(|e| crate::CaptureError::Os {
        message: format!("write preview: {e}"),
    })?;
    Ok(ScreenPreview {
        path: preview_path.to_string_lossy().into_owned(),
        width: result.width,
        height: result.height,
        token: mint_preview_token(),
    })
}

#[tauri::command]
pub fn capture_permission_status() -> CapturePermissionStatus {
    crate::permissions::status()
}

#[tauri::command]
pub fn open_screen_recording_settings() -> Result<()> {
    crate::permissions::open_screen_recording_settings()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_preview_token_yields_unique_strings() {
        let a = mint_preview_token();
        let b = mint_preview_token();
        assert_ne!(a, b, "two calls must return different tokens");
        assert!(!a.is_empty());
        assert!(!b.is_empty());
    }

    #[test]
    fn should_hide_own_windows_defaults_to_true_when_setting_missing() {
        let dir = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&dir.path().join("test.db")).unwrap();
        assert!(should_hide_own_windows(&db));
    }

    #[test]
    fn should_hide_own_windows_reads_false_when_setting_false() {
        let dir = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&dir.path().join("test.db")).unwrap();
        snk_library::settings::set(&db, HIDE_OWN_WINDOWS_KEY, &serde_json::Value::Bool(false))
            .unwrap();
        assert!(!should_hide_own_windows(&db));
    }

    #[test]
    fn should_hide_own_windows_reads_true_when_setting_true() {
        let dir = tempfile::tempdir().unwrap();
        let db = snk_library::Db::open(&dir.path().join("test.db")).unwrap();
        snk_library::settings::set(&db, HIDE_OWN_WINDOWS_KEY, &serde_json::Value::Bool(true))
            .unwrap();
        assert!(should_hide_own_windows(&db));
    }
}
