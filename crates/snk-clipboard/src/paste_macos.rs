#[cfg(target_os = "macos")]
use std::sync::Mutex;

use crate::paste_flow::{self, PasteChord, PasteDesktop};
use crate::{ClipboardError, Result};

pub(crate) trait MacSystem {
    fn accessibility_granted(&self) -> bool;
    fn keys_released(&self) -> bool;
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "Only native macOS capture reads AppKit state")
    )]
    fn frontmost_pid(&self) -> Option<i32>;
    fn live_frontmost_pid(&self) -> Option<i32>;
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "Only native macOS capture needs its process id")
    )]
    fn own_pid(&self) -> i32;
    fn is_running(&self, pid: i32) -> bool;
    fn activate(&self, pid: i32);
    fn pause(&self);
    fn send_cmd_v(&self) -> std::result::Result<(), String>;
}

#[cfg(target_os = "macos")]
pub(crate) static TARGET: Mutex<Option<i32>> = Mutex::new(None);

pub(crate) fn capture(
    popup_visible: bool,
    frontmost_pid: Option<i32>,
    own_pid: i32,
    stored: &mut Option<i32>,
) {
    if !popup_visible {
        *stored = frontmost_pid.filter(|&pid| pid != own_pid);
    }
}

struct MacDesktop<'a, S: MacSystem> {
    sys: &'a S,
    pid: i32,
}

impl<S: MacSystem> PasteDesktop for MacDesktop<'_, S> {
    fn target_valid(&self) -> bool {
        self.sys.is_running(self.pid)
    }

    fn target_foreground(&self) -> bool {
        self.sys.live_frontmost_pid() == Some(self.pid)
    }

    fn keys_released(&self) -> bool {
        self.sys.keys_released()
    }

    fn focus_target(&mut self) {
        self.sys.activate(self.pid);
    }

    fn pause(&self) {
        self.sys.pause();
    }

    fn send(&mut self, _chord: PasteChord) -> std::result::Result<(), String> {
        if !self.target_valid() || !self.target_foreground() || !self.keys_released() {
            return Err(
                "The destination or keyboard state changed before paste. Try again.".into(),
            );
        }
        self.sys.send_cmd_v()
    }
}

fn failed(reason: String) -> ClipboardError {
    ClipboardError::PasteFailed { reason }
}

pub(crate) fn prepare<S: MacSystem>(sys: &S, stored: Option<i32>) -> Result<Option<i32>> {
    if !sys.accessibility_granted() {
        return Err(ClipboardError::AccessibilityRequired);
    }
    let desktop = MacDesktop { sys, pid: 0 };
    paste_flow::wait_for_release(&desktop).map_err(failed)?;
    Ok(stored)
}

#[cfg(target_os = "macos")]
pub(crate) fn prepare_target<S: MacSystem>(sys: &S) -> Result<Option<i32>> {
    prepare(sys, None)?;
    TARGET
        .lock()
        .map(|target| *target)
        .map_err(|_| failed("Paste target state unavailable.".into()))
}

pub(crate) fn synthesize<S: MacSystem>(sys: &S, target: Option<i32>) -> Result<()> {
    match target {
        None => sys.send_cmd_v().map_err(failed),
        Some(pid) => paste_flow::paste_to_target(&mut MacDesktop { sys, pid }, PasteChord::CtrlV)
            .map_err(failed),
    }
}

#[cfg(target_os = "macos")]
pub(crate) struct RealMac;

#[cfg(target_os = "macos")]
impl MacSystem for RealMac {
    fn accessibility_granted(&self) -> bool {
        crate::permissions::accessibility_granted()
    }

    fn keys_released(&self) -> bool {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
        }

        const COMBINED_SESSION_STATE: i32 = 0;
        [55, 54, 56, 60, 9].into_iter().all(|key| {
            // Safety: the state id and keycodes are valid CoreGraphics values.
            !unsafe { CGEventSourceKeyState(COMBINED_SESSION_STATE, key) }
        })
    }

    fn frontmost_pid(&self) -> Option<i32> {
        objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|application| application.processIdentifier())
    }

    fn live_frontmost_pid(&self) -> Option<i32> {
        use core_foundation::array::CFArray;
        use core_foundation::base::{CFType, TCFType};
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::number::CFNumber;
        use core_foundation::string::CFString;
        use core_graphics::window::{
            copy_window_info, kCGNullWindowID, kCGWindowLayer, kCGWindowListExcludeDesktopElements,
            kCGWindowListOptionOnScreenOnly, kCGWindowOwnerPID,
        };

        let windows = copy_window_info(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        )?;
        // Safety: CoreGraphics returns an array of Core Foundation window dictionaries.
        let windows =
            unsafe { CFArray::<CFType>::wrap_under_get_rule(windows.as_concrete_TypeRef()) };
        // Safety: these framework constants are valid Core Foundation strings.
        let (layer_key, pid_key) = unsafe {
            (
                CFString::wrap_under_get_rule(kCGWindowLayer).into_CFType(),
                CFString::wrap_under_get_rule(kCGWindowOwnerPID).into_CFType(),
            )
        };
        for window in windows.iter() {
            let Some(window) = window.downcast::<CFDictionary>() else {
                continue;
            };
            // Safety: CoreGraphics window dictionaries contain Core Foundation keys and values.
            let window = unsafe {
                CFDictionary::<CFType, CFType>::wrap_under_get_rule(window.as_concrete_TypeRef())
            };
            let number = |key| window.find(key)?.downcast::<CFNumber>()?.to_i32();
            if number(&layer_key) == Some(0) {
                return number(&pid_key);
            }
        }
        None
    }

    fn own_pid(&self) -> i32 {
        std::process::id() as i32
    }

    fn is_running(&self, pid: i32) -> bool {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }

        const EPERM: i32 = 1;
        if pid <= 0 {
            return false;
        }
        // Safety: signal 0 only probes process existence and sends no signal.
        (unsafe { kill(pid, 0) == 0 })
            || std::io::Error::last_os_error().raw_os_error() == Some(EPERM)
    }

    fn activate(&self, pid: i32) {
        use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

        if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        }
    }

    fn pause(&self) {
        #[cfg(not(test))]
        {
            // A nested run loop deadlocks tao's main-thread observer; paste reads state live.
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn send_cmd_v(&self) -> std::result::Result<(), String> {
        use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| "failed to create CGEventSource".to_string())?;
        let key_v: CGKeyCode = 9;
        let key_down = CGEvent::new_keyboard_event(source.clone(), key_v, true)
            .map_err(|_| "failed to create key-down event".to_string())?;
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);
        let key_up = CGEvent::new_keyboard_event(source, key_v, false)
            .map_err(|_| "failed to create key-up event".to_string())?;
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);

        key_down.post(CGEventTapLocation::HID);
        key_up.post(CGEventTapLocation::HID);
        Ok(())
    }
}

#[cfg(test)]
#[path = "paste_macos_tests.rs"]
mod tests;
