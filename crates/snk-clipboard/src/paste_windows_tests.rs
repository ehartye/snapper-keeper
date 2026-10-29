use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW, RegisterClassW,
    TranslateMessage, UnregisterClassW, CS_HREDRAW, CS_VREDRAW, MSG, PM_REMOVE, SW_SHOW,
    WINDOW_EX_STYLE, WM_KEYDOWN, WM_SYSKEYDOWN, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

static CHORD: AtomicUsize = AtomicUsize::new(0);
unsafe extern "system" fn recorder(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
        let ctrl = GetKeyState(0x11) < 0;
        let shift = GetKeyState(0x10) < 0;
        if w.0 == 0x56 && ctrl && !shift {
            CHORD.store(1, Ordering::SeqCst);
        }
        if w.0 == 0x2d && shift && !ctrl {
            CHORD.store(2, Ordering::SeqCst);
        }
        // Consume every input message; never invoke a clipboard paste handler.
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, w, l)
}

#[test]
#[ignore = "requires an interactive desktop; focuses only disposable input-recorder windows"]
fn native_input_reaches_only_owned_normal_and_terminal_class_windows() {
    unsafe {
        let previous = GetForegroundWindow();
        for (class, expected) in [
            ("SNK_Paste_Input_Recorder", 1),
            ("CASCADIA_HOSTING_WINDOW_CLASS", 2),
            ("ConsoleWindowClass", 2),
            ("mintty", 2),
        ] {
            let name: Vec<u16> = class.encode_utf16().chain(Some(0)).collect();
            let class_name = PCWSTR(name.as_ptr());
            let instance = HINSTANCE(std::ptr::null_mut());
            let definition = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(recorder),
                hInstance: instance,
                lpszClassName: class_name,
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&definition), 0, "register {class}");
            let target = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("Snapper Keeper disposable paste test"),
                WS_OVERLAPPEDWINDOW,
                100,
                100,
                400,
                180,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap();
            let _ = ShowWindow(target, SW_SHOW);
            let _ = SetForegroundWindow(target);
            CHORD.store(0, Ordering::SeqCst);
            let destination = Target {
                hwnd: target.0 as usize,
                pid: std::process::id(),
                chord: paste_flow::chord_for_class(class),
            };
            let result = synthesize(destination);
            if result.is_err() {
                eprintln!(
                    "foreground_matches={} held_virtual_keys={:?}",
                    Desktop(destination).target_foreground(),
                    (0x08..=0xfe)
                        .filter(|key| GetAsyncKeyState(*key) < 0)
                        .collect::<Vec<_>>()
                );
            }
            let mut message = MSG::default();
            for _ in 0..100 {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                if CHORD.load(Ordering::SeqCst) == expected {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let received = CHORD.load(Ordering::SeqCst);
            let _ = DestroyWindow(target);
            let _ = UnregisterClassW(class_name, Some(instance));
            if result.is_err() {
                let _ = SetForegroundWindow(previous);
            }
            result.unwrap();
            assert_eq!(received, expected, "real SendInput chord for {class}");
        }
        let _ = SetForegroundWindow(previous);
    }
}
