use std::path::{Path, PathBuf};

use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::Result;

/// Compute the relative path a new capture file should live at:
///   captures/YYYY/MM/<uuid>.png
pub fn capture_relative_path(id: &Uuid, ext: &str) -> PathBuf {
    let now = Utc::now();
    PathBuf::from("captures")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{id}.{ext}"))
}

/// Compute the relative path a new clipboard image file should live at:
///   clipboard/YYYY/MM/<uuid>.png
pub fn clipboard_image_relative_path(id: &Uuid) -> PathBuf {
    let now = Utc::now();
    PathBuf::from("clipboard")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{id}.png"))
}

/// Atomic-ish file write: write to <path>.tmp, fsync, rename.
/// Rename is atomic on the same filesystem on POSIX and on NTFS.
pub fn write_atomic(library_root: &Path, relative: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let full = library_root.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = full.with_extension(format!(
        "{}.tmp",
        full.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &full)?;
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_uses_year_month_dirs() {
        let id = Uuid::now_v7();
        let p = capture_relative_path(&id, "png");
        let s = p.to_string_lossy();
        assert!(s.starts_with("captures/") || s.starts_with("captures\\"));
        assert!(s.ends_with(".png"));
    }

    #[test]
    fn write_atomic_writes_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let rel = Path::new("captures/2026/05/x.png");
        let full = write_atomic(dir.path(), rel, b"hello").unwrap();
        assert!(full.exists());
        let read = std::fs::read(&full).unwrap();
        assert_eq!(read, b"hello");
        // tmp should be gone
        assert!(!full.with_extension("png.tmp").exists());
    }

    #[test]
    fn write_atomic_handles_extensionless_filenames() {
        let dir = tempfile::tempdir().unwrap();
        let rel = Path::new("captures/no-ext-file");
        let full = write_atomic(dir.path(), rel, b"hi").unwrap();
        assert!(full.exists());
        assert_eq!(std::fs::read(&full).unwrap(), b"hi");
    }

    #[test]
    fn write_atomic_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let rel = Path::new("a.png");
        write_atomic(dir.path(), rel, b"first").unwrap();
        let full = write_atomic(dir.path(), rel, b"second").unwrap();
        assert_eq!(std::fs::read(&full).unwrap(), b"second");
    }

    #[test]
    fn clipboard_image_relative_path_uses_clipboard_year_month() {
        let id = Uuid::now_v7();
        let p = clipboard_image_relative_path(&id);
        let s = p.to_string_lossy();
        assert!(s.starts_with("clipboard/") || s.starts_with("clipboard\\"));
        assert!(s.ends_with(".png"));
    }

    #[test]
    fn relative_paths_embed_the_uuid_as_filename() {
        let id = Uuid::now_v7();
        let p = capture_relative_path(&id, "jpg");
        let fname = p.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(fname, format!("{id}.jpg"));
    }
}
