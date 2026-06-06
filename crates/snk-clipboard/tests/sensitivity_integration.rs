//! Integration test — manipulates the real OS clipboard. Runs serial
//! against any other env-mutating tests via `serial_test::serial(clipboard)`.
//!
//! These are `#[ignore]`-by-default: they read/write the shared system
//! clipboard, so they flake against any concurrent clipboard activity (managers,
//! other apps, parallel test processes) and never run in CI (rust-test is
//! Linux-only, where these are `cfg`-compiled out). Run them deliberately on an
//! interactive desktop with `cargo test -p snk-clipboard --test sensitivity_integration -- --ignored`.

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manipulates the real OS clipboard; racy against concurrent clipboard activity. Run explicitly with `cargo test -- --ignored` on an interactive desktop."]
#[serial_test::serial(clipboard)]
fn macos_concealed_type_marks_sensitive() {
    use objc2::rc::Retained;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::{NSArray, NSString};

    unsafe {
        let pasteboard: Retained<NSPasteboard> = NSPasteboard::generalPasteboard();
        let _ = pasteboard.clearContents();

        let value = NSString::from_str("secret");
        let concealed_type = NSString::from_str("org.nspasteboard.ConcealedType");
        let types = NSArray::from_slice(&[concealed_type.as_ref()]);
        let _ = pasteboard.declareTypes_owner(&types, None);
        let _ = pasteboard.setString_forType(&value, &concealed_type);
    }

    assert!(
        snk_clipboard::sensitivity::is_sensitive(),
        "concealed type should be reported as sensitive"
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manipulates the real OS clipboard; racy against concurrent clipboard activity. Run explicitly with `cargo test -- --ignored` on an interactive desktop."]
#[serial_test::serial(clipboard)]
fn macos_plain_text_is_not_sensitive() {
    use objc2::rc::Retained;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::NSString;

    unsafe {
        let pasteboard: Retained<NSPasteboard> = NSPasteboard::generalPasteboard();
        let _ = pasteboard.clearContents();
        let value = NSString::from_str("hello world");
        let _ = pasteboard.setString_forType(&value, &NSString::from_str("public.utf8-plain-text"));
    }

    assert!(!snk_clipboard::sensitivity::is_sensitive());
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "manipulates the real OS clipboard; racy against concurrent clipboard activity. Run explicitly with `cargo test -- --ignored` on an interactive desktop."]
#[serial_test::serial(clipboard)]
fn windows_can_include_in_history_zero_marks_sensitive() {
    use windows::core::w;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    unsafe {
        let _ = OpenClipboard(None);
        let _ = EmptyClipboard();

        // Write a dummy text payload first.
        let text = "hello\0";
        let bytes = text.as_bytes();
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).expect("GlobalAlloc text");
        let ptr = GlobalLock(handle);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        let _ = GlobalUnlock(handle);
        let _ = SetClipboardData(1u32 /* CF_TEXT */, Some(HANDLE(handle.0)));

        // Set the CanIncludeInClipboardHistory format to 0 (a DWORD).
        let fmt = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
        let h = GlobalAlloc(GMEM_MOVEABLE, 4).expect("GlobalAlloc dword");
        let p = GlobalLock(h);
        *(p as *mut u32) = 0;
        let _ = GlobalUnlock(h);
        let _ = SetClipboardData(fmt, Some(HANDLE(h.0)));
    }

    // Close the clipboard BEFORE asserting. This mirrors the real
    // production state: by the time `is_sensitive` runs in the
    // WM_CLIPBOARDUPDATE handler, arboard's earlier `get_text()` has
    // already opened AND closed the clipboard. The previous version of
    // this test kept the clipboard open through the assertion, which
    // masked the production bug where `is_sensitive` silently failed
    // because `GetClipboardData` returns NULL without an OpenClipboard
    // call. `is_sensitive` now owns its own OpenClipboard/CloseClipboard
    // lifecycle and must work from this caller-doesn't-own state.
    unsafe {
        let _ = CloseClipboard();
    }

    assert!(snk_clipboard::sensitivity::is_sensitive());
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[test]
fn linux_test_skipped() {
    eprintln!("SKIP: sensitivity integration runs on macOS + Windows only");
}
