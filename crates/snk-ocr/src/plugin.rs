use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};

use snk_library::ocr::OcrWord;
use snk_library::LibraryState;

use crate::backend::OcrBackend;
use crate::queue::OcrQueue;
use crate::{OcrError, Result};

pub struct OcrState {
    pub queue: OcrQueue,
    pub backend_name: &'static str,
    pub backend_version: String,
    pub last_error: std::sync::Arc<std::sync::Mutex<Option<OcrError>>>,
}

#[tauri::command]
pub fn ocr_status<R: Runtime>(app: tauri::AppHandle<R>) -> Result<serde_json::Value> {
    let state = app
        .try_state::<OcrState>()
        .ok_or_else(|| OcrError::StateUnavailable {
            reason: "ocr state missing".into(),
        })?;
    let last_err = state
        .last_error
        .lock()
        .map_err(|e| OcrError::StateUnavailable {
            reason: e.to_string(),
        })?;
    Ok(serde_json::json!({
        "backend": state.backend_name,
        "version": state.backend_version,
        "last_error": last_err.as_ref().map(|e| serde_json::to_value(e).unwrap_or_default()),
    }))
}

#[tauri::command]
pub fn get_ocr_words<R: Runtime>(
    app: tauri::AppHandle<R>,
    capture_id: String,
) -> Result<Vec<OcrWord>> {
    let lib = app.state::<LibraryState>();
    let row = snk_library::ocr::get(&lib.db, &capture_id).map_err(|e| OcrError::Library {
        detail: e.to_string(),
    })?;
    Ok(row.and_then(|r| r.words).unwrap_or_default())
}

fn build_backend() -> Result<Arc<dyn OcrBackend>> {
    #[cfg(target_os = "macos")]
    {
        let b = crate::vision::VisionBackend::new()?;
        Ok(Arc::new(b) as Arc<dyn OcrBackend>)
    }
    #[cfg(target_os = "windows")]
    {
        let b = crate::winocr::WinOcrBackend::new()?;
        Ok(Arc::new(b) as Arc<dyn OcrBackend>)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(OcrError::BackendUnavailable {
            reason: "no OCR backend available on this platform".into(),
        })
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-ocr")
        .invoke_handler(tauri::generate_handler![ocr_status, get_ocr_words])
        .setup(|app, _api| {
            let lib_state = app.state::<LibraryState>();
            let db = lib_state.db.clone();
            let root = lib_state.root.clone();

            let app_handle_for_emit = app.app_handle().clone();
            let emit_ready: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |capture_id: &str| {
                if let Err(e) = app_handle_for_emit.emit("ocr:ready", capture_id) {
                    tracing::warn!(error = %e, "failed to emit ocr:ready");
                }
            });

            match build_backend() {
                Ok(backend) => {
                    let backend_name = backend.name();
                    let backend_version = backend.engine_version();
                    tracing::info!(backend = backend_name, version = %backend_version, "ocr backend ready");

                    let last_error: std::sync::Arc<std::sync::Mutex<Option<OcrError>>> =
                        std::sync::Arc::new(std::sync::Mutex::new(None));
                    let last_error_for_cb = std::sync::Arc::clone(&last_error);
                    let on_error: Arc<dyn Fn(OcrError) + Send + Sync> =
                        Arc::new(move |e: OcrError| {
                            if let Ok(mut g) = last_error_for_cb.lock() {
                                *g = Some(e);
                            }
                        });

                    let queue = OcrQueue::start(Arc::clone(&backend), Arc::clone(&db), root, emit_ready, on_error);
                    app.manage(OcrState {
                        queue,
                        backend_name,
                        backend_version,
                        last_error,
                    });

                    let db_for_listener = Arc::clone(&db);
                    let app_handle = app.app_handle().clone();
                    app_handle.clone().listen("capture:saved", move |event| {
                        let capture_id = event.payload().trim_matches('"').to_string();
                        if capture_id.is_empty() { return; }
                        match snk_library::captures::get(&db_for_listener, &capture_id) {
                            Ok(capture) => {
                                let image_path = std::path::PathBuf::from(&capture.file_path);
                                if let Some(ocr) = app_handle.try_state::<OcrState>() {
                                    ocr.queue.enqueue(capture_id, image_path);
                                }
                            }
                            Err(e) => tracing::warn!(capture_id, error = %e, "could not look up capture for ocr"),
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = ?e, "ocr backend unavailable; OCR disabled this session");
                    app.manage(OcrState {
                        queue: OcrQueue::disabled(),
                        backend_name: "none",
                        backend_version: "unavailable".into(),
                        last_error: std::sync::Arc::new(std::sync::Mutex::new(Some(e))),
                    });
                }
            }

            Ok(())
        })
        .build()
}
