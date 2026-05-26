use std::sync::Arc;

use serde_json::json;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};

use snk_library::ocr::OcrWord;
use snk_library::LibraryState;

use crate::backend::OcrBackend;
use crate::queue::OcrQueue;
use crate::OcrError;

pub struct OcrState {
    pub queue: OcrQueue,
    pub backend_name: &'static str,
    pub backend_version: String,
    pub last_error: std::sync::Arc<std::sync::Mutex<Option<OcrError>>>,
}

#[tauri::command]
pub fn ocr_status<R: Runtime>(app: tauri::AppHandle<R>) -> Result<serde_json::Value, String> {
    let state = app.try_state::<OcrState>().ok_or("ocr state missing")?;
    let last_err = state.last_error.lock().map_err(|e| e.to_string())?;
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
) -> Result<Vec<OcrWord>, String> {
    let lib = app.state::<LibraryState>();
    let row = snk_library::ocr::get(&lib.db, &capture_id).map_err(|e| e.to_string())?;
    Ok(row.and_then(|r| r.words).unwrap_or_default())
}

fn build_backend() -> Result<Arc<dyn OcrBackend>, OcrError> {
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

                    // Startup sweep (per #40): re-enqueue captures whose OCR never
                    // ran (e.g. app quit mid-queue, queue overflow, plugin start
                    // failure on previous launch). Deferred to a background task so
                    // the synchronous DB query doesn't block Tauri's main thread.
                    let app_handle_for_sweep = app.app_handle().clone();
                    let db_for_sweep = Arc::clone(&db);
                    tauri::async_runtime::spawn(async move {
                        tokio::task::spawn_blocking(move || {
                            startup_sweep(app_handle_for_sweep, db_for_sweep);
                        })
                        .await
                        .ok();
                    });

                    let db_for_listener = Arc::clone(&db);
                    let app_handle = app.app_handle().clone();
                    app_handle.clone().listen("capture:saved", move |event| {
                        let capture_id = event.payload().trim_matches('"').to_string();
                        if capture_id.is_empty() {
                            return;
                        }
                        match snk_library::captures::get(&db_for_listener, &capture_id) {
                            Ok(capture) => {
                                let image_path = std::path::PathBuf::from(&capture.file_path);
                                if let Some(ocr) = app_handle.try_state::<OcrState>() {
                                    let dropped = ocr.queue.enqueue(capture_id, image_path);
                                    if let Some(dropped_id) = dropped {
                                        emit_dropped(&app_handle, &dropped_id);
                                    }
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

/// Re-enqueue any non-deleted captures that lack an OCR row. Runs once
/// at plugin setup in a background thread. Bounded by the queue capacity
/// — if the sweep finds more than 100 missing captures, the first 100 are
/// enqueued and the rest are left for the next startup sweep (they will
/// still be missing OCR text and will be picked up then).
fn startup_sweep<R: Runtime>(app: tauri::AppHandle<R>, db: Arc<snk_library::Db>) {
    match snk_library::ocr::captures_missing_text(&db) {
        Ok(missing) => {
            if missing.is_empty() {
                tracing::info!("ocr startup sweep: no missing captures");
                return;
            }
            tracing::info!(
                count = missing.len(),
                "ocr startup sweep: enqueueing captures missing OCR"
            );
            let Some(ocr) = app.try_state::<OcrState>() else {
                tracing::warn!("ocr startup sweep: OcrState not yet managed; skipping");
                return;
            };
            for (capture_id, file_path) in missing {
                if ocr.queue.is_full() {
                    tracing::info!(
                        "ocr startup sweep: queue full, remaining captures deferred to next sweep"
                    );
                    break;
                }
                let dropped = ocr.queue.enqueue(
                    capture_id,
                    std::path::PathBuf::from(file_path),
                );
                if let Some(dropped_id) = dropped {
                    emit_dropped(&app, &dropped_id);
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "ocr startup sweep query failed; some captures may never OCR until next sweep");
        }
    }
}

fn emit_dropped<R: Runtime>(app: &tauri::AppHandle<R>, capture_id: &str) {
    if let Err(e) = app.emit("ocr:dropped", json!({ "capture_id": capture_id })) {
        tracing::warn!(error = %e, "failed to emit ocr:dropped event");
    }
}
