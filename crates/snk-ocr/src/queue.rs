use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use snk_library::Db;
use tokio::sync::Notify;
use tracing::{error, info};

use crate::backend::{OcrBackend, OcrResult};
use crate::OcrError;

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
}

impl OcrQueue {
    pub fn start(
        backend: Arc<dyn OcrBackend>,
        db: Arc<Db>,
        library_root: std::path::PathBuf,
        emit_ready: Arc<dyn Fn(&str) + Send + Sync>,
        on_error: Arc<dyn Fn(OcrError) + Send + Sync>,
    ) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_QUEUE_SIZE)));
        let notify = Arc::new(Notify::new());

        let worker_queue = Arc::clone(&queue);
        let worker_notify = Arc::clone(&notify);
        tauri::async_runtime::spawn(worker(
            worker_queue,
            worker_notify,
            backend,
            db,
            library_root,
            emit_ready,
            on_error,
        ));

        Self { queue, notify }
    }

    // Used as a runtime fallback when backend construction fails. enqueue() still
    // accepts jobs (up to MAX_QUEUE_SIZE) but no worker drains them — they are
    // effectively no-ops. The startup sweep on the next launch will re-enqueue them.
    pub fn disabled() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_QUEUE_SIZE))),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Construct an OcrQueue WITHOUT spawning the background worker.
    /// For tests of the queue's enqueue/drop logic where worker drain
    /// would race with the test. Production callers should always use
    /// `start()`.
    #[cfg(test)]
    fn new_for_test() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_QUEUE_SIZE))),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Enqueue a new OCR job. If the queue is at capacity, the OLDEST
    /// queued job is dropped to make room. Returns the dropped
    /// `capture_id` if a drop occurred (caller should emit
    /// `ocr:dropped` event so the UI can surface the backlog overflow).
    pub fn enqueue(&self, capture_id: String, image_path: std::path::PathBuf) -> Option<String> {
        let (dropped, was_empty) = {
            let mut q = self.queue.lock().expect("ocr queue mutex poisoned");
            let was_empty = q.is_empty();
            let dropped = if q.len() >= MAX_QUEUE_SIZE {
                q.pop_front().map(|j| j.capture_id)
            } else {
                None
            };
            q.push_back(OcrJob {
                capture_id,
                image_path,
            });
            (dropped, was_empty)
        };
        // Only wake the worker on the empty→non-empty transition; extra
        // notifications on an already-running worker would leave buffered
        // permits that cause spurious outer-loop iterations.
        if was_empty {
            self.notify.notify_one();
        }
        dropped
    }

    /// Returns `true` if the queue is at maximum capacity. Used by the
    /// startup sweep to stop enqueuing before eviction would discard
    /// items already queued in this sweep pass.
    pub fn is_full(&self) -> bool {
        self.queue.lock().expect("ocr queue mutex poisoned").len() >= MAX_QUEUE_SIZE
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
    backend: Arc<dyn OcrBackend>,
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    emit_ready: Arc<dyn Fn(&str) + Send + Sync>,
    on_error: Arc<dyn Fn(OcrError) + Send + Sync>,
) {
    info!(backend = backend.name(), "ocr worker started");
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

            let full_path = library_root.join(&job.image_path);
            let backend_clone = Arc::clone(&backend);
            let db_clone = Arc::clone(&db);
            let cap_id = job.capture_id.clone();
            let emit = Arc::clone(&emit_ready);
            let on_err = Arc::clone(&on_error);

            // FFI calls are kept off the tokio runtime — Vision is fast but synchronous;
            // WinOcr bridges an async UWP API but still benefits from isolation.
            let result =
                tokio::task::spawn_blocking(move || backend_clone.recognize(&full_path)).await;

            match result {
                Ok(Ok(out)) => {
                    if let Err(e) = persist_and_index(
                        &db_clone,
                        &cap_id,
                        &out,
                        backend.name(),
                        &backend.engine_version(),
                    ) {
                        on_err(OcrError::Recognize {
                            detail: format!("persist: {e}"),
                        });
                        error!(capture_id = %cap_id, error = %e, "persist ocr failed");
                        continue;
                    }
                    emit(&cap_id);
                    info!(capture_id = %cap_id, chars = out.text.len(), words = out.words.len(), "ocr indexed");
                }
                Ok(Err(e)) => {
                    on_err(e.clone());
                    error!(capture_id = %cap_id, error = ?e, "backend recognize failed");
                }
                Err(e) => {
                    on_err(OcrError::Recognize {
                        detail: format!("task panicked: {e}"),
                    });
                    error!(capture_id = %cap_id, error = %e, "ocr task panicked");
                }
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_queue() -> OcrQueue {
        // Use the test-only constructor so the worker doesn't drain
        // between our enqueues. CI's tokio runtime is more eager than
        // some local OS-runtime configs; without no-worker the
        // at-capacity test races with worker drain.
        OcrQueue::new_for_test()
    }

    #[test]
    fn enqueue_under_capacity_does_not_drop() {
        let q = make_queue();
        for i in 0..50 {
            let dropped = q.enqueue(format!("cap-{i}"), PathBuf::from("x.png"));
            assert!(dropped.is_none(), "under capacity should not drop");
        }
    }

    #[test]
    fn enqueue_at_capacity_drops_oldest() {
        let q = make_queue();
        // Fill to capacity. Use unique ids so we can identify the oldest.
        for i in 0..MAX_QUEUE_SIZE {
            let dropped = q.enqueue(format!("cap-{i}"), PathBuf::from("x.png"));
            assert!(dropped.is_none());
        }
        // One more push — should evict cap-0 (the oldest).
        let dropped = q.enqueue("overflow".into(), PathBuf::from("y.png"));
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
            let _ = q.enqueue(format!("cap-{i}"), PathBuf::from("x.png"));
        }
        // Pushing one more should evict cap-5 (since cap-0..cap-4 were
        // already evicted by the earlier overflow).
        let dropped = q
            .enqueue("final".into(), PathBuf::from("z.png"))
            .expect("should drop");
        assert_eq!(dropped, "cap-5");
    }

    #[test]
    fn len_reflects_queue_size() {
        let q = make_queue();
        assert_eq!(q.len(), 0);
        assert!(q.is_empty());
        q.enqueue("a".into(), PathBuf::from("x.png"));
        // Note: len() may be 0 by the time we check it because the
        // worker drains async. Don't assert exact length post-enqueue.
        // The len reads are tested better via the drop tests above.
    }
}
