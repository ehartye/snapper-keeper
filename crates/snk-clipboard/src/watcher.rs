use std::path::Path;
use std::sync::Arc;

use tracing::debug;

use snk_library::clipboard::{ClipboardItemKind, NewClipboardItem};
use snk_library::{files, Db};

use crate::blocklist;
use crate::health::HealthSink;
use crate::sensitivity::SensitivityProbe;
use crate::skip_set;
use crate::source_app::SourceApp;

/// A single observed clipboard change that the watcher must decide
/// what to do with.
pub(crate) enum ClipboardEvent {
    /// Text content was on the clipboard at the time of observation.
    Text(String),
    /// Raw RGBA image bytes from the clipboard, with pixel dimensions.
    /// worker_step encodes these to PNG before writing to disk.
    Image {
        bytes: Vec<u8>,
        width: usize,
        height: usize,
    },
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
    /// The event matches a content hash recently emitted by us
    /// (e.g. via `paste_item`). The watcher would otherwise re-record
    /// what we just wrote into the clipboard ourselves.
    OwnWrite,
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
    // At most one unpublished image (including its temporary write) can need cleanup.
    pending_image: Option<std::path::PathBuf>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            last_hash: None,
            pending_image: None,
        }
    }
}

// VERIFIED: privacy-md/clipboard-retention
// Clipboard history keeps at most this many unpinned items; older unpinned
// entries are evicted as new ones arrive (pinned items are exempt). Backs the
// retention claim in PRIVACY.md.
const MAX_UNPINNED: u32 = 200;

/// Shim retained for source compatibility during the SKIP_NEXT → skip_set
/// transition; callers (e.g. `paste_item`) now hash their content and
/// call `skip_set::mark_emitted` directly. Kept temporarily so any
/// downstream consumer using the public name doesn't break.
#[deprecated(note = "use skip_set::mark_emitted with a content hash instead")]
pub fn mark_skip_next() {
    // Intentionally a no-op. The new mechanism is content-aware, so a
    // blanket "skip the next event" can't be implemented without a hash.
    // Callers should compute the hash and call skip_set::mark_emitted.
}

pub fn start_watcher(db: Arc<Db>, library_root: std::path::PathBuf, sink: Arc<dyn HealthSink>) {
    #[cfg(target_os = "windows")]
    {
        crate::platform_watcher::windows::start(db, library_root, sink);
    }

    #[cfg(not(target_os = "windows"))]
    {
        start_polling(
            db,
            library_root,
            std::time::Duration::from_millis(100),
            sink,
        );
    }
}

pub(crate) use crate::polling::start_polling;

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
    worker_step_with_store(event, state, &LibraryStore(db), library_root, probe, source)
}

/// Persistence boundary allows failures to be exercised without reaching into library tables.
trait Store {
    fn blocked(&self, source: &SourceApp) -> bool;
    fn find(&self, hash: &str) -> snk_library::Result<Option<String>>;
    fn bump(&self, id: &str) -> snk_library::Result<()>;
    fn insert(&self, item: NewClipboardItem) -> snk_library::Result<String>;
    fn evict(&self);
}

struct LibraryStore<'a>(&'a Db);
impl Store for LibraryStore<'_> {
    fn blocked(&self, source: &SourceApp) -> bool {
        blocklist::matches(self.0, source)
    }
    fn find(&self, hash: &str) -> snk_library::Result<Option<String>> {
        snk_library::clipboard::find_by_hash(self.0, hash).map(|item| item.map(|item| item.id))
    }
    fn bump(&self, id: &str) -> snk_library::Result<()> {
        snk_library::clipboard::bump_timestamp(self.0, id)
    }
    fn insert(&self, item: NewClipboardItem) -> snk_library::Result<String> {
        snk_library::clipboard::insert(self.0, item).map(|item| item.id)
    }
    fn evict(&self) {
        let _ = snk_library::clipboard::evict_unpinned(self.0, MAX_UNPINNED);
    }
}

fn worker_step_with_store(
    event: ClipboardEvent,
    state: &mut WatcherState,
    store: &dyn Store,
    library_root: &Path,
    probe: &dyn SensitivityProbe,
    source: Option<SourceApp>,
) -> StepResult {
    let hash = hash_of_event(&event);
    let event_bytes = match &event {
        ClipboardEvent::Text(text) => text.as_bytes(),
        ClipboardEvent::Image { bytes, .. } => bytes,
    };
    if skip_set::should_skip(skip_set::hash_content(event_bytes)) {
        debug!("skipping own clipboard write (hash match)");
        state.last_hash = Some(hash);
        return StepResult::Skipped(SkipReason::OwnWrite);
    }
    if probe.is_sensitive() {
        state.last_hash = Some(hash);
        return StepResult::Skipped(SkipReason::SensitiveFlag);
    }
    if let Some(ref src) = source {
        if store.blocked(src) {
            state.last_hash = Some(hash);
            return StepResult::Skipped(SkipReason::AppBlocked(src.identifier.clone()));
        }
    }
    if event_bytes.is_empty() {
        return StepResult::Skipped(SkipReason::EmptyContent);
    }
    if state.last_hash.as_deref() == Some(&hash) {
        return StepResult::Skipped(SkipReason::DuplicateHash);
    }
    // A failure is never acknowledged: the same generation can retry unchanged bytes.
    let failed = || StepResult::Skipped(SkipReason::PersistFailed);
    match store.find(&hash) {
        Err(_) => return failed(),
        Ok(Some(existing_id)) => {
            if store.bump(&existing_id).is_err() {
                return failed();
            }
            state.last_hash = Some(hash);
            return StepResult::DedupedTo { existing_id };
        }
        Ok(None) => {}
    }
    let (kind, text_content, file_path) = match event {
        ClipboardEvent::Text(text) => (ClipboardItemKind::Text, Some(text), None),
        ClipboardEvent::Image {
            bytes,
            width,
            height,
        } => {
            // Never allocate another file while the previous failed attempt still exists.
            if !cleanup_pending_image(state, library_root) {
                return failed();
            }
            let Some(png) = encode_rgba_to_png(&bytes, width, height) else {
                return failed();
            };
            let relative = files::clipboard_image_relative_path(&uuid::Uuid::now_v7());
            state.pending_image = Some(relative.clone());
            if files::write_atomic(library_root, &relative, &png).is_err() {
                cleanup_pending_image(state, library_root);
                return failed();
            }
            (ClipboardItemKind::Image, None, Some(relative))
        }
    };
    let image = file_path.is_some();
    let item = NewClipboardItem {
        kind,
        text_content,
        file_path,
        content_hash: hash.clone(),
        source_app: source.map(|source| source.identifier),
        source_window_title: None,
    };
    match store.insert(item) {
        Ok(item_id) => {
            if image {
                state.pending_image = None;
            }
            state.last_hash = Some(hash);
            store.evict();
            StepResult::Saved { item_id }
        }
        Err(_) => {
            if image {
                cleanup_pending_image(state, library_root);
            }
            failed()
        }
    }
}

fn cleanup_pending_image(state: &mut WatcherState, root: &Path) -> bool {
    let Some(relative) = state.pending_image.as_ref() else {
        return true;
    };
    match files::remove_unpublished_clipboard_image(root, relative) {
        Ok(()) => {
            state.pending_image = None;
            true
        }
        Err(error) => {
            tracing::warn!(%error, "failed clipboard image cleanup; retaining pending path");
            false
        }
    }
}

fn hash_of_event(event: &ClipboardEvent) -> String {
    match event {
        ClipboardEvent::Text(t) => crate::hasher::hash_text(t),
        ClipboardEvent::Image { bytes, .. } => crate::hasher::hash_image_bytes(bytes),
    }
}

/// Encode raw RGBA bytes (as returned by arboard) to a PNG byte vector.
/// Returns `None` if the dimensions don't match the byte length or encoding fails.
pub(crate) fn encode_rgba_to_png(bytes: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;

    let expected = (width as u64).checked_mul(height as u64)?.checked_mul(4)?;
    if bytes.len() as u64 != expected {
        return None;
    }

    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(
            bytes,
            width as u32,
            height as u32,
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(out)
}

#[cfg(test)]
#[path = "watcher_tests.rs"]
mod tests;
