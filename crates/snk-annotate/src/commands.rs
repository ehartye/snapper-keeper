use tauri::{Emitter, Runtime, State};

use snk_library::plugin::LibraryState;
use snk_library::{captures, files, Capture, NewCapture};

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

#[tauri::command]
pub fn derive_capture<R: Runtime>(
    state: State<'_, LibraryState>,
    app: tauri::AppHandle<R>,
    parent_id: String,
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    state_json: String,
) -> Result<Capture> {
    let parent = captures::get(&state.db, &parent_id)?;

    // Pre-generate the uuid so the on-disk filename matches the DB row.
    let new_id = uuid::Uuid::now_v7();
    let relative = files::capture_relative_path(&new_id, "png");
    files::write_atomic(&state.root, &relative, &png_data)?;

    // Insert with the pre-chosen id, inheriting source metadata from
    // the parent so the new capture's "where it came from" stays
    // honest. Tags and pinned start fresh per spec.
    let new_capture = captures::insert_with_id(
        &state.db,
        new_id,
        NewCapture {
            file_path: relative,
            width,
            height,
            source_app: parent.source_app.clone(),
            source_window_title: parent.source_window_title.clone(),
            monitor: parent.monitor.clone(),
        },
    )?;

    captures::set_annotation_state(&state.db, &new_capture.id, &state_json)?;

    // Mirror how snk-capture announces fresh captures — snk-ocr listens
    // for this event and runs OCR async on the new file.
    let _ = app.emit("capture:saved", &new_capture.id);

    captures::get(&state.db, &new_capture.id).map_err(Into::into)
}
