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
        .ok_or(crate::CaptureError::NoMonitors)?;

    let monitor_name = mon.name().to_string();
    let full_image = mon.capture_image()?;

    let x = x.min(full_image.width().saturating_sub(1));
    let y = y.min(full_image.height().saturating_sub(1));
    let w = w.min(full_image.width().saturating_sub(x));
    let h = h.min(full_image.height().saturating_sub(y));

    if w == 0 || h == 0 {
        return Err(crate::CaptureError::Os {
            message: "region has zero area".into(),
        });
    }

    let cropped = image::imageops::crop_imm(&full_image, x, y, w, h).to_image();
    let (cw, ch) = (cropped.width(), cropped.height());

    let mut buf = Cursor::new(Vec::with_capacity((cw * ch * 4) as usize / 2));
    PngEncoder::new(&mut buf).write_image(cropped.as_raw(), cw, ch, ColorType::Rgba8.into())?;

    Ok(GrabResult {
        png_bytes: buf.into_inner(),
        width: cw,
        height: ch,
        monitor_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grab_region_rejects_zero_area() {
        let result = grab_region(9999, 0, 0, 100, 100);
        assert!(result.is_err());
    }
}
