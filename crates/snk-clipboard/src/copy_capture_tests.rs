use std::path::Path;

use super::*;
use serial_test::serial;

struct TestSink<F>(F);

impl<F: FnMut(usize, usize, Vec<u8>) -> Result<()>> ImageSink for TestSink<F> {
    fn write(&mut self, width: usize, height: usize, rgba: Vec<u8>) -> Result<()> {
        (self.0)(width, height, rgba)
    }
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .unwrap()
        .save(path)
        .unwrap();
}

fn unexpected_sink() -> Result<SystemClipboard> {
    panic!("clipboard must not be opened")
}

#[test]
fn loads_png_as_rgba() {
    let root = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir_in(root.path()).unwrap();
    let path = folder.path().join("capture.png");
    let bytes = vec![10, 20, 30, 40, 50, 60, 70, 80];
    write_png(&path, 2, 1, &bytes);
    let image = load_rgba(&path).unwrap();
    assert_eq!(image.dimensions(), (2, 1));
    assert_eq!(image.into_raw(), bytes);

    let relative = path.strip_prefix(root.path()).unwrap().to_str().unwrap();
    copy_to_sink(root.path(), relative, || {
        Ok(TestSink(|width, height, rgba| {
            assert_eq!((width, height), (2, 1));
            assert_eq!(rgba, bytes);
            Ok(())
        }))
    })
    .unwrap();
}

#[test]
fn rejects_absolute_path() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let outside = parent.path().join("outside.png");
    write_png(&outside, 1, 1, &[1, 2, 3, 4]);
    let result = copy_to_sink(&root, outside.to_str().unwrap(), unexpected_sink);
    assert!(matches!(result, Err(ClipboardError::PasteFailed { .. })));
}

#[test]
fn rejects_parent_dir_path() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    std::fs::create_dir(&root).unwrap();
    write_png(&parent.path().join("outside.png"), 1, 1, &[1, 2, 3, 4]);
    let result = copy_to_sink(&root, "../outside.png", unexpected_sink);
    assert!(matches!(result, Err(ClipboardError::PasteFailed { .. })));
}

#[test]
fn missing_file_returns_error() {
    let root = tempfile::tempdir().unwrap();
    let result = copy_to_sink(root.path(), "missing.png", unexpected_sink);
    assert!(matches!(result, Err(ClipboardError::PasteFailed { reason }) if !reason.is_empty()));
}

#[test]
fn undecodable_file_returns_error() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("bad.png"), b"not an image").unwrap();
    let result = copy_to_sink(root.path(), "bad.png", unexpected_sink);
    assert!(matches!(result, Err(ClipboardError::PasteFailed { reason }) if !reason.is_empty()));
}

#[test]
#[serial(skip_set)]
fn marks_own_write_as_emitted() {
    let root = tempfile::tempdir().unwrap();
    let bytes = uuid::Uuid::now_v7().as_bytes().to_vec();
    write_png(&root.path().join("unique.png"), 2, 2, &bytes);
    assert!(!skip_set::should_skip(skip_set::hash_content(&bytes)));
    copy_to_sink(root.path(), "unique.png", || {
        Ok(TestSink(|width, height, rgba: Vec<u8>| {
            assert_eq!((width, height), (2, 2));
            assert_eq!(rgba, bytes);
            assert!(skip_set::should_skip(skip_set::hash_content(&rgba)));
            Ok(())
        }))
    })
    .unwrap();
}

#[test]
fn clipboard_open_failure_returns_access_error() {
    let root = tempfile::tempdir().unwrap();
    write_png(&root.path().join("capture.png"), 1, 1, &[9, 8, 7, 6]);
    let result = copy_to_sink(root.path(), "capture.png", || -> Result<SystemClipboard> {
        Err(ClipboardError::Access {
            message: "open failed".into(),
        })
    });
    assert!(matches!(result, Err(ClipboardError::Access { .. })));
}

#[test]
fn clipboard_write_failure_returns_access_error() {
    let root = tempfile::tempdir().unwrap();
    write_png(&root.path().join("capture.png"), 1, 1, &[9, 8, 7, 6]);
    let result = copy_to_sink(root.path(), "capture.png", || {
        Ok(TestSink(|_, _, _| {
            Err(ClipboardError::Access {
                message: "write failed".into(),
            })
        }))
    });
    assert!(matches!(result, Err(ClipboardError::Access { .. })));
}

#[test]
#[ignore = "writes the real OS clipboard; run with -- --ignored on an interactive desktop"]
fn writes_image_to_system_clipboard() {
    let root = tempfile::tempdir().unwrap();
    let bytes = uuid::Uuid::now_v7().as_bytes().to_vec();
    write_png(&root.path().join("unique.png"), 2, 2, &bytes);
    copy_to_sink(root.path(), "unique.png", SystemClipboard::open).unwrap();

    let mut clipboard = arboard::Clipboard::new().unwrap();
    let image = clipboard.get_image().unwrap();
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(image.bytes.as_ref(), bytes);
}
