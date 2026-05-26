use std::sync::Arc;

use snk_library::Db;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::backend::{OcrBackend, OcrResult};
use crate::OcrError;

pub struct OcrQueue {
    tx: mpsc::UnboundedSender<OcrJob>,
}

struct OcrJob {
    capture_id: String,
    image_path: std::path::PathBuf,
}

impl OcrQueue {
    pub fn start(
        backend: Arc<dyn OcrBackend>,
        db: Arc<Db>,
        library_root: std::path::PathBuf,
        emit_ready: Arc<dyn Fn(&str) + Send + Sync>,
        on_error: Arc<dyn Fn(OcrError) + Send + Sync>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(worker(rx, backend, db, library_root, emit_ready, on_error));
        Self { tx }
    }

    // Used in the T9→T10 transition (plugin can't yet construct a backend) and as a
    // runtime fallback in T10 when backend construction fails. enqueue() becomes a
    // no-op because the receiver is dropped — tx.send returns Err and is logged.
    pub fn disabled() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self { tx }
    }

    pub fn enqueue(&self, capture_id: String, image_path: std::path::PathBuf) {
        if self
            .tx
            .send(OcrJob {
                capture_id,
                image_path,
            })
            .is_err()
        {
            error!("ocr queue closed");
        }
    }
}

async fn worker(
    mut rx: mpsc::UnboundedReceiver<OcrJob>,
    backend: Arc<dyn OcrBackend>,
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    emit_ready: Arc<dyn Fn(&str) + Send + Sync>,
    on_error: Arc<dyn Fn(OcrError) + Send + Sync>,
) {
    info!(backend = backend.name(), "ocr worker started");
    while let Some(job) = rx.recv().await {
        let full_path = library_root.join(&job.image_path);
        let backend_clone = Arc::clone(&backend);
        let db_clone = Arc::clone(&db);
        let cap_id = job.capture_id.clone();
        let emit = Arc::clone(&emit_ready);

        // FFI calls are kept off the tokio runtime — Vision is fast but synchronous;
        // WinOcr bridges an async UWP API but still benefits from isolation.
        let result = tokio::task::spawn_blocking(move || backend_clone.recognize(&full_path)).await;

        match result {
            Ok(Ok(out)) => {
                if let Err(e) = persist_and_index(
                    &db_clone,
                    &cap_id,
                    &out,
                    backend.name(),
                    &backend.engine_version(),
                ) {
                    on_error(OcrError::Recognize {
                        detail: format!("persist: {e}"),
                    });
                    error!(capture_id = %cap_id, error = %e, "persist ocr failed");
                    continue;
                }
                emit(&cap_id);
                info!(capture_id = %cap_id, chars = out.text.len(), words = out.words.len(), "ocr indexed");
            }
            Ok(Err(e)) => {
                on_error(e.clone());
                error!(capture_id = %cap_id, error = ?e, "backend recognize failed");
            }
            Err(e) => {
                on_error(OcrError::Recognize {
                    detail: format!("task panicked: {e}"),
                });
                error!(capture_id = %cap_id, error = %e, "ocr task panicked");
            }
        }
    }
    info!("ocr worker stopped");
}

fn persist_and_index(
    db: &Db,
    capture_id: &str,
    out: &OcrResult,
    backend_name: &str,
    engine_version: &str,
) -> Result<(), String> {
    // Use the qualified engine string the caller passed in; backend_name is for logging only.
    let _ = backend_name;
    snk_library::ocr::upsert_full(
        db,
        capture_id,
        &out.text,
        &out.language,
        out.confidence,
        &out.words,
        engine_version,
    )
    .map_err(|e| e.to_string())?;
    let cap = snk_library::captures::get(db, capture_id).map_err(|e| e.to_string())?;
    snk_library::search::index_capture(
        db,
        capture_id,
        cap.source_app.as_deref(),
        cap.source_window_title.as_deref(),
        Some(&out.text),
        None,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
