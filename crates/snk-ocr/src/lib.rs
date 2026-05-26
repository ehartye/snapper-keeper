//! snk-ocr — native OCR backend selection (Vision / Windows.Media.Ocr) + async queue.

pub mod backend;
pub mod error;
pub mod plugin;
pub mod queue;

// Sidecar module kept (private) until T9 rewrites queue.rs and T10 rewrites plugin.rs.
// Both still reference crate::sidecar::* internally. Full deletion happens in T13.
mod sidecar;

#[cfg(target_os = "macos")]
pub mod vision;

#[cfg(target_os = "windows")]
pub mod winocr;

pub use backend::{OcrBackend, OcrResult};
pub use error::OcrError;
pub use plugin::init;
