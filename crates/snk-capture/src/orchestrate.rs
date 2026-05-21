use std::sync::Arc;

use snk_library::{captures, files, Capture, Db, NewCapture};
use uuid::Uuid;

use crate::grab::{grab_primary_monitor, GrabResult};
use crate::Result;

/// Capture the primary monitor, write the PNG to disk, and insert a row.
/// Returns the persisted Capture row.
pub fn capture_full_screen(db: &Arc<Db>, library_root: &std::path::Path) -> Result<Capture> {
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab_primary_monitor()?;
    let id = Uuid::now_v7();
    let relative = files::capture_relative_path(&id, "png");
    let _full = files::write_atomic(library_root, &relative, &png_bytes)?;
    let row = captures::insert(
        db,
        NewCapture {
            file_path: relative,
            width,
            height,
            source_app: None,
            source_window_title: None,
            monitor: Some(monitor_name),
        },
    )?;
    Ok(row)
}
