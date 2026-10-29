use crate::Result;

#[cfg(target_os = "windows")]
#[path = "paste_windows.rs"]
mod windows;

pub struct PasteTarget {
    #[cfg(target_os = "windows")]
    inner: windows::Target,
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
    #[cfg(not(target_os = "windows"))]
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
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        Ok(PasteTarget {})
    }
}

pub fn synthesize_paste(target: &PasteTarget) -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    let _ = target;
    #[cfg(target_os = "windows")]
    {
        windows::synthesize(target.inner)
    }
    #[cfg(target_os = "macos")]
    {
        // Posting the synthetic Cmd+V is silently dropped by macOS unless the
        // process holds Accessibility permission. Check first and surface a
        // typed error the popup can act on, rather than no-op'ing invisibly.
        if !crate::permissions::accessibility_granted() {
            return Err(crate::ClipboardError::AccessibilityRequired);
        }
        synthesize_paste_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(crate::ClipboardError::PasteFailed {
            reason: "unsupported platform".into(),
        })
    }
}

#[cfg(target_os = "macos")]
fn synthesize_paste_macos() -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).map_err(|_| {
        crate::ClipboardError::PasteFailed {
            reason: "failed to create CGEventSource".into(),
        }
    })?;

    // 'v' key is keycode 9
    let key_v: CGKeyCode = 9;

    let key_down = CGEvent::new_keyboard_event(source.clone(), key_v, true).map_err(|_| {
        crate::ClipboardError::PasteFailed {
            reason: "failed to create key-down event".into(),
        }
    })?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);

    let key_up = CGEvent::new_keyboard_event(source, key_v, false).map_err(|_| {
        crate::ClipboardError::PasteFailed {
            reason: "failed to create key-up event".into(),
        }
    })?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);

    key_down.post(core_graphics::event::CGEventTapLocation::HID);
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}
