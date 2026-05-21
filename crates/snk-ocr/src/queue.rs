use std::sync::Arc;

use snk_library::Db;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::sidecar;

pub struct OcrQueue {
    tx: mpsc::UnboundedSender<OcrJob>,
}

struct OcrJob {
    capture_id: String,
    image_path: std::path::PathBuf,
    language: String,
}

impl OcrQueue {
    pub fn start(db: Arc<Db>, library_root: std::path::PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(worker(rx, db, library_root));
        Self { tx }
    }

    pub fn enqueue(&self, capture_id: String, image_path: std::path::PathBuf, language: String) {
        if self
            .tx
            .send(OcrJob {
                capture_id,
                image_path,
                language,
            })
            .is_err()
        {
            error!("ocr queue closed");
        }
    }
}

async fn worker(
    mut rx: mpsc::UnboundedReceiver<OcrJob>,
    db: Arc<Db>,
    library_root: std::path::PathBuf,
) {
    info!("ocr worker started");
    while let Some(job) = rx.recv().await {
        let OcrJob {
            capture_id,
            image_path,
            language,
        } = job;
        let full_path = library_root.join(&image_path);
        let db_clone = db.clone();
        let lang_for_blocking = language.clone();

        // Tesseract is a blocking child-process call — keep it off the async runtime.
        let result = tokio::task::spawn_blocking(move || {
            sidecar::run_tesseract(&full_path, &lang_for_blocking)
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.text.is_empty() {
                    info!(capture_id = %capture_id, "ocr produced no text");
                    continue;
                }
                if let Err(e) = snk_library::ocr::upsert(
                    &db_clone,
                    &capture_id,
                    &output.text,
                    &language,
                    output.confidence,
                ) {
                    error!(capture_id = %capture_id, error = %e, "failed to store ocr text");
                    continue;
                }
                // Re-index capture with OCR text so FTS matches the new content.
                match snk_library::captures::get(&db_clone, &capture_id) {
                    Ok(cap) => {
                        if let Err(e) = snk_library::search::index_capture(
                            &db_clone,
                            &capture_id,
                            cap.source_app.as_deref(),
                            cap.source_window_title.as_deref(),
                            Some(&output.text),
                            None,
                        ) {
                            error!(capture_id = %capture_id, error = %e, "failed to re-index capture for fts");
                            continue;
                        }
                    }
                    Err(e) => {
                        error!(capture_id = %capture_id, error = %e, "capture missing during ocr re-index");
                        continue;
                    }
                }
                info!(capture_id = %capture_id, chars = output.text.len(), "ocr indexed");
            }
            Ok(Err(e)) => {
                error!(capture_id = %capture_id, error = %e, "ocr sidecar failed");
            }
            Err(e) => {
                error!(capture_id = %capture_id, error = %e, "ocr task panicked");
            }
        }
    }
    info!("ocr worker stopped");
}
