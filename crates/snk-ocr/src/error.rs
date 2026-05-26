use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug, Error, Serialize, Clone, TS)]
#[ts(
    export,
    export_to = "../../../packages/snk-ocr/src/generated/errors.ts"
)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OcrError {
    #[error("OCR backend unavailable: {reason}")]
    BackendUnavailable { reason: String },

    #[error("no recognizer language available: {detail}")]
    NoRecognizerLanguage { detail: String },

    #[error("recognize failed: {detail}")]
    Recognize { detail: String },

    #[error("library access failed: {detail}")]
    Library { detail: String },

    #[error("OCR state unavailable: {reason}")]
    StateUnavailable { reason: String },

    #[error("image load failed for {path}: {detail}")]
    ImageLoad { path: String, detail: String },
}

pub type Result<T> = std::result::Result<T, OcrError>;
