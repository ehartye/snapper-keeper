#[cfg(target_os = "macos")]
mod macos_sck;
#[cfg(not(target_os = "macos"))]
mod xcap_adapter;

use crate::{
    grab::{GrabResult, WindowInfo},
    Result,
};

pub trait ScreenshotBackend {
    fn grab_primary_monitor(&self) -> Result<GrabResult>;
    fn grab_monitor(&self, monitor_id: u32) -> Result<GrabResult>;
    fn grab_window(&self, window_id: u32) -> Result<GrabResult>;
    fn grab_region(&self, monitor_id: u32, x: u32, y: u32, w: u32, h: u32) -> Result<GrabResult>;
    fn list_capturable_windows(&self) -> Result<Vec<WindowInfo>>;
}

#[cfg(target_os = "macos")]
pub fn platform_backend() -> Box<dyn ScreenshotBackend> {
    Box::new(macos_sck::ScreenCaptureKitBackend)
}

#[cfg(not(target_os = "macos"))]
pub fn platform_backend() -> Box<dyn ScreenshotBackend> {
    Box::new(xcap_adapter::XcapBackend)
}
