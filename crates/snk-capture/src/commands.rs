use std::time::Duration;

use snk_library::{Capture, LibraryState};
use tauri::{Emitter, Runtime, State};

use crate::grab::WindowInfo;
use crate::window_hider::{TauriWindowManager, WindowVisibilityGuard};
use crate::worker::CaptureWorker;
use crate::Result;

const HIDE_OWN_WINDOWS_KEY: &str = "capture.hide_own_windows";
/// The frontend owns the capture overlay lifecycle. Region crops operate on
/// the stored preview, so they do not hide windows or grab the desktop again.
const EXCLUDE_LABELS: &[&str] = &["capture-overlay"];
/// Delay between hiding our windows and grabbing pixels. Lets the
/// compositor unmap the windows before xcap reads the framebuffer.
/// 50ms left captured windows ghosting on real hardware — the hide
/// had returned but the OS compositor hadn't finished re-painting
/// the underlying content. 150ms matches the proven-reliable
/// compositor settling delay established during native capture testing.
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
/// Screen Recording permission, and trigger the system prompt so the app
/// is registered with TCC. Requires the app to run as a signed .app bundle
/// (`pnpm dev:mac-capture` for development). No-op on non-macOS.
fn require_screen_recording<R: Runtime>(_app: &tauri::AppHandle<R>) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Preserve the permission check and prompt's previous main-thread
        // context; only the blocking worker waits for the response.
        let (tx, rx) = std::sync::mpsc::channel();
        _app.run_on_main_thread(move || {
            let granted = crate::permissions::screen_recording_granted();
            if !granted {
                crate::permissions::request_screen_recording_access();
            }
            let _ = tx.send(granted);
        })
        .map_err(|e| crate::CaptureError::Os {
            message: format!("dispatch screen recording permission: {e}"),
        })?;
        let granted = rx.recv().map_err(|e| crate::CaptureError::Os {
            message: format!("screen recording permission response: {e}"),
        })?;
        if !granted {
            return Err(crate::CaptureError::ScreenRecordingPermissionDenied);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    worker: State<'_, CaptureWorker>,
    app: tauri::AppHandle<R>,
) -> Result<Capture> {
    let db = state.db.clone();
    let root = state.root.clone();
    worker
        .run(move || {
            require_screen_recording(&app)?;
            let capture = with_hidden_own_windows(&app, &db, || {
                crate::orchestrate::capture_full_screen(&db, &root)
            })?;
            let _ = app.emit("capture:saved", &capture.id);
            Ok(capture)
        })
        .await
}

#[tauri::command]
pub async fn capture_window<R: Runtime>(
    state: State<'_, LibraryState>,
    worker: State<'_, CaptureWorker>,
    app: tauri::AppHandle<R>,
    window_id: u32,
) -> Result<Capture> {
    let db = state.db.clone();
    let root = state.root.clone();
    worker
        .run(move || {
            require_screen_recording(&app)?;
            let capture = with_hidden_own_windows(&app, &db, || {
                crate::orchestrate::capture_window(&db, &root, window_id)
            })?;
            let _ = app.emit("capture:saved", &capture.id);
            Ok(capture)
        })
        .await
}

#[tauri::command]
// Keep the existing flat IPC arguments; the extra parameter is managed state.
#[allow(clippy::too_many_arguments)]
pub async fn capture_region<R: Runtime>(
    state: State<'_, LibraryState>,
    worker: State<'_, CaptureWorker>,
    app: tauri::AppHandle<R>,
    preview_token: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<Capture> {
    let db = state.db.clone();
    let root = state.root.clone();
    worker
        .run_with_preview(move |session| {
            let (pixels, foreground) = session.crop(&preview_token, x, y, w, h)?;
            let capture = crate::orchestrate::persist(
                &db,
                &root,
                &pixels.png_bytes,
                pixels.width,
                pixels.height,
                Some(pixels.monitor_name),
                foreground,
            )?;
            let _ = app.emit("capture:saved", &capture.id);
            Ok(capture)
        })
        .await
}

#[tauri::command]
pub async fn list_capturable_windows(worker: State<'_, CaptureWorker>) -> Result<Vec<WindowInfo>> {
    worker.run(crate::grab::list_capturable_windows).await
}

pub use crate::preview::ScreenPreview;

#[tauri::command]
pub async fn grab_screen_preview<R: Runtime>(
    state: State<'_, LibraryState>,
    worker: State<'_, CaptureWorker>,
    app: tauri::AppHandle<R>,
    monitor_id: Option<u32>,
) -> Result<ScreenPreview> {
    let db = state.db.clone();
    let root = state.root.clone();
    worker
        .run_with_preview(move |session| {
            require_screen_recording(&app)?;
            // Resolve identity, geometry and source before hiding any windows. The
            // same native monitor supplies the pixels; no frontend enumeration join.
            let monitor = crate::display::select_preview_monitor(monitor_id)?;
            let display = crate::display::describe(&monitor)?;
            let foreground = crate::foreground::get_foreground_info();
            let pixels = with_hidden_own_windows(&app, &db, || {
                let image = monitor.capture_image()?;
                Ok::<_, crate::CaptureError>(crate::grab::GrabResult {
                    png_bytes: crate::grab::encode_rgba_to_png(
                        image.as_raw(),
                        image.width(),
                        image.height(),
                    )?,
                    width: image.width(),
                    height: image.height(),
                    monitor_name: monitor.name().unwrap_or_default(),
                })
            })?;
            let path = snk_library::files::write_atomic(
                &root,
                std::path::Path::new("captures/.preview.png"),
                &pixels.png_bytes,
            )?;
            let preview = ScreenPreview {
                path: path.to_string_lossy().into_owned(),
                width: pixels.width,
                height: pixels.height,
                token: uuid::Uuid::now_v7().to_string(),
                display,
            };
            session.replace(preview.token.clone(), pixels, foreground);
            Ok(preview)
        })
        .await
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
