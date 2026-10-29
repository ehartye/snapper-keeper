use std::sync::Mutex;
use std::time::Duration;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow,
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use crate::paste_flow::{self, PasteChord, PasteDesktop};
use crate::{ClipboardError, Result};

#[derive(Clone, Copy)]
pub(super) struct Target {
    hwnd: usize,
    pid: u32,
    chord: PasteChord,
}

static TARGET: Mutex<Option<Target>> = Mutex::new(None);

fn failed(reason: impl Into<String>) -> ClipboardError {
    ClipboardError::PasteFailed {
        reason: reason.into(),
    }
}

pub(super) fn capture(popup: Option<HWND>) -> Result<()> {
    let mut saved = TARGET
        .lock()
        .map_err(|_| failed("Paste target state unavailable."))?;
    let hwnd = unsafe { GetForegroundWindow() };
    // Repeated hotkey presses while the popup owns focus must not replace the
    // original destination with the popup itself.
    if popup == Some(hwnd) {
        return Ok(());
    }
    // Each new popup invocation captures a fresh destination, including our own
    // library window; only the popup itself is excluded above.
    *saved = None;
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if hwnd.is_invalid() || pid == 0 {
            return Err(failed(
                "Open clipboard history while the destination app is focused.",
            ));
        }
        let mut class = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class);
        *saved = Some(Target {
            hwnd: hwnd.0 as usize,
            pid,
            chord: paste_flow::chord_for_class(&String::from_utf16_lossy(&class[..len as usize])),
        });
    }
    Ok(())
}

pub(super) fn prepare(popup: HWND) -> Result<Target> {
    let target = TARGET
        .lock()
        .map_err(|_| failed("Paste target state unavailable."))?
        .as_ref()
        .copied()
        .ok_or_else(|| failed("Reopen clipboard history from the destination app."))?;
    let desktop = Desktop(target);
    if !desktop.target_valid() {
        return Err(failed(
            "The destination closed. Reopen clipboard history there.",
        ));
    }
    if unsafe { GetForegroundWindow() } != popup {
        return Err(failed(
            "Clipboard history lost focus. Reopen it from the destination app.",
        ));
    }
    // Keep the popup focused while Enter/digit/shortcut keys are held. Hiding it
    // first would let key repeat reach a terminal (including an Enter key).
    paste_flow::wait_for_release(&desktop).map_err(failed)?;
    if unsafe { GetForegroundWindow() } != popup {
        return Err(failed(
            "Clipboard history lost focus. Reopen it from the destination app.",
        ));
    }
    Ok(target)
}

pub(super) fn synthesize(target: Target) -> Result<()> {
    paste_flow::paste_to_target(&mut Desktop(target), target.chord).map_err(failed)
}

struct Desktop(Target);
impl PasteDesktop for Desktop {
    fn target_valid(&self) -> bool {
        unsafe {
            let hwnd = HWND(self.0.hwnd as *mut _);
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            IsWindow(Some(hwnd)).as_bool() && pid == self.0.pid
        }
    }
    fn target_foreground(&self) -> bool {
        unsafe { GetForegroundWindow().0 as usize == self.0.hwnd }
    }
    fn keys_released(&self) -> bool {
        // Includes both sides of Ctrl/Shift/Alt/Win and selection keys. Never
        // manufacture key-up events for physical keys the user still holds.
        (0x08..=0xfe).all(|key| unsafe { GetAsyncKeyState(key) >= 0 })
    }
    fn focus_target(&mut self) {
        unsafe {
            let hwnd = HWND(self.0.hwnd as *mut _);
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
    fn pause(&self) {
        std::thread::sleep(Duration::from_millis(10));
    }
    fn send(&mut self, chord: PasteChord) -> std::result::Result<(), String> {
        if !self.target_valid() || !self.target_foreground() || !self.keys_released() {
            return Err(
                "The destination or keyboard state changed before paste. Try again.".into(),
            );
        }
        let (modifier, key) = match chord {
            PasteChord::CtrlV => (0x11, 0x56),
            PasteChord::ShiftInsert => (0x10, 0x2d),
        };
        let input = |vk, up| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    ..Default::default()
                },
            },
        };
        let inputs = [
            input(modifier, false),
            input(key, false),
            input(key, true),
            input(modifier, true),
        ];
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            // Release only our synthetic chord after a partial insertion; do not
            // retry the paste, which could duplicate content in the destination.
            let releases = [input(key, true), input(modifier, true)];
            unsafe {
                SendInput(&releases, std::mem::size_of::<INPUT>() as i32);
            }
            return Err(
                "Windows blocked automatic paste. For an elevated destination, paste manually."
                    .into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "paste_windows_tests.rs"]
mod tests;
