use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use snk_library::{authz, captures, LibraryState};
use tauri::{Runtime, State};

use crate::{skip_set, ClipboardError, Result};

pub(crate) trait ImageSink {
    fn write(&mut self, width: usize, height: usize, rgba: Vec<u8>) -> Result<()>;
}

pub(crate) struct SystemClipboard(arboard::Clipboard);

impl SystemClipboard {
    fn open() -> Result<Self> {
        Ok(Self(arboard::Clipboard::new()?))
    }
}

impl ImageSink for SystemClipboard {
    fn write(&mut self, width: usize, height: usize, rgba: Vec<u8>) -> Result<()> {
        self.0.set_image(arboard::ImageData {
            width,
            height,
            bytes: Cow::Owned(rgba),
        })?;
        Ok(())
    }
}

pub(crate) fn resolve_path(root: &Path, file_path: &str) -> Result<PathBuf> {
    let path = Path::new(file_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ClipboardError::PasteFailed {
            reason: "capture path must contain only relative normal components".into(),
        });
    }
    Ok(root.join(path))
}

pub(crate) fn load_rgba(path: &Path) -> Result<image::RgbaImage> {
    let bytes = std::fs::read(path).map_err(|e| ClipboardError::PasteFailed {
        reason: format!("read image file: {e}"),
    })?;
    let img = image::load_from_memory(&bytes).map_err(|e| ClipboardError::PasteFailed {
        reason: format!("decode image: {e}"),
    })?;
    Ok(img.to_rgba8())
}

pub(crate) fn copy_to_sink<S: ImageSink>(
    root: &Path,
    file_path: &str,
    open: impl FnOnce() -> Result<S>,
) -> Result<()> {
    let path = resolve_path(root, file_path)?;
    let rgba = load_rgba(&path)?;
    let (width, height) = rgba.dimensions();
    let rgba_bytes = rgba.into_raw();
    skip_set::mark_emitted(skip_set::hash_content(&rgba_bytes));
    open()?.write(width as usize, height as usize, rgba_bytes)
}

#[tauri::command]
pub fn copy_capture_image<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    state: State<'_, LibraryState>,
    capture_id: String,
) -> Result<()> {
    authz::authorize(
        &window,
        "copy_capture_image",
        authz::COPY_CAPTURE_IMAGE_WINDOWS,
        &capture_id,
    )?;
    let capture = captures::get(&state.db, &capture_id)?;
    copy_to_sink(&state.root, &capture.file_path, SystemClipboard::open)
}

#[cfg(test)]
#[path = "copy_capture_tests.rs"]
mod tests;
