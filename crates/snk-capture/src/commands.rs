use snk_library::{plugin::LibraryState, Capture};
use tauri::{Runtime, State};

use crate::Result;

#[tauri::command]
pub fn capture_full_screen<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
) -> Result<Capture> {
    crate::orchestrate::capture_full_screen(&state.db, &state.root)
}
