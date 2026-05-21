use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ClipboardError {
    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),

    #[error("clipboard access failed: {message}")]
    Access { message: String },

    #[error("paste failed: {reason}")]
    PasteFailed { reason: String },

    #[error("not found: {what}")]
    NotFound { what: String },
}

impl From<snk_library::LibraryError> for ClipboardError {
    fn from(e: snk_library::LibraryError) -> Self {
        ClipboardError::Library(e)
    }
}

impl From<arboard::Error> for ClipboardError {
    fn from(e: arboard::Error) -> Self {
        ClipboardError::Access {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ClipboardError>;
