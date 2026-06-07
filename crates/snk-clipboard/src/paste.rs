use crate::Result;

pub fn synthesize_paste() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        synthesize_paste_windows()
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

#[cfg(target_os = "windows")]
fn synthesize_paste_windows() -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };

    const VK_CONTROL: VIRTUAL_KEY = VIRTUAL_KEY(0x11);
    const VK_V: VIRTUAL_KEY = VIRTUAL_KEY(0x56);

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != 4 {
            return Err(crate::ClipboardError::PasteFailed {
                reason: format!("SendInput returned {sent}, expected 4"),
            });
        }
    }
    Ok(())
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
