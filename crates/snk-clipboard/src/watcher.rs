use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use arboard::Clipboard;
use tracing::{debug, error};

use snk_library::clipboard::{ClipboardItemKind, NewClipboardItem};
use snk_library::{files, Db};

use crate::blocklist;
use crate::sensitivity::SensitivityProbe;
use crate::source_app::SourceApp;

/// A single observed clipboard change that the watcher must decide
/// what to do with.
pub(crate) enum ClipboardEvent {
    /// Text content was on the clipboard at the time of observation.
    Text(String),
    /// Image bytes were on the clipboard (already PNG-encoded by arboard).
    Image(Vec<u8>),
}

/// Why the watcher did not record a particular event.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkipReason {
    SensitiveFlag,
    AppBlocked(String), // identifier
    DuplicateHash,
    EmptyContent,
    /// Insert or disk-write failed; treated as a skip so the watcher loop
    /// keeps draining events instead of crashing on a transient error.
    PersistFailed,
}

/// Outcome of a single decision cycle.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StepResult {
    Saved { item_id: String },
    DedupedTo { existing_id: String },
    Skipped(SkipReason),
}

/// Shared per-thread state the watcher carries across cycles.
pub(crate) struct WatcherState {
    pub last_hash: Option<String>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self { last_hash: None }
    }
}

const MAX_UNPINNED: u32 = 200;

pub(crate) static SKIP_NEXT: AtomicBool = AtomicBool::new(false);

pub fn mark_skip_next() {
    SKIP_NEXT.store(true, Ordering::SeqCst);
}

pub fn start_watcher(db: Arc<Db>, library_root: std::path::PathBuf) {
    #[cfg(target_os = "windows")]
    {
        crate::platform_watcher::windows::start(db, library_root);
        return;
    }

    #[cfg(not(target_os = "windows"))]
    {
        start_polling(db, library_root, std::time::Duration::from_millis(100));
    }
}

pub(crate) fn start_polling(
    db: Arc<Db>,
    library_root: std::path::PathBuf,
    interval: std::time::Duration,
) {
    use crate::sensitivity::OsProbe;
    use crate::source_app;

    std::thread::spawn(move || {
        let mut clip = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "failed to open clipboard for watching");
                return;
            }
        };
        let mut state = WatcherState::new();
        let probe = OsProbe;

        loop {
            std::thread::sleep(interval);
            if SKIP_NEXT
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                debug!("skipping own clipboard write");
                continue;
            }

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
                ClipboardEvent::Image(img.bytes.into_owned())
            } else {
                continue;
            };

            let source = source_app::current();
            let _ = worker_step(event, &mut state, &db, &library_root, &probe, source);
        }
    });
}

/// Pure decision cycle. The probe + source-app lookup are injected so
/// unit tests can run this without touching the real OS clipboard.
pub(crate) fn worker_step(
    event: ClipboardEvent,
    state: &mut WatcherState,
    db: &Db,
    library_root: &Path,
    probe: &dyn SensitivityProbe,
    source: Option<SourceApp>,
) -> StepResult {
    if probe.is_sensitive() {
        // Record the hash so a follow-up identical observation doesn't
        // re-run the whole pipeline. We compute it cheaply from the event.
        state.last_hash = Some(hash_of_event(&event));
        return StepResult::Skipped(SkipReason::SensitiveFlag);
    }

    if let Some(ref src) = source {
        if blocklist::matches(db, src) {
            state.last_hash = Some(hash_of_event(&event));
            return StepResult::Skipped(SkipReason::AppBlocked(src.identifier.clone()));
        }
    }

    match event {
        ClipboardEvent::Text(text) => {
            if text.is_empty() {
                return StepResult::Skipped(SkipReason::EmptyContent);
            }
            let hash = crate::hasher::hash_text(&text);
            if state.last_hash.as_deref() == Some(&hash) {
                return StepResult::Skipped(SkipReason::DuplicateHash);
            }
            state.last_hash = Some(hash.clone());

            match snk_library::clipboard::find_by_hash(db, &hash) {
                Ok(Some(existing)) => {
                    let _ = snk_library::clipboard::bump_timestamp(db, &existing.id);
                    StepResult::DedupedTo { existing_id: existing.id }
                }
                _ => {
                    let new_item = NewClipboardItem {
                        kind: ClipboardItemKind::Text,
                        text_content: Some(text),
                        file_path: None,
                        content_hash: hash,
                        source_app: source.as_ref().map(|s| s.identifier.clone()),
                        source_window_title: None,
                    };
                    match snk_library::clipboard::insert(db, new_item) {
                        Ok(item) => {
                            let _ = snk_library::clipboard::evict_unpinned(db, MAX_UNPINNED);
                            StepResult::Saved { item_id: item.id }
                        }
                        Err(_) => StepResult::Skipped(SkipReason::PersistFailed),
                    }
                }
            }
        }
        ClipboardEvent::Image(bytes) => {
            if bytes.is_empty() {
                return StepResult::Skipped(SkipReason::EmptyContent);
            }
            let hash = crate::hasher::hash_image_bytes(&bytes);
            if state.last_hash.as_deref() == Some(&hash) {
                return StepResult::Skipped(SkipReason::DuplicateHash);
            }
            state.last_hash = Some(hash.clone());

            match snk_library::clipboard::find_by_hash(db, &hash) {
                Ok(Some(existing)) => {
                    let _ = snk_library::clipboard::bump_timestamp(db, &existing.id);
                    StepResult::DedupedTo { existing_id: existing.id }
                }
                _ => {
                    let id = uuid::Uuid::now_v7();
                    let relative = files::clipboard_image_relative_path(&id);
                    if files::write_atomic(library_root, &relative, &bytes).is_err() {
                        return StepResult::Skipped(SkipReason::PersistFailed);
                    }
                    let new_item = NewClipboardItem {
                        kind: ClipboardItemKind::Image,
                        text_content: None,
                        file_path: Some(relative),
                        content_hash: hash,
                        source_app: source.as_ref().map(|s| s.identifier.clone()),
                        source_window_title: None,
                    };
                    match snk_library::clipboard::insert(db, new_item) {
                        Ok(item) => {
                            let _ = snk_library::clipboard::evict_unpinned(db, MAX_UNPINNED);
                            StepResult::Saved { item_id: item.id }
                        }
                        Err(_) => StepResult::Skipped(SkipReason::PersistFailed),
                    }
                }
            }
        }
    }
}

fn hash_of_event(event: &ClipboardEvent) -> String {
    match event {
        ClipboardEvent::Text(t) => crate::hasher::hash_text(t),
        ClipboardEvent::Image(b) => crate::hasher::hash_image_bytes(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitivity::FakeProbe;
    use crate::source_app::{SourceApp, SourceAppKind};
    use serde_json::json;
    use snk_library::settings;

    fn fresh_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        let db = Db::open(&path).unwrap();
        (dir, db)
    }

    #[test]
    fn sensitive_flag_skips_without_persisting() {
        let (tmp, db) = fresh_db();
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text("secret".into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: true },
            None,
        );
        assert_eq!(result, StepResult::Skipped(SkipReason::SensitiveFlag));
        assert!(state.last_hash.is_some(), "last_hash should be set on skip");

        let items =
            snk_library::clipboard::list(&db, snk_library::ListClipboardQuery::default()).unwrap();
        assert_eq!(items.len(), 0, "no row should be inserted on sensitive skip");
    }

    #[test]
    fn blocked_app_skips_without_persisting() {
        let (tmp, db) = fresh_db();
        settings::set(
            &db,
            "clipboard.app_blocklist",
            &json!([{
                "identifier": "1password.exe",
                "display_name": "1Password",
                "kind": "windows_exe"
            }]),
        )
        .unwrap();
        let src = SourceApp {
            identifier: "1password.exe".into(),
            display_name: "1Password".into(),
            kind: SourceAppKind::WindowsExe,
        };
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text("password123".into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            Some(src.clone()),
        );
        assert_eq!(result, StepResult::Skipped(SkipReason::AppBlocked(src.identifier)));
        let items =
            snk_library::clipboard::list(&db, snk_library::ListClipboardQuery::default()).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn allowed_text_event_is_saved_with_source_app() {
        let (tmp, db) = fresh_db();
        let src = SourceApp {
            identifier: "code.exe".into(),
            display_name: "Visual Studio Code".into(),
            kind: SourceAppKind::WindowsExe,
        };
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text("hello".into()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            Some(src.clone()),
        );
        match result {
            StepResult::Saved { item_id } => {
                let stored = snk_library::clipboard::get(&db, &item_id).unwrap();
                assert_eq!(stored.source_app, Some(src.identifier));
            }
            other => panic!("expected Saved, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_hash_skips_without_re_inserting() {
        let (tmp, db) = fresh_db();
        let mut state = WatcherState::new();
        let probe = FakeProbe { answer: false };

        let first = worker_step(
            ClipboardEvent::Text("dup".into()),
            &mut state,
            &db,
            tmp.path(),
            &probe,
            None,
        );
        assert!(matches!(first, StepResult::Saved { .. }));

        let second = worker_step(
            ClipboardEvent::Text("dup".into()),
            &mut state,
            &db,
            tmp.path(),
            &probe,
            None,
        );
        assert_eq!(second, StepResult::Skipped(SkipReason::DuplicateHash));
    }

    #[test]
    fn empty_text_is_skipped() {
        let (tmp, db) = fresh_db();
        let mut state = WatcherState::new();
        let result = worker_step(
            ClipboardEvent::Text(String::new()),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            None,
        );
        assert_eq!(result, StepResult::Skipped(SkipReason::EmptyContent));
    }
}
