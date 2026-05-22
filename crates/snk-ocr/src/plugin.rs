use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Listener, Manager, Runtime};

use snk_library::plugin::LibraryState;

use crate::queue::OcrQueue;

pub struct OcrState {
    pub queue: OcrQueue,
}

#[tauri::command]
pub fn ocr_status<R: Runtime>(_app: tauri::AppHandle<R>) -> Result<String, String> {
    Ok("running".to_string())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-ocr")
        .invoke_handler(tauri::generate_handler![ocr_status])
        .setup(|app, _api| {
            let lib_state = app.state::<LibraryState>();
            let db = lib_state.db.clone();
            let root = lib_state.root.clone();

            // Tell the sidecar where the bundled tesseract distribution lives,
            // so a packaged build can use it without requiring a system install.
            match app.path().resource_dir() {
                Ok(dir) => crate::sidecar::set_bundled_resource_dir(dir),
                Err(e) => tracing::debug!(error = %e, "no resource dir; falling back to system tesseract"),
            }

            let queue = OcrQueue::start(Arc::clone(&db), root);
            app.manage(OcrState { queue });

            // Re-bind only what the listener closure captures.
            let db_for_listener = Arc::clone(&db);
            let app_handle = app.app_handle().clone();
            // Clone the receiver so the original can move into the closure for try_state.
            app_handle.clone().listen("capture:saved", move |event| {
                let capture_id = event.payload().trim_matches('"').to_string();
                if capture_id.is_empty() {
                    return;
                }

                match snk_library::captures::get(&db_for_listener, &capture_id) {
                    Ok(capture) => {
                        let image_path = std::path::PathBuf::from(&capture.file_path);
                        let language = "eng".to_string();
                        if let Some(ocr) = app_handle.try_state::<OcrState>() {
                            ocr.queue.enqueue(capture_id, image_path, language);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(capture_id, error = %e, "could not look up capture for ocr");
                    }
                }
            });

            Ok(())
        })
        .build()
}
