use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use image::imageops::crop_imm;
use snk_library::{plugin::LibraryState, Capture};
use tauri::{Emitter, Manager, Runtime, State};

use crate::grab::{GrabResult, WindowInfo};
use crate::window_hider::{TauriWindowManager, WindowVisibilityGuard};
use crate::{CaptureError, Result};
#[cfg(target_os = "macos")]
use objc2_core_graphics::CGEvent;

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

#[derive(Default)]
pub struct PreviewSessionState {
    #[cfg(target_os = "macos")]
    session: Mutex<Option<PreviewSession>>,
    #[cfg(not(target_os = "macos"))]
    session: Mutex<Option<()>>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRegionRequest {
    monitor_id: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    scale_factor: f64,
    preview_token: Option<String>,
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct PreviewSession {
    token: String,
    png_bytes: Vec<u8>,
    monitor_name: String,
}

#[cfg(target_os = "macos")]
fn lock_preview_session_state<'a>(
    preview_state: &'a PreviewSessionState,
) -> Result<MutexGuard<'a, Option<PreviewSession>>> {
    preview_state.session.lock().map_err(|_| CaptureError::Os {
        message: "preview session lock poisoned".into(),
    })
}

#[cfg(target_os = "macos")]
fn store_preview_session(
    preview_state: &PreviewSessionState,
    token: &str,
    result: &GrabResult,
) -> Result<()> {
    let mut session = lock_preview_session_state(preview_state)?;
    *session = Some(PreviewSession {
        token: token.to_string(),
        png_bytes: result.png_bytes.clone(),
        monitor_name: result.monitor_name.clone(),
    });
    Ok(())
}

#[cfg(target_os = "macos")]
fn preview_session_for_token(
    preview_state: &PreviewSessionState,
    token: &str,
) -> Result<PreviewSession> {
    let session = lock_preview_session_state(preview_state)?;
    session
        .as_ref()
        .filter(|session| session.token == token)
        .cloned()
        .ok_or_else(|| CaptureError::Os {
            message: "region preview expired; start region capture again".into(),
        })
}

#[cfg(target_os = "macos")]
fn clear_preview_session(preview_state: &PreviewSessionState, token: &str) -> Result<()> {
    let mut session = lock_preview_session_state(preview_state)?;
    if session
        .as_ref()
        .is_some_and(|current| current.token == token)
    {
        *session = None;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn crop_preview_png(
    png_bytes: &[u8],
    monitor_name: &str,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    scale_factor: f64,
) -> Result<GrabResult> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(CaptureError::Os {
            message: format!("invalid preview scale factor: {scale_factor}"),
        });
    }

    let preview = image::load_from_memory(png_bytes)
        .map_err(|e| CaptureError::Os {
            message: format!("decode preview: {e}"),
        })?
        .to_rgba8();
    let scaled_x = (x as f64 * scale_factor).round() as u32;
    let scaled_y = (y as f64 * scale_factor).round() as u32;
    let scaled_w = (w as f64 * scale_factor).round() as u32;
    let scaled_h = (h as f64 * scale_factor).round() as u32;
    let (scaled_x, scaled_y, scaled_w, scaled_h) = crate::grab::clamp_region(
        preview.width(),
        preview.height(),
        scaled_x,
        scaled_y,
        scaled_w,
        scaled_h,
    )
    .ok_or_else(|| CaptureError::Os {
        message: "region has zero area".into(),
    })?;
    let cropped = crop_imm(&preview, scaled_x, scaled_y, scaled_w, scaled_h).to_image();
    let png_bytes = crate::grab::encode_rgba_to_png(cropped.as_raw(), scaled_w, scaled_h)?;
    Ok(GrabResult {
        png_bytes,
        width: scaled_w,
        height: scaled_h,
        monitor_name: monitor_name.to_string(),
        display_frame: None,
        display_index: None,
    })
}

#[cfg(target_os = "macos")]
fn capture_region_from_preview_session(
    preview_state: &PreviewSessionState,
    preview_token: &str,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    scale_factor: f64,
) -> Result<GrabResult> {
    let session = preview_session_for_token(preview_state, preview_token)?;
    let result = crop_preview_png(
        &session.png_bytes,
        &session.monitor_name,
        x,
        y,
        w,
        h,
        scale_factor,
    )?;
    clear_preview_session(preview_state, preview_token)?;
    Ok(result)
}

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
/// Screen Recording permission, and trigger the system prompt so the app
/// is registered with TCC. Requires the app to run as a signed .app bundle
/// (`pnpm dev:mac-capture` for development). No-op on non-macOS.
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
    preview_state: State<'_, PreviewSessionState>,
    app: tauri::AppHandle<R>,
    request: CaptureRegionRequest,
) -> Result<Capture> {
    require_screen_recording()?;
    let CaptureRegionRequest {
        monitor_id,
        x,
        y,
        w,
        h,
        scale_factor,
        preview_token,
    } = request;
    #[cfg(target_os = "macos")]
    {
        let _ = monitor_id;
        let preview_token = preview_token.ok_or_else(|| CaptureError::Os {
            message: "region preview token missing".into(),
        })?;
        let fg = crate::foreground::get_foreground_info();
        let result = capture_region_from_preview_session(
            &preview_state,
            &preview_token,
            x,
            y,
            w,
            h,
            scale_factor,
        )?;
        let capture = crate::orchestrate::persist(
            &state.db,
            &state.root,
            &result.png_bytes,
            result.width,
            result.height,
            Some(result.monitor_name),
            fg,
        )?;
        let _ = app.emit("capture:saved", &capture.id);
        Ok(capture)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (&preview_state, &preview_token);
        let (x, y, w, h) = (
            (x as f64 * scale_factor).round() as u32,
            (y as f64 * scale_factor).round() as u32,
            (w as f64 * scale_factor).round() as u32,
            (h as f64 * scale_factor).round() as u32,
        );
        let capture = with_hidden_own_windows(&app, &state.db, || {
            crate::orchestrate::capture_region(&state.db, &state.root, monitor_id, x, y, w, h)
        })?;
        let _ = app.emit("capture:saved", &capture.id);
        Ok(capture)
    }
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
#[serde(rename_all = "camelCase")]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub token: String,
    pub display_frame: Option<crate::grab::DisplayFrame>,
    pub display_index: Option<usize>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

#[tauri::command]
pub fn grab_screen_preview<R: Runtime>(
    state: State<'_, LibraryState>,
    preview_state: State<'_, PreviewSessionState>,
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
    // assetProtocol allow scope (`$APPDATA/captures/**`). Keep the file
    // non-hidden: the overlay preview is loaded through the asset
    // protocol, and the hidden `.preview.png` path regressed to a broken
    // `<img>` (black backdrop + question-mark placeholder) in the
    // bundled macOS runtime even though the PNG was written correctly.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| crate::CaptureError::Os {
            message: format!("app data dir: {e}"),
        })?
        .join("captures");
    let preview_path = dir.join("preview.png");
    std::fs::create_dir_all(&dir).map_err(|e| crate::CaptureError::Os {
        message: format!("create dir: {e}"),
    })?;
    std::fs::write(&preview_path, &result.png_bytes).map_err(|e| crate::CaptureError::Os {
        message: format!("write preview: {e}"),
    })?;
    let token = mint_preview_token();
    #[cfg(target_os = "macos")]
    store_preview_session(&preview_state, &token, &result)?;
    #[cfg(not(target_os = "macos"))]
    let _ = &preview_state;
    Ok(ScreenPreview {
        path: preview_path.to_string_lossy().into_owned(),
        width: result.width,
        height: result.height,
        token,
        display_frame: result.display_frame.clone(),
        display_index: result.display_index,
    })
}

#[tauri::command]
pub fn capture_cursor_position() -> Result<CursorPosition> {
    #[cfg(target_os = "macos")]
    {
        let event = CGEvent::new(None).ok_or_else(|| CaptureError::Os {
            message: "failed to read cursor position".into(),
        })?;
        let loc = CGEvent::location(Some(event.as_ref()));
        Ok(CursorPosition {
            x: loc.x.round() as i32,
            y: loc.y.round() as i32,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(CaptureError::Os {
            message: "native capture cursor position is only supported on macOS".into(),
        })
    }
}

#[tauri::command]
pub fn capture_permission_status() -> bool {
    crate::permissions::screen_recording_granted()
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

    #[test]
    fn crop_preview_png_uses_scale_factor_before_clamping() {
        let mut rgba = Vec::new();
        for y in 0..4u8 {
            for x in 0..4u8 {
                rgba.extend_from_slice(&[x * 10, y * 20, 0, 255]);
            }
        }
        let png = crate::grab::encode_rgba_to_png(&rgba, 4, 4).unwrap();

        let cropped = crop_preview_png(&png, "display-under-test", 1, 0, 1, 1, 2.0).unwrap();
        assert_eq!(cropped.monitor_name, "display-under-test");
        assert_eq!((cropped.width, cropped.height), (2, 2));

        let img = image::load_from_memory(&cropped.png_bytes)
            .unwrap()
            .to_rgba8();
        assert_eq!(img.get_pixel(0, 0).0, [20, 0, 0, 255]);
        assert_eq!(img.get_pixel(1, 1).0, [30, 20, 0, 255]);
    }

    #[test]
    fn crop_preview_png_rejects_non_positive_scale_factor() {
        let png = crate::grab::encode_rgba_to_png(&[255, 0, 0, 255], 1, 1).unwrap();
        let err = crop_preview_png(&png, "display-under-test", 0, 0, 1, 1, 0.0)
            .err()
            .unwrap();
        assert!(err.to_string().contains("scale factor"));
    }
}
