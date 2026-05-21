use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CaretPosition {
    pub x: i32,
    pub y: i32,
}

/// Last-resort popup origin when neither caret nor cursor APIs return a
/// usable position — places it near the top-left of the primary screen so
/// the user can still see and interact with it.
const POPUP_FALLBACK: CaretPosition = CaretPosition { x: 100, y: 100 };

pub fn get_caret_position() -> Option<CaretPosition> {
    #[cfg(target_os = "windows")]
    {
        get_caret_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_caret_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

pub fn get_cursor_position() -> Option<CaretPosition> {
    #[cfg(target_os = "windows")]
    {
        get_cursor_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_cursor_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

pub fn resolve_popup_position() -> CaretPosition {
    get_caret_position()
        .or_else(get_cursor_position)
        .unwrap_or(POPUP_FALLBACK)
}

#[cfg(target_os = "windows")]
fn get_caret_windows() -> Option<CaretPosition> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};

    unsafe {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(0, &mut info).is_ok() {
            let pt = POINT {
                x: info.rcCaret.left,
                y: info.rcCaret.bottom,
            };
            if pt.x != 0 || pt.y != 0 {
                return Some(CaretPosition { x: pt.x, y: pt.y });
            }
        }
        None
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_windows() -> Option<CaretPosition> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            return Some(CaretPosition { x: pt.x, y: pt.y });
        }
        None
    }
}

#[cfg(target_os = "macos")]
fn get_caret_macos() -> Option<CaretPosition> {
    // AXUIElement caret detection requires Accessibility permission.
    // Deferred to a future refinement — use cursor fallback for v1.
    None
}

#[cfg(target_os = "macos")]
fn get_cursor_macos() -> Option<CaretPosition> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let loc = event.location();
    Some(CaretPosition {
        x: loc.x as i32,
        y: loc.y as i32,
    })
}
