use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
#[serde(tag = "kind")]
pub enum OcrError {
    #[error("OCR backend unavailable: {reason}")]
    BackendUnavailable { reason: String },

    #[error("no recognizer language available: {detail}")]
    NoRecognizerLanguage { detail: String },

    #[error("recognize failed: {detail}")]
    Recognize { detail: String },

    #[error("image load failed for {path}: {detail}")]
    ImageLoad { path: String, detail: String },
}
