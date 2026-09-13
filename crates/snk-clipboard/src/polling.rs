use crate::{
    health::HealthSink,
    watcher::{worker_step, ClipboardEvent, WatcherState},
};
use snk_library::Db;
use std::sync::Arc;

pub(crate) fn start_polling(
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    interval: std::time::Duration,
    sink: Arc<dyn HealthSink>,
) {
    use crate::sensitivity::OsProbe;
    use crate::source_app;

    std::thread::spawn(move || {
        // Retry-with-backoff instead of dying silently on the first failure;
        // reports availability through the sink so the UI can show a banner.
        let mut clip = crate::health::open_clipboard_with_backoff(&*sink);
        let mut state = WatcherState::new();
        let probe = OsProbe;

        loop {
            std::thread::sleep(interval);
            // Skip-set check moved into worker_step (content-hash based,
            // see SkipReason::OwnWrite). The polling loop no longer
            // needs its own skip gate.

            // Try text first; image only if text is absent.
            let event = if let Ok(t) = clip.get_text() {
                if t.is_empty() {
                    continue;
                }
                ClipboardEvent::Text(t)
            } else if let Ok(img) = clip.get_image() {
                if img.bytes.is_empty() {
                    continue;
                }
                let width = img.width;
                let height = img.height;
                ClipboardEvent::Image {
                    bytes: img.bytes.into_owned(),
                    width,
                    height,
                }
            } else {
                continue;
            };

            let source = source_app::current();
            let _ = worker_step(event, &mut state, &db, &library_root, &probe, source);
        }
    });
}
