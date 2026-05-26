//! Per-OS implementations re-exported through trait-shaped helpers.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::*;
#[cfg(target_os = "windows")]
pub(crate) use windows::*;

// Stub for OSes (Linux dev builds) without a real impl.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn current_source_app() -> Option<crate::source_app::SourceApp> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn is_sensitive() -> bool {
    false
}
