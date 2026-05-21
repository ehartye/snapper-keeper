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

    captures::set_annotated_path(&state.db, &capture_id, &annotated_str)?;
    captures::get(&state.db, &capture_id).map_err(Into::into)
}
