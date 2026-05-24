//! Windows native implementations: Win32 clipboard API for sensitive
//! flag inspection, foreground-window lookup for source-app detection.

use std::ffi::c_void;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    GetClipboardData, IsClipboardFormatAvailable, RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
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
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        let tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if tid == 0 || pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf: [u16; 1024] = [0; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        if ok.is_err() || len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let exe = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())?;

        // display_name = FileDescription from version info if available,
        // else the exe basename.
        let display_name = file_description(&path).unwrap_or_else(|| exe.clone());

        Some(SourceApp {
            identifier: exe.to_ascii_lowercase(),
            display_name,
            kind: crate::source_app::SourceAppKind::WindowsExe,
        })
    }
}

fn file_description(exe_path: &str) -> Option<String> {
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };

    let wide_path = wide_null(exe_path);
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(wide_path.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buf: Vec<u8> = vec![0; size as usize];
        if GetFileVersionInfoW(
            PCWSTR(wide_path.as_ptr()),
            // windows-rs 0.61: dwhandle is Option<u32>, not raw u32.
            // The Win32 docs say "this parameter is ignored," so None
            // is the canonical pass-through.
            None,
            size,
            buf.as_mut_ptr() as *mut c_void,
        )
        .is_err()
        {
            return None;
        }
        // Use the language-neutral 040904B0 codepage to query
        // FileDescription. (Most binaries ship at least the English entry.)
        let sub_block = wide_null(r"\StringFileInfo\040904B0\FileDescription");
        let mut value_ptr: *mut c_void = std::ptr::null_mut();
        let mut value_len: u32 = 0;
        let ok = VerQueryValueW(
            buf.as_ptr() as *const c_void,
            PCWSTR(sub_block.as_ptr()),
            &mut value_ptr,
            &mut value_len,
        );
        if !ok.as_bool() || value_ptr.is_null() || value_len == 0 {
            return None;
        }
        let slice = std::slice::from_raw_parts(value_ptr as *const u16, value_len as usize);
        let s = String::from_utf16_lossy(slice);
        Some(s.trim_end_matches('\0').to_string())
    }
}
