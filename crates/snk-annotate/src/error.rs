use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AnnotateError {
    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),

    #[error("image error: {message}")]
    Image { message: String },

    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },
}

impl From<snk_library::LibraryError> for AnnotateError {
    fn from(e: snk_library::LibraryError) -> Self {
        AnnotateError::Library(e)
    }
}

impl From<image::ImageError> for AnnotateError {
    fn from(e: image::ImageError) -> Self {
        AnnotateError::Image {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AnnotateError>;
