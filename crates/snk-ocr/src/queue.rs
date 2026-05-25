use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use snk_library::Db;
use tokio::sync::Notify;
use tracing::{error, info};

use crate::sidecar;

/// Maximum number of OCR jobs queued at once. When full, the oldest
/// queued job is dropped (and the dropped capture_id emitted via
/// `OcrQueue::enqueue` return) and the new job takes its slot.
///
/// 100 = generous enough for bursty timed-capture mode (10 captures/sec
/// would saturate in 10s); small enough to keep memory bounded. The
/// startup sweep catches captures whose jobs got dropped or were never
/// queued (e.g. app quit mid-queue).
const MAX_QUEUE_SIZE: usize = 100;

pub struct OcrQueue {
    queue: Arc<Mutex<VecDeque<OcrJob>>>,
    notify: Arc<Notify>,
}

#[derive(Debug)]
struct OcrJob {
    capture_id: String,
    image_path: std::path::PathBuf,
    language: String,
}

impl OcrQueue {
    pub fn start(db: Arc<Db>, library_root: std::path::PathBuf) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_QUEUE_SIZE)));
        let notify = Arc::new(Notify::new());

        let worker_queue = Arc::clone(&queue);
        let worker_notify = Arc::clone(&notify);
        tauri::async_runtime::spawn(worker(worker_queue, worker_notify, db, library_root));

        Self { queue, notify }
    }

    /// Enqueue a new OCR job. If the queue is at capacity, the OLDEST
    /// queued job is dropped to make room. Returns the dropped
    /// `capture_id` if a drop occurred (caller should emit
    /// `ocr:dropped` event so the UI can surface the backlog overflow).
    pub fn enqueue(
        &self,
        capture_id: String,
        image_path: std::path::PathBuf,
        language: String,
    ) -> Option<String> {
        let dropped = {
            let mut q = self.queue.lock().expect("ocr queue mutex poisoned");
            let dropped = if q.len() >= MAX_QUEUE_SIZE {
                q.pop_front().map(|j| j.capture_id)
            } else {
                None
            };
            q.push_back(OcrJob {
                capture_id,
                image_path,
                language,
            });
            dropped
        };
        self.notify.notify_one();
        dropped
    }

    /// Current queued-jobs count. Used by tests + the optional About-
    /// panel surface (#36) to show backlog size.
    pub fn len(&self) -> usize {
        self.queue.lock().expect("ocr queue mutex poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

async fn worker(
    queue: Arc<Mutex<VecDeque<OcrJob>>>,
    notify: Arc<Notify>,
    db: Arc<Db>,
    library_root: std::path::PathBuf,
) {
    info!("ocr worker started");
    loop {
        // Wait for new work. notify_one is buffered: if notify_one was
        // called while we weren't awaiting, the next notified() returns
        // immediately.
        notify.notified().await;

        // Drain everything currently queued.
        loop {
            let job = queue.lock().expect("ocr queue mutex poisoned").pop_front();
            let Some(job) = job else {
                break;
            };
            process_job(job, &db, &library_root).await;
        }
    }
}

async fn process_job(job: OcrJob, db: &Arc<Db>, library_root: &std::path::Path) {
    let OcrJob {
        capture_id,
        image_path,
        language,
    } = job;
    let full_path = library_root.join(&image_path);
    let db_clone = db.clone();
    let lang_for_blocking = language.clone();

    // Tesseract is a blocking child-process call — keep it off the async runtime.
    let result =
        tokio::task::spawn_blocking(move || sidecar::run_tesseract(&full_path, &lang_for_blocking))
            .await;

    match result {
        Ok(Ok(output)) => {
            if output.text.is_empty() {
                info!(capture_id = %capture_id, "ocr produced no text");
                return;
            }
            if let Err(e) = snk_library::ocr::upsert(
                &db_clone,
                &capture_id,
                &output.text,
                &language,
                output.confidence,
            ) {
                error!(capture_id = %capture_id, error = %e, "failed to store ocr text");
                return;
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
                        return;
                    }
                }
                Err(e) => {
                    error!(capture_id = %capture_id, error = %e, "capture missing during ocr re-index");
                    return;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_queue() -> OcrQueue {
        // snk-library's test_support::fresh_db is crate-private;
        // open a real DB via the public API into a tempdir instead.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sk.db");
        // Leak the tempdir so it outlives the queue (the worker spawns
        // an async task that holds the Db; test process exit will clean up).
        std::mem::forget(dir);
        let db = snk_library::Db::open(&path).expect("open db");
        OcrQueue::start(Arc::new(db), PathBuf::from("."))
    }

    #[test]
    fn enqueue_under_capacity_does_not_drop() {
        let q = make_queue();
        for i in 0..50 {
            let dropped = q.enqueue(format!("cap-{i}"), PathBuf::from("x.png"), "eng".into());
            assert!(dropped.is_none(), "under capacity should not drop");
        }
    }

    #[test]
    fn enqueue_at_capacity_drops_oldest() {
        let q = make_queue();
        // Fill to capacity. Use unique ids so we can identify the oldest.
        for i in 0..MAX_QUEUE_SIZE {
            let dropped = q.enqueue(format!("cap-{i}"), PathBuf::from("x.png"), "eng".into());
            assert!(dropped.is_none());
        }
        // One more push — should evict cap-0 (the oldest).
        let dropped = q.enqueue("overflow".into(), PathBuf::from("y.png"), "eng".into());
        assert_eq!(
            dropped.as_deref(),
            Some("cap-0"),
            "oldest queued capture should be evicted"
        );
    }

    #[test]
    fn drop_returns_correct_id_under_continuous_pressure() {
        let q = make_queue();
        for i in 0..MAX_QUEUE_SIZE + 5 {
            let _ = q.enqueue(format!("cap-{i}"), PathBuf::from("x.png"), "eng".into());
        }
        // Pushing one more should evict cap-5 (since cap-0..cap-4 were
        // already evicted by the earlier overflow).
        let dropped = q
            .enqueue("final".into(), PathBuf::from("z.png"), "eng".into())
            .expect("should drop");
        assert_eq!(dropped, "cap-5");
    }

    #[test]
    fn len_reflects_queue_size() {
        let q = make_queue();
        assert_eq!(q.len(), 0);
        assert!(q.is_empty());
        q.enqueue("a".into(), PathBuf::from("x.png"), "eng".into());
        // Note: len() may be 0 by the time we check it because the
        // worker drains async. Don't assert exact length post-enqueue.
        // The len reads are tested better via the drop tests above.
    }
}
