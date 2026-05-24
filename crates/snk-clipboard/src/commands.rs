use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use tauri::{Runtime, State};
use tracing::info;

use snk_library::clipboard;
use snk_library::LibraryState;

use crate::caret;
use crate::paste;
use crate::watcher;
use crate::{ClipboardError, Result};

/// Wait between writing to the clipboard and synthesizing Ctrl/Cmd+V so the
/// target app's clipboard listener has time to observe the new content before
/// the paste keystroke arrives.
const PASTE_SETTLE: Duration = Duration::from_millis(50);

#[tauri::command]
pub fn paste_item<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    let item = clipboard::get(&state.db, &id)?;

    let text = item.text_content.as_deref().ok_or_else(|| {
        // Image paste needs PNG decode + arboard::ImageData (raw RGBA);
        // deferred until the popup can handle image rows end-to-end.
        ClipboardError::PasteFailed {
            reason: format!("paste for kind '{}' not yet supported", item.kind),
        }
    })?;

    let mut clip = Clipboard::new()?;
    watcher::mark_skip_next();
    clip.set_text(text)?;

    thread::sleep(PASTE_SETTLE);

    paste::synthesize_paste()?;
    clipboard::bump_timestamp(&state.db, &id)?;

    info!(id = %id, "pasted clipboard item");
    Ok(())
}

#[tauri::command]
pub fn show_popup<R: Runtime>(_app: tauri::AppHandle<R>) -> Result<crate::caret::CaretPosition> {
    Ok(caret::resolve_popup_position())
}
