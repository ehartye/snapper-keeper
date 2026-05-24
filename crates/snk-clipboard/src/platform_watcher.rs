//! Per-OS event-driven clipboard observation. macOS uses polling
//! (handled directly in `watcher.rs`); Windows uses
//! AddClipboardFormatListener + WM_CLIPBOARDUPDATE.

#[cfg(target_os = "windows")]
pub mod windows {
    use std::sync::Arc;

    use snk_library::Db;

    pub fn start(_db: Arc<Db>, _library_root: std::path::PathBuf) {
        // Placeholder until Task 13.
    }
}
