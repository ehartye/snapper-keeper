use arboard::Clipboard;
use tauri::{Runtime, State};
use tracing::info;

use snk_library::clipboard;
use snk_library::plugin::LibraryState;

use crate::caret;
use crate::paste;
use crate::watcher;
use crate::Result;

#[tauri::command]
pub fn paste_item<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    let item = clipboard::get(&state.db, &id)?;

    let mut clip = Clipboard::new()?;

    watcher::mark_skip_next();

    if let Some(ref text) = item.text_content {
        clip.set_text(text)?;
    }

    std::thread::sleep(std::time::Duration::from_millis(50));

    paste::synthesize_paste()?;

    clipboard::bump_timestamp(&state.db, &id)?;

    info!(id = %id, "pasted clipboard item");
    Ok(())
}

#[tauri::command]
pub fn show_popup<R: Runtime>(_app: tauri::AppHandle<R>) -> Result<crate::caret::CaretPosition> {
    let pos = caret::resolve_popup_position();
    Ok(pos)
}
