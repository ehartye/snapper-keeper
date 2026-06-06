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
    window: tauri::WebviewWindow<R>,
    id: String,
) -> Result<()> {
    crate::authz::authorize(
        &window,
        "hard_delete_capture",
        crate::authz::HARD_DELETE_CAPTURE_WINDOWS,
        &id,
    )?;
    captures::hard_delete(&state.db, &state.root, &id)
}

#[tauri::command]
pub fn purge_trash<R: Runtime>(
    state: State<'_, LibraryState>,
    window: tauri::WebviewWindow<R>,
) -> Result<u32> {
    crate::authz::authorize(
        &window,
        "purge_trash",
        crate::authz::PURGE_TRASH_WINDOWS,
        "",
    )?;
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
    window: tauri::WebviewWindow<R>,
    key: String,
    value: serde_json::Value,
) -> Result<()> {
    // Log the key (non-sensitive) but never the value — values can carry
    // user data and must not land in security-events.log.
    crate::authz::authorize(
        &window,
        "set_setting",
        crate::authz::SET_SETTING_WINDOWS,
        &key,
    )?;
    crate::settings::set(&state.db, &key, &value)
}

/// Settings key the UI persists the active theme under (mirrors
/// `SETTING_KEY` in app/src/lib/theme.ts).
const THEME_SETTING_KEY: &str = "theme";

/// Read the persisted theme id, if any. Split out for unit testing.
pub fn read_theme(db: &crate::Db) -> Result<Option<String>> {
    Ok(crate::settings::get(db, THEME_SETTING_KEY)?.and_then(|v| v.as_str().map(str::to_string)))
}

/// Narrow, read-only theme accessor. Every window applies the active theme on
/// load (App.tsx's WindowRouter calls useTheme for all windows), but only the
/// library/settings windows may *change* a setting. This command exposes the
/// theme read alone so non-privileged windows (annotate, clipboard popup,
/// capture overlay) can theme themselves without being granted broad
/// `get_setting` access. Writing the theme still goes through `set_setting`,
/// which is authorization-gated to the library/settings windows.
#[tauri::command]
pub fn get_theme(state: State<'_, LibraryState>) -> Result<Option<String>> {
    read_theme(&state.db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_theme_returns_stored_value_then_none_when_unset() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::Db::open(&dir.path().join("t.db")).unwrap();

        assert_eq!(read_theme(&db).unwrap(), None, "unset theme reads as None");

        crate::settings::set(
            &db,
            THEME_SETTING_KEY,
            &serde_json::Value::String("memphis-dark".into()),
        )
        .unwrap();
        assert_eq!(read_theme(&db).unwrap(), Some("memphis-dark".to_string()));
    }

    #[test]
    fn read_theme_ignores_non_string_values() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::Db::open(&dir.path().join("t.db")).unwrap();
        crate::settings::set(&db, THEME_SETTING_KEY, &serde_json::Value::Bool(true)).unwrap();
        assert_eq!(
            read_theme(&db).unwrap(),
            None,
            "non-string theme reads as None"
        );
    }
}
