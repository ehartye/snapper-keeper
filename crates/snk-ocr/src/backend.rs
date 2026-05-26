use std::path::Path;

pub use snk_library::ocr::{BBox, OcrWord};

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<OcrWord>,
    pub language: String,
    pub confidence: f64,
}

pub trait OcrBackend: Send + Sync {
    fn recognize(&self, image_path: &Path) -> Result<OcrResult, crate::OcrError>;
    fn name(&self) -> &'static str;
    fn engine_version(&self) -> String;
}
