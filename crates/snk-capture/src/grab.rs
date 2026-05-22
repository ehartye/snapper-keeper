use std::io::Cursor;

use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use xcap::{Monitor, Window};

use crate::Result;

pub struct GrabResult {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub monitor_name: String,
}

pub fn grab_primary_monitor() -> Result<GrabResult> {
    let monitors = Monitor::all()?;
    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary())
        .or_else(|| Monitor::all().ok().and_then(|mut v| v.pop()))
        .ok_or(crate::CaptureError::NoMonitors)?;

    let image = primary.capture_image()?;
    let (w, h) = (image.width(), image.height());
    let name = primary.name().to_string();

    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(image.as_raw(), w, h, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
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
        .filter(|w| !w.is_minimized() && w.width() > 0 && w.height() > 0)
        .map(|w| WindowInfo {
            id: w.id(),
            app_name: w.app_name().to_string(),
            title: w.title().to_string(),
            width: w.width(),
            height: w.height(),
        })
        .collect();
    Ok(infos)
}

pub fn grab_window(window_id: u32) -> Result<GrabResult> {
    let windows = Window::all()?;
    let target = windows
        .into_iter()
        .find(|w| w.id() == window_id)
        .ok_or(crate::CaptureError::WindowNotFound { id: window_id })?;

    let monitor_name = target.current_monitor().name().to_string();
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

pub fn grab_region(monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult> {
    let monitors = Monitor::all()?;
    let mon = monitors
        .into_iter()
        .find(|m| m.id() == monitor_id)
        .or_else(|| Monitor::all().ok().and_then(|v| v.into_iter().find(|m| m.is_primary())))
        .or_else(|| Monitor::all().ok().and_then(|mut v| v.pop()))
        .ok_or(crate::CaptureError::NoMonitors)?;

    let monitor_name = mon.name().to_string();
    let full_image = mon.capture_image()?;

    let (x, y, w, h) =
        clamp_region(full_image.width(), full_image.height(), x, y, w, h).ok_or_else(|| {
            crate::CaptureError::Os {
                message: "region has zero area".into(),
            }
        })?;

    let cropped = image::imageops::crop_imm(&full_image, x, y, w, h).to_image();
    let (cw, ch) = (cropped.width(), cropped.height());
    let png_bytes = encode_rgba_to_png(cropped.as_raw(), cw, ch)?;

    Ok(GrabResult {
        png_bytes,
        width: cw,
        height: ch,
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
    fn grab_region_rejects_zero_area() {
        // Width 0 → zero-area region must be rejected even if the monitor
        // resolves (we fall back to the primary monitor for unknown ids).
        let result = grab_region(0, 0, 0, 0, 100);
        assert!(result.is_err());
    }

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
}
