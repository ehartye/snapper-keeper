use tauri::{Runtime, State};

use crate::captures::{self, Capture, ListCapturesQuery};
use crate::plugin::LibraryState;
use crate::Result;

#[tauri::command]
pub fn list_captures<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    query: Option<ListCapturesQuery>,
) -> Result<Vec<Capture>> {
    captures::list(&state.db, query.unwrap_or_default())
}

#[tauri::command]
pub fn get_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<Capture> {
    captures::get(&state.db, &id)
}
