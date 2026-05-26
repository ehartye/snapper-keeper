use std::sync::Arc;

use serde_json::json;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};

use snk_library::LibraryState;

use crate::queue::OcrQueue;

pub struct OcrState {
    pub queue: OcrQueue,
}

/// Default OCR language code. Future enhancement: per-capture or
/// per-app language detection.
const DEFAULT_LANGUAGE: &str = "eng";

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
                Err(e) => {
                    tracing::debug!(error = %e, "no resource dir; falling back to system tesseract")
                }
            }

            let queue = OcrQueue::start(Arc::clone(&db), root.clone());
            app.manage(OcrState { queue });

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
                        if let Some(ocr) = app_handle.try_state::<OcrState>() {
                            let dropped = ocr.queue.enqueue(
                                capture_id,
                                image_path,
                                DEFAULT_LANGUAGE.to_string(),
                            );
                            if let Some(dropped_id) = dropped {
                                emit_dropped(&app_handle, &dropped_id);
                            }
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
                    DEFAULT_LANGUAGE.to_string(),
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
