use serde::Serialize;
use xcap::Monitor;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use crate::CaptureError;
use crate::Result;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayFrame {
    pub coordinate_space: &'static str,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Display {
    /// Native display identity for this capture, never an enumeration index.
    pub id: u32,
    pub frame: DisplayFrame,
}

pub fn describe(monitor: &Monitor) -> Result<Display> {
    Ok(Display {
        id: monitor.id()?,
        frame: DisplayFrame {
            // xcap's macOS geometry comes from CGDisplayBounds (points).
            // Its Windows geometry comes from EnumDisplaySettings (pixels).
            coordinate_space: if cfg!(target_os = "macos") {
                "logical"
            } else {
                "physical"
            },
            x: monitor.x()?,
            y: monitor.y()?,
            width: monitor.width()?,
            height: monitor.height()?,
        },
    })
}

pub fn select_preview_monitor(id: Option<u32>) -> Result<Monitor> {
    if let Some(id) = id {
        return crate::grab::select_monitor(Some(id));
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let (x, y) = cursor_position()?;
        Ok(Monitor::from_point(x, y)?)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Linux remains a development convenience target. Preserve its primary
        // display fallback until it has a native cursor-coordinate adapter.
        crate::grab::select_monitor(None)
    }
}

#[cfg(target_os = "macos")]
fn cursor_position() -> Result<(i32, i32)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let event = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .and_then(CGEvent::new)
        .map_err(|_| CaptureError::Os {
            message: "read native cursor position".into(),
        })?;
    let point = event.location();
    Ok((point.x.floor() as i32, point.y.floor() as i32))
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Result<(i32, i32)> {
    use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.map_err(|e| CaptureError::Os {
        message: e.to_string(),
    })?;
    Ok((point.x, point.y))
}
