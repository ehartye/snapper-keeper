use tauri::{Runtime, State};

use snk_library::plugin::LibraryState;
use snk_library::{captures, files, Capture};

use crate::Result;

#[tauri::command]
pub fn save_annotation<R: Runtime>(
    state: State<'_, LibraryState>,
    _app: tauri::AppHandle<R>,
    capture_id: String,
    png_data: Vec<u8>,
    state_json: String,
) -> Result<Capture> {
    let capture = captures::get(&state.db, &capture_id)?;

    let annotated_relative = captures::annotated_relative_path(&capture.file_path);
    files::write_atomic(&state.root, &annotated_relative, &png_data)?;

    let annotated_str = annotated_relative
        .to_str()
        .ok_or_else(|| crate::AnnotateError::InvalidInput {
            reason: "non-utf8 annotated path".into(),
        })?
        .to_string();

    // Both writes go to the same Db wrapper; rusqlite execs are individually
    // atomic. We accept the small window between them — set_annotated_path
    // is harmless on its own (the PNG is already on disk) and the next
    // save will overwrite both. Wrapping in an explicit transaction would
    // require a Db API change we don't need today.
    captures::set_annotated_path(&state.db, &capture_id, &annotated_str)?;
    captures::set_annotation_state(&state.db, &capture_id, &state_json)?;

    captures::get(&state.db, &capture_id).map_err(Into::into)
}
