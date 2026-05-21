use xcap::Window;

#[derive(Debug, Clone)]
pub struct ForegroundInfo {
    pub app_name: String,
    pub window_title: String,
}

pub fn get_foreground_info() -> Option<ForegroundInfo> {
    // Approximation: xcap::Window::all() does not guarantee z-order, so the first
    // non-minimized window is a best-effort foreground guess until we wrap native APIs.
    let windows = Window::all().ok()?;
    let win = windows.into_iter().find(|w| !w.is_minimized())?;
    Some(ForegroundInfo {
        app_name: win.app_name().to_string(),
        window_title: win.title().to_string(),
    })
}
