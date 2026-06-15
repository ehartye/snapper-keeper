use xcap::{Monitor, Window};

use crate::{
    grab::{clamp_region, encode_rgba_to_png, GrabResult, WindowInfo},
    CaptureError, Result,
};

use super::ScreenshotBackend;

pub(super) struct XcapBackend;

fn resolve_requested_monitor_position(monitor_ids: &[u32], requested: u32) -> Option<usize> {
    let index = requested as usize;
    if index < monitor_ids.len() {
        return Some(index);
    }
    monitor_ids.iter().position(|id| *id == requested)
}

fn select_monitor(monitor_id: Option<u32>) -> Result<Monitor> {
    let mut monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitors);
    }

    if let Some(id) = monitor_id {
        let monitor_ids: Vec<u32> = monitors
            .iter()
            .map(|m| m.id().unwrap_or(u32::MAX))
            .collect();
        if let Some(pos) = resolve_requested_monitor_position(&monitor_ids, id) {
            return Ok(monitors.swap_remove(pos));
        }
    }

    if let Some(pos) = monitors
        .iter()
        .position(|m| m.is_primary().unwrap_or(false))
    {
        return Ok(monitors.swap_remove(pos));
    }

    monitors.pop().ok_or(CaptureError::NoMonitors)
}

impl ScreenshotBackend for XcapBackend {
    fn grab_primary_monitor(&self) -> Result<GrabResult> {
        let primary = select_monitor(None)?;
        let image = primary.capture_image()?;
        let (w, h) = (image.width(), image.height());
        let name = primary.name().unwrap_or_default();
        let png_bytes = encode_rgba_to_png(image.as_raw(), w, h)?;
        Ok(GrabResult {
            png_bytes,
            width: w,
            height: h,
            monitor_name: name,
            display_frame: None,
            display_index: None,
        })
    }

    fn grab_monitor(&self, monitor_id: u32) -> Result<GrabResult> {
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
            display_frame: None,
            display_index: None,
        })
    }

    fn grab_window(&self, window_id: u32) -> Result<GrabResult> {
        let windows = Window::all()?;
        let target = windows
            .into_iter()
            .find(|w| w.id().unwrap_or(0) == window_id)
            .ok_or(CaptureError::WindowNotFound { id: window_id })?;
        let monitor_name = target
            .current_monitor()
            .ok()
            .and_then(|m| m.name().ok())
            .unwrap_or_default();
        let image = target.capture_image()?;
        let (w, h) = (image.width(), image.height());
        let png_bytes = encode_rgba_to_png(image.as_raw(), w, h)?;
        Ok(GrabResult {
            png_bytes,
            width: w,
            height: h,
            monitor_name,
            display_frame: None,
            display_index: None,
        })
    }

    fn grab_region(&self, monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult> {
        let mon = select_monitor(Some(monitor_id))?;
        let monitor_name = mon.name().unwrap_or_default();
        let full_image = mon.capture_image()?;
        let (x, y, w, h) = clamp_region(full_image.width(), full_image.height(), x, y, w, h)
            .ok_or_else(|| CaptureError::Os {
                message: "region has zero area".into(),
            })?;
        let cropped = image::imageops::crop_imm(&full_image, x, y, w, h).to_image();
        let (cw, ch) = (cropped.width(), cropped.height());
        let png_bytes = encode_rgba_to_png(cropped.as_raw(), cw, ch)?;
        Ok(GrabResult {
            png_bytes,
            width: cw,
            height: ch,
            monitor_name,
            display_frame: None,
            display_index: None,
        })
    }

    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_requested_monitor_position_prefers_index_over_id_match() {
        let ids = vec![1, 2];
        // requested=1 could mean index 1 (second monitor) or id 1 (first monitor).
        // We prefer index semantics for frontend callers that pass monitor index.
        assert_eq!(resolve_requested_monitor_position(&ids, 1), Some(1));
    }

    #[test]
    fn resolve_requested_monitor_position_falls_back_to_id_when_index_is_out_of_range() {
        let ids = vec![42, 77];
        assert_eq!(resolve_requested_monitor_position(&ids, 77), Some(1));
    }
}
