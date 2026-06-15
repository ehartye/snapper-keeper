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
        ..
    } = grab::grab_primary_monitor()?;
    persist(
        db,
        library_root,
        &png_bytes,
        width,
        height,
        Some(monitor_name),
        fg,
    )
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
        ..
    } = grab::grab_window(window_id)?;
    persist(
        db,
        library_root,
        &png_bytes,
        width,
        height,
        Some(monitor_name),
        fg,
    )
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
        ..
    } = grab::grab_region(monitor_id, x, y, w, h)?;
    persist(
        db,
        library_root,
        &png_bytes,
        width,
        height,
        Some(monitor_name),
        fg,
    )
}

/// Write a captured PNG to the library on disk and insert the row in SQLite.
/// Public so it can be unit-tested without a real monitor.
pub fn persist(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foreground::ForegroundInfo;

    /// Returns the `TempDir` along with the `Arc<Db>` so the caller can
    /// bind both — `Drop` then runs at end of test instead of relying on
    /// the older `std::mem::forget(dir)` leak.
    fn fresh_db() -> (tempfile::TempDir, Arc<Db>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sk.db");
        let db = Arc::new(Db::open(&path).unwrap());
        (dir, db)
    }

    fn small_png() -> Vec<u8> {
        // 2x1 red pixel encoded so write_atomic doesn't fall over.
        let rgba = [255u8, 0, 0, 255, 255, 0, 0, 255];
        crate::grab::encode_rgba_to_png(&rgba, 2, 1).unwrap()
    }

    #[test]
    fn persist_writes_file_and_db_row() {
        let tmp = tempfile::tempdir().unwrap();
        let (_tmp_dir, db) = fresh_db();
        let row = persist(
            &db,
            tmp.path(),
            &small_png(),
            2,
            1,
            Some("Mon0".into()),
            Some(ForegroundInfo {
                app_name: "Firefox".into(),
                window_title: "github".into(),
            }),
        )
        .unwrap();
        assert_eq!(row.width, 2);
        assert_eq!(row.height, 1);
        assert_eq!(row.source_app.as_deref(), Some("Firefox"));
        assert_eq!(row.source_window_title.as_deref(), Some("github"));
        assert_eq!(row.monitor.as_deref(), Some("Mon0"));
        assert!(tmp.path().join(&row.file_path).exists());
    }

    #[test]
    fn persist_handles_missing_foreground_info() {
        let tmp = tempfile::tempdir().unwrap();
        let (_tmp_dir, db) = fresh_db();
        let row = persist(&db, tmp.path(), &small_png(), 2, 1, None, None).unwrap();
        assert!(row.source_app.is_none());
        assert!(row.source_window_title.is_none());
        assert!(row.monitor.is_none());
    }
}
