//! Windows native implementations: Win32 clipboard API for sensitive
//! flag inspection, foreground-window lookup for source-app detection.

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    GetClipboardData, IsClipboardFormatAvailable, RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
// HWND is added back in Task 9 when source-app detection uses it.
// OpenClipboard/CloseClipboard are not needed — WM_CLIPBOARDUPDATE
// handler runs with the clipboard already open by the OS.

use crate::source_app::SourceApp;

// The two registered formats Win+V honors (and the older
// CFSTR_EXCLUDECLIPBOARDCONTENTFROMMONITORING).
const FMT_EXCLUDE_FROM_MONITORING: &str = "ExcludeClipboardContentFromMonitoring";
const FMT_CAN_INCLUDE_IN_HISTORY: &str = "CanIncludeInClipboardHistory";
const FMT_CAN_UPLOAD_TO_CLOUD: &str = "CanUploadToCloudClipboard";

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn register_format(name: &str) -> u32 {
    let wide = wide_null(name);
    // SAFETY: RegisterClipboardFormatW is reentrant + thread-safe; same
    // format name returns the same id on repeated calls.
    unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) }
}

/// Read a `u32` value out of a clipboard format that stores a single
/// DWORD. Returns `None` if the format isn't present or the data isn't
/// the expected size.
fn read_u32_format(fmt: u32) -> Option<u32> {
    unsafe {
        // windows-rs 0.61: IsClipboardFormatAvailable returns Result<()>,
        // not BOOL. Ok == "format present"; Err == "not present" (or error).
        if IsClipboardFormatAvailable(fmt).is_err() {
            return None;
        }
        // The watcher already holds the clipboard open when this is
        // called from the WM_CLIPBOARDUPDATE handler; no explicit
        // OpenClipboard / CloseClipboard needed.
        let handle: HANDLE = GetClipboardData(fmt).ok()?;
        // HANDLE and HGLOBAL are both #[repr(transparent)] newtypes around
        // *mut c_void, but the GlobalLock signature takes HGLOBAL — bridge
        // explicitly via the public ctor rather than a raw cast.
        let hglobal = HGLOBAL(handle.0);
        let ptr: *mut c_void = GlobalLock(hglobal);
        if ptr.is_null() {
            return None;
        }
        let value = *(ptr as *const u32);
        let _ = GlobalUnlock(hglobal);
        Some(value)
    }
}

pub(crate) fn is_sensitive() -> bool {
    let exclude = register_format(FMT_EXCLUDE_FROM_MONITORING);
    let can_include = register_format(FMT_CAN_INCLUDE_IN_HISTORY);
    let can_upload = register_format(FMT_CAN_UPLOAD_TO_CLOUD);

    // CFSTR_EXCLUDECLIPBOARDCONTENTFROMMONITORING is presence-only — its
    // existence flags the content as excluded. (windows-rs 0.61:
    // Result<()> — Ok == present, Err == not present.)
    unsafe {
        if IsClipboardFormatAvailable(exclude).is_ok() {
            return true;
        }
    }

    // CanIncludeInClipboardHistory / CanUploadToCloudClipboard are DWORDs
    // with value 0 = "do not include / do not upload".
    if matches!(read_u32_format(can_include), Some(0)) {
        return true;
    }
    if matches!(read_u32_format(can_upload), Some(0)) {
        return true;
    }
    false
}

pub(crate) fn current_source_app() -> Option<SourceApp> {
    // Placeholder — implemented in the next task.
    None
}
