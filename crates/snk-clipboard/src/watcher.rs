use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arboard::Clipboard;
use tracing::{debug, error, warn};

use snk_library::clipboard::{self, ClipboardItemKind, NewClipboardItem};
use snk_library::{files, Db};

use crate::hasher;

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const MAX_UNPINNED: u32 = 200;

static SKIP_NEXT: AtomicBool = AtomicBool::new(false);

pub fn mark_skip_next() {
    SKIP_NEXT.store(true, Ordering::SeqCst);
}

pub fn start_watcher(db: Arc<Db>, library_root: std::path::PathBuf) {
    std::thread::spawn(move || {
        let mut clip = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "failed to open clipboard for watching");
                return;
            }
        };
        let mut last_hash: Option<String> = None;

        loop {
            std::thread::sleep(POLL_INTERVAL);

            if SKIP_NEXT
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                debug!("skipping own clipboard write");
                continue;
            }

            if poll_text(&mut clip, &db, &mut last_hash) {
                continue;
            }
            poll_image(&mut clip, &db, &library_root, &mut last_hash);
        }
    });
}

/// Returns true if text was processed (so the caller should skip the image
/// branch). Text content always wins over an image on the clipboard.
fn poll_text(clip: &mut Clipboard, db: &Db, last_hash: &mut Option<String>) -> bool {
    let Ok(text) = clip.get_text() else {
        return false;
    };
    if text.is_empty() {
        return false;
    }
    let hash = hasher::hash_text(&text);
    if last_hash.as_deref() == Some(&hash) {
        return true;
    }
    *last_hash = Some(hash.clone());

    match clipboard::find_by_hash(db, &hash) {
        Ok(Some(existing)) => {
            let _ = clipboard::bump_timestamp(db, &existing.id);
        }
        Ok(None) => {
            let new_item = NewClipboardItem {
                kind: ClipboardItemKind::Text,
                text_content: Some(text),
                file_path: None,
                content_hash: hash,
                source_app: None,
                source_window_title: None,
            };
            match clipboard::insert(db, new_item) {
                Ok(_) => {
                    let _ = clipboard::evict_unpinned(db, MAX_UNPINNED);
                }
                Err(e) => warn!(error = ?e, "clipboard text insert failed"),
            }
        }
        Err(e) => warn!(error = ?e, "clipboard text hash lookup failed"),
    }
    true
}

fn poll_image(
    clip: &mut Clipboard,
    db: &Db,
    library_root: &std::path::Path,
    last_hash: &mut Option<String>,
) {
    let Ok(img) = clip.get_image() else { return };
    if img.bytes.is_empty() {
        return;
    }
    let hash = hasher::hash_image_bytes(&img.bytes);
    if last_hash.as_deref() == Some(&hash) {
        return;
    }
    *last_hash = Some(hash.clone());

    match clipboard::find_by_hash(db, &hash) {
        Ok(Some(existing)) => {
            let _ = clipboard::bump_timestamp(db, &existing.id);
        }
        Ok(None) => {
            let id = uuid::Uuid::now_v7();
            let relative = files::clipboard_image_relative_path(&id);
            if let Err(e) = files::write_atomic(library_root, &relative, &img.bytes) {
                warn!(error = ?e, path = %relative.display(), "clipboard image write failed");
                return;
            }
            let new_item = NewClipboardItem {
                kind: ClipboardItemKind::Image,
                text_content: None,
                file_path: Some(relative),
                content_hash: hash,
                source_app: None,
                source_window_title: None,
            };
            match clipboard::insert(db, new_item) {
                Ok(_) => {
                    let _ = clipboard::evict_unpinned(db, MAX_UNPINNED);
                }
                Err(e) => warn!(error = ?e, "clipboard image insert failed"),
            }
        }
        Err(e) => warn!(error = ?e, "clipboard image hash lookup failed"),
    }
}
