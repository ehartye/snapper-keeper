use crate::{
    health::HealthSink,
    watcher::{worker_step, ClipboardEvent, WatcherState},
};
use snk_library::Db;
use std::sync::Arc;

#[cfg(not(target_os = "macos"))]
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

#[cfg(target_os = "macos")]
pub(crate) fn start_polling(
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    interval: std::time::Duration,
    sink: Arc<dyn HealthSink>,
) {
    use crate::observation::{poll_once, GenerationGate};
    std::thread::spawn(move || {
        // Health currently describes opening the clipboard, not individual payload reads.
        let mut clip =
            objc2::rc::autoreleasepool(|_| crate::health::open_clipboard_with_backoff(&*sink));
        let mut state = WatcherState::new();
        let mut gate = GenerationGate::default();
        let clock = std::time::Instant::now();
        loop {
            // Each Cocoa observation owns a pool; no autoreleased payload survives sleep.
            objc2::rc::autoreleasepool(|_| {
                let pasteboard = objc2_app_kit::NSPasteboard::generalPasteboard();
                let mut source = MacSource {
                    clipboard: &mut clip,
                    pasteboard: &pasteboard,
                };
                poll_once(
                    &mut source,
                    &mut gate,
                    &mut state,
                    clock.elapsed(),
                    |event, state, probe, app| {
                        worker_step(event, state, &db, &library_root, probe, app)
                    },
                );
            });
            std::thread::sleep(interval);
        }
    });
}

#[cfg(target_os = "macos")]
struct MacSource<'a, R> {
    clipboard: &'a mut R,
    pasteboard: &'a objc2_app_kit::NSPasteboard,
}

#[cfg(target_os = "macos")]
impl<R: crate::observation::PayloadReader> crate::observation::ObservationSource
    for MacSource<'_, R>
{
    fn generation(&mut self) -> isize {
        self.pasteboard.changeCount()
    }
    fn sensitive(&mut self) -> bool {
        crate::platform::is_sensitive_pasteboard(self.pasteboard)
    }
    fn event(&mut self) -> Option<ClipboardEvent> {
        crate::observation::read_payload(self.clipboard)
    }
    // Frontmost application is a heuristic, not pasteboard writer attribution.
    fn source_app(&mut self) -> Option<crate::source_app::SourceApp> {
        crate::source_app::current()
    }
}

#[cfg(target_os = "macos")]
impl crate::observation::PayloadReader for arboard::Clipboard {
    fn text(&mut self) -> Option<String> {
        self.get_text().ok()
    }
    fn image(&mut self) -> Option<ClipboardEvent> {
        let image = self.get_image().ok()?;
        Some(ClipboardEvent::Image {
            bytes: image.bytes.into_owned(),
            width: image.width,
            height: image.height,
        })
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "polling_macos_tests.rs"]
mod macos_tests;
