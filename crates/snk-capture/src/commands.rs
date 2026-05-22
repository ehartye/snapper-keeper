use snk_library::{plugin::LibraryState, Capture};
use tauri::{Emitter, Manager, Runtime, State};

use crate::grab::WindowInfo;
use crate::Result;

#[tauri::command]
pub fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
) -> Result<Capture> {
    let capture = crate::orchestrate::capture_full_screen(&state.db, &state.root)?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn capture_window<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    window_id: u32,
) -> Result<Capture> {
    let capture = crate::orchestrate::capture_window(&state.db, &state.root, window_id)?;
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
    let capture =
        crate::orchestrate::capture_region(&state.db, &state.root, monitor_id, x, y, w, h)?;
    let _ = app.emit("capture:saved", &capture.id);
    Ok(capture)
}

#[tauri::command]
pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    crate::grab::list_capturable_windows()
}

#[derive(serde::Serialize)]
pub struct ScreenPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn grab_screen_preview<R: Runtime>(app: tauri::AppHandle<R>) -> Result<ScreenPreview> {
    let result = crate::grab::grab_primary_monitor()?;
    let dir = app.path().app_data_dir().map_err(|e| crate::CaptureError::Os {
        message: format!("app data dir: {e}"),
    })?;
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
    })
}
