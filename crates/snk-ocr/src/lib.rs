//! snk-ocr — async OCR queue + tesseract sidecar.

pub mod plugin;
pub mod queue;
pub mod sidecar;

pub use plugin::init;
