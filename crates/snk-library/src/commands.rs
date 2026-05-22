use tauri::{Runtime, State};

use crate::captures::{self, Capture, ListCapturesQuery};
use crate::clipboard::{self, ClipboardItem, ListClipboardQuery};
use crate::plugin::LibraryState;
use crate::search::{self, SearchResult};
use crate::tags::{self, Tag};
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
pub fn set_capture_pinned<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
    pinned: bool,
) -> Result<()> {
    captures::set_pinned(&state.db, &id, pinned)
}

#[tauri::command]
pub fn hard_delete_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    captures::hard_delete(&state.db, &state.root, &id)
}

#[tauri::command]
pub fn purge_trash<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
) -> Result<u32> {
    captures::purge_trash(&state.db, &state.root)
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

#[tauri::command]
pub fn list_tags<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
) -> Result<Vec<Tag>> {
    tags::list(&state.db)
}

#[tauri::command]
pub fn create_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    name: String,
    color: String,
) -> Result<Tag> {
    tags::create(&state.db, &name, &color)
}

#[tauri::command]
pub fn update_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
    name: String,
    color: String,
) -> Result<Tag> {
    tags::update(&state.db, &id, &name, &color)
}

#[tauri::command]
pub fn delete_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    id: String,
) -> Result<()> {
    tags::delete(&state.db, &id)
}

#[tauri::command]
pub fn assign_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
    tag_id: String,
) -> Result<()> {
    tags::assign(&state.db, &capture_id, &tag_id)
}

#[tauri::command]
pub fn remove_tag<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
    tag_id: String,
) -> Result<()> {
    tags::remove(&state.db, &capture_id, &tag_id)
}

#[tauri::command]
pub fn list_capture_tags<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
) -> Result<Vec<Tag>> {
    tags::list_for_capture(&state.db, &capture_id)
}

#[tauri::command]
pub fn get_setting<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    key: String,
) -> Result<Option<serde_json::Value>> {
    crate::settings::get(&state.db, &key)
}

#[tauri::command]
pub fn set_setting<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    key: String,
    value: serde_json::Value,
) -> Result<()> {
    crate::settings::set(&state.db, &key, &value)
}
