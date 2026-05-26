//! snk-ocr — native OCR backend selection (Vision / Windows.Media.Ocr) + async queue.

pub mod backend;
pub mod error;
pub mod plugin;
pub mod queue;

// Sidecar module kept (private) until T9 rewrites queue.rs and T10 rewrites plugin.rs.
// Both still reference crate::sidecar::* internally. Full deletion happens in T13.
mod sidecar;

pub use backend::{OcrBackend, OcrResult};
pub use error::OcrError;
pub use plugin::init;
