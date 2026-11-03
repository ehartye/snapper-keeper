use crate::Result;

#[cfg(target_os = "windows")]
#[path = "paste_windows.rs"]
mod windows;

pub struct PasteTarget {
    #[cfg(target_os = "windows")]
    inner: windows::Target,
    #[cfg(target_os = "macos")]
    inner: Option<i32>,
}

pub fn capture_target<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;
        windows::capture(
            app.get_webview_window("clipboard-popup")
                .and_then(|w| w.hwnd().ok()),
        )?;
    }
    #[cfg(target_os = "macos")]
    {
        use crate::paste_macos::MacSystem;
        use tauri::Manager;

        let system = crate::paste_macos::RealMac;
        let popup_visible = app
            .get_webview_window("clipboard-popup")
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);
        let mut target =
            crate::paste_macos::TARGET
                .lock()
                .map_err(|_| crate::ClipboardError::PasteFailed {
                    reason: "Paste target state unavailable.".into(),
                })?;
        crate::paste_macos::capture(
            popup_visible,
            system.frontmost_pid(),
            system.own_pid(),
            &mut target,
        );
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = app;
    Ok(())
}

pub fn prepare_paste<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<PasteTarget> {
    #[cfg(target_os = "windows")]
    {
        let popup = window
            .hwnd()
            .map_err(|e| crate::ClipboardError::PasteFailed {
                reason: e.to_string(),
            })?;
        Ok(PasteTarget {
            inner: windows::prepare(popup)?,
        })
    }
    #[cfg(target_os = "macos")]
    {
        let _ = window;
        Ok(PasteTarget {
            inner: crate::paste_macos::prepare_target(&crate::paste_macos::RealMac)?,
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = window;
        Ok(PasteTarget {})
    }
}

pub fn synthesize_paste(target: &PasteTarget) -> Result<()> {
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = target;
    #[cfg(target_os = "windows")]
    {
        windows::synthesize(target.inner)
    }
    #[cfg(target_os = "macos")]
    {
        crate::paste_macos::synthesize(&crate::paste_macos::RealMac, target.inner)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(crate::ClipboardError::PasteFailed {
            reason: "unsupported platform".into(),
        })
    }
}
