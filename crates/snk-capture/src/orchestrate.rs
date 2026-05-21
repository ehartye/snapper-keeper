use std::sync::Arc;

use snk_library::{captures, files, Capture, Db, NewCapture};
use uuid::Uuid;

use crate::foreground::get_foreground_info;
use crate::grab::{self, GrabResult};
use crate::Result;

pub fn capture_full_screen(db: &Arc<Db>, library_root: &std::path::Path) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab::grab_primary_monitor()?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

pub fn capture_window(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    window_id: u32,
) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab::grab_window(window_id)?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

pub fn capture_region(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    monitor_id: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<Capture> {
    let fg = get_foreground_info();
    let GrabResult {
        png_bytes,
        width,
        height,
        monitor_name,
    } = grab::grab_region(monitor_id, x, y, w, h)?;
    persist(db, library_root, &png_bytes, width, height, Some(monitor_name), fg)
}

fn persist(
    db: &Arc<Db>,
    library_root: &std::path::Path,
    png_bytes: &[u8],
    width: u32,
    height: u32,
    monitor: Option<String>,
    fg: Option<crate::foreground::ForegroundInfo>,
) -> Result<Capture> {
    let id = Uuid::now_v7();
    let relative = files::capture_relative_path(&id, "png");
    let _full = files::write_atomic(library_root, &relative, png_bytes)?;
    let row = captures::insert(
        db,
        NewCapture {
            file_path: relative,
            width,
            height,
            source_app: fg.as_ref().map(|f| f.app_name.clone()),
            source_window_title: fg.as_ref().map(|f| f.window_title.clone()),
            monitor,
        },
    )?;
    Ok(row)
}
