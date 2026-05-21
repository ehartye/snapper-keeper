use tauri::{Runtime, State};

use crate::captures::{self, Capture, ListCapturesQuery};
use crate::clipboard::{self, ClipboardItem, ListClipboardQuery};
use crate::plugin::LibraryState;
use crate::search::{self, SearchResult};
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

#[tauri::command]
pub fn soft_delete_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    captures::soft_delete(&state.db, &id)
}

#[tauri::command]
pub fn list_clipboard_items<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    query: Option<ListClipboardQuery>,
) -> Result<Vec<ClipboardItem>> {
    clipboard::list(&state.db, query.unwrap_or_default())
}

#[tauri::command]
pub fn get_clipboard_item<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<ClipboardItem> {
    clipboard::get(&state.db, &id)
}

#[tauri::command]
pub fn toggle_clipboard_pin<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
    pinned: bool,
) -> Result<()> {
    clipboard::set_pinned(&state.db, &id, pinned)
}

#[tauri::command]
pub fn search_library<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>> {
    search::search(&state.db, &query, limit.unwrap_or(50))
}
