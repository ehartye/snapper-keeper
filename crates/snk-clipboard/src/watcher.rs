use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arboard::Clipboard;
use tracing::{debug, error, warn};

use snk_library::clipboard::{self, NewClipboardItem};
use snk_library::{files, Db};

use crate::hasher;

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
            std::thread::sleep(Duration::from_millis(500));

            if SKIP_NEXT
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                debug!("skipping own clipboard write");
                continue;
            }

            if let Ok(text) = clip.get_text() {
                if !text.is_empty() {
                    let hash = hasher::hash_text(&text);
                    if last_hash.as_deref() == Some(&hash) {
                        continue;
                    }
                    last_hash = Some(hash.clone());

                    match clipboard::find_by_hash(&db, &hash) {
                        Ok(Some(existing)) => {
                            let _ = clipboard::bump_timestamp(&db, &existing.id);
                        }
                        Ok(None) => {
                            let new_item = NewClipboardItem {
                                kind: "text".into(),
                                text_content: Some(text),
                                file_path: None,
                                content_hash: hash,
                                source_app: None,
                                source_window_title: None,
                            };
                            match clipboard::insert(&db, new_item) {
                                Ok(_) => {
                                    let _ = clipboard::evict_unpinned(&db, 200);
                                }
                                Err(e) => warn!(error = ?e, "clipboard insert failed"),
                            }
                        }
                        Err(e) => warn!(error = ?e, "clipboard hash lookup failed"),
                    }
                    continue;
                }
            }

            if let Ok(img) = clip.get_image() {
                let bytes = img.bytes.to_vec();
                if !bytes.is_empty() {
                    let hash = hasher::hash_image_bytes(&bytes);
                    if last_hash.as_deref() == Some(&hash) {
                        continue;
                    }
                    last_hash = Some(hash.clone());

                    match clipboard::find_by_hash(&db, &hash) {
                        Ok(Some(existing)) => {
                            let _ = clipboard::bump_timestamp(&db, &existing.id);
                        }
                        Ok(None) => {
                            let id = uuid::Uuid::now_v7();
                            let relative = files::clipboard_image_relative_path(&id);
                            if files::write_atomic(&library_root, &relative, &bytes).is_ok() {
                                let new_item = NewClipboardItem {
                                    kind: "image".into(),
                                    text_content: None,
                                    file_path: Some(relative),
                                    content_hash: hash,
                                    source_app: None,
                                    source_window_title: None,
                                };
                                match clipboard::insert(&db, new_item) {
                                    Ok(_) => {
                                        let _ = clipboard::evict_unpinned(&db, 200);
                                    }
                                    Err(e) => warn!(error = ?e, "clipboard image insert failed"),
                                }
                            }
                        }
                        Err(e) => warn!(error = ?e, "clipboard image hash lookup failed"),
                    }
                }
            }
        }
    });
}
