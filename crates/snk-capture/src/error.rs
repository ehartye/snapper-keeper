use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CaptureError {
    #[error("no monitors found")]
    NoMonitors,

    #[error("xcap error: {message}")]
    Os { message: String },

    #[error("encode error: {message}")]
    Encode { message: String },

    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),
}

impl From<xcap::XCapError> for CaptureError {
    fn from(e: xcap::XCapError) -> Self {
        CaptureError::Os { message: e.to_string() }
    }
}

impl From<image::ImageError> for CaptureError {
    fn from(e: image::ImageError) -> Self {
        CaptureError::Encode { message: e.to_string() }
    }
}

impl From<snk_library::LibraryError> for CaptureError {
    fn from(e: snk_library::LibraryError) -> Self {
        CaptureError::Library(e)
    }
}

pub type Result<T> = std::result::Result<T, CaptureError>;
