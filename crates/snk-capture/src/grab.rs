use std::io::Cursor;

use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use xcap::Monitor;

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
