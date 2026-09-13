use std::io::Cursor;

use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use xcap::{Monitor, Window};

use crate::Result;

#[derive(Clone)]
pub struct GrabResult {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub monitor_name: String,
}

fn resolve_requested_monitor_position(monitor_ids: &[u32], requested: u32) -> Option<usize> {
    monitor_ids.iter().position(|id| *id == requested)
}

pub(crate) fn select_monitor(monitor_id: Option<u32>) -> Result<Monitor> {
    let mut monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(crate::CaptureError::NoMonitors);
    }

    if let Some(id) = monitor_id {
        let monitor_ids = monitors
            .iter()
            .map(Monitor::id)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if let Some(pos) = resolve_requested_monitor_position(&monitor_ids, id) {
            return Ok(monitors.swap_remove(pos));
        }
        return Err(crate::CaptureError::MonitorNotFound { id });
    }

    if let Some(pos) = monitors
        .iter()
        .position(|m| m.is_primary().unwrap_or(false))
    {
        return Ok(monitors.swap_remove(pos));
    }

    monitors.pop().ok_or(crate::CaptureError::NoMonitors)
}

pub fn grab_primary_monitor() -> Result<GrabResult> {
    let primary = select_monitor(None)?;

    let image = primary.capture_image()?;
    let (w, h) = (image.width(), image.height());
    let name = primary.name().unwrap_or_default();

    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(image.as_raw(), w, h, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
        width: w,
        height: h,
        monitor_name: name,
    })
}

pub fn grab_monitor(monitor_id: u32) -> Result<GrabResult> {
    let monitor = select_monitor(Some(monitor_id))?;
    let image = monitor.capture_image()?;
    let (w, h) = (image.width(), image.height());
    let name = monitor.name().unwrap_or_default();
    let png_bytes = encode_rgba_to_png(image.as_raw(), w, h)?;

    Ok(GrabResult {
        png_bytes,
        width: w,
        height: h,
        monitor_name: name,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u32,
    pub app_name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    let windows = Window::all()?;
    let infos = windows
        .into_iter()
        .filter(|w| {
            !w.is_minimized().unwrap_or(true)
                && w.width().unwrap_or(0) > 0
                && w.height().unwrap_or(0) > 0
        })
        .map(|w| WindowInfo {
            id: w.id().unwrap_or(0),
            app_name: w.app_name().unwrap_or_default(),
            title: w.title().unwrap_or_default(),
            width: w.width().unwrap_or(0),
            height: w.height().unwrap_or(0),
        })
        .collect();
    Ok(infos)
}

pub fn grab_window(window_id: u32) -> Result<GrabResult> {
    let windows = Window::all()?;
    let target = windows
        .into_iter()
        .find(|w| w.id().unwrap_or(0) == window_id)
        .ok_or(crate::CaptureError::WindowNotFound { id: window_id })?;

    let monitor_name = target
        .current_monitor()
        .ok()
        .and_then(|m| m.name().ok())
        .unwrap_or_default();
    let image = target.capture_image()?;
    let (w, h) = (image.width(), image.height());

    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(image.as_raw(), w, h, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
        width: w,
        height: h,
        monitor_name,
    })
}

/// Clamp a requested capture region against an image's bounds. Returns
/// `None` when the resulting region would have zero area (so callers can
/// surface a single error). Public so it can be unit-tested without a real
/// monitor.
pub fn clamp_region(
    img_w: u32,
    img_h: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Option<(u32, u32, u32, u32)> {
    let x = x.min(img_w.saturating_sub(1));
    let y = y.min(img_h.saturating_sub(1));
    let w = w.min(img_w.saturating_sub(x));
    let h = h.min(img_h.saturating_sub(y));
    if w == 0 || h == 0 {
        None
    } else {
        Some((x, y, w, h))
    }
}

/// Encode RGBA8 pixels into a PNG byte stream. Pulled out of the grab
/// functions so it's testable without a real monitor.
pub fn encode_rgba_to_png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(rgba, w, h, ColorType::Rgba8.into())?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_region_returns_none_for_zero_dimensions() {
        assert_eq!(clamp_region(100, 100, 0, 0, 0, 50), None);
        assert_eq!(clamp_region(100, 100, 0, 0, 50, 0), None);
        assert_eq!(clamp_region(0, 0, 0, 0, 10, 10), None);
    }

    #[test]
    fn clamp_region_caps_x_y_to_image_bounds() {
        // x=200 in a 100-wide image → clamped to 99 (img_w - 1)
        // w then = (100 - 99) = 1.
        let r = clamp_region(100, 100, 200, 50, 50, 50).unwrap();
        assert_eq!(r, (99, 50, 1, 50));
    }

    #[test]
    fn clamp_region_caps_w_h_to_remaining_image() {
        // x=80 in a 100-wide image, w=50 requested → only 20 fits.
        let r = clamp_region(100, 100, 80, 80, 50, 50).unwrap();
        assert_eq!(r, (80, 80, 20, 20));
    }

    #[test]
    fn clamp_region_passes_through_valid_region() {
        let r = clamp_region(800, 600, 100, 50, 200, 150).unwrap();
        assert_eq!(r, (100, 50, 200, 150));
    }

    #[test]
    fn encode_rgba_to_png_produces_valid_png_signature() {
        // 2x1 image: red, green
        let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255];
        let png = encode_rgba_to_png(&rgba, 2, 1).unwrap();
        // PNG magic bytes
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
        // image crate should round-trip
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 1);
    }

    #[test]
    fn encode_rgba_to_png_round_trips_a_small_image() {
        // 4x4 solid blue
        let mut rgba = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            rgba.extend_from_slice(&[0, 0, 255, 255]);
        }
        let png = encode_rgba_to_png(&rgba, 4, 4).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (4, 4));
        // every pixel should be solid blue
        for px in img.pixels() {
            assert_eq!(px.0, [0, 0, 255, 255]);
        }
    }

    #[test]
    fn resolve_requested_monitor_position_uses_native_id_even_when_it_is_an_index() {
        let ids = vec![1, 2];
        // Native id 1 must not be confused with enumeration index 1.
        assert_eq!(resolve_requested_monitor_position(&ids, 1), Some(0));
    }

    #[test]
    fn resolve_requested_monitor_position_survives_reordering_and_rejects_unknown() {
        let ids = vec![42, 77];
        assert_eq!(resolve_requested_monitor_position(&ids, 77), Some(1));
        assert_eq!(resolve_requested_monitor_position(&[77, 42], 42), Some(1));
        assert_eq!(resolve_requested_monitor_position(&[77, 42], 0), None);
    }
}
