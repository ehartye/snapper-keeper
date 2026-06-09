use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

#[derive(Error, Debug, Serialize, TS)]
#[ts(
    export,
    export_to = "../../../packages/snk-capture/src/generated/errors.ts"
)]
#[serde(tag = "kind", content = "data", rename_all = "kebab-case")]
pub enum CaptureError {
    #[error("no monitors found")]
    NoMonitors,

    #[error("window not found: {id}")]
    WindowNotFound { id: u32 },

    #[error("xcap error: {message}")]
    Os { message: String },

    #[error("encode error: {message}")]
    Encode { message: String },

    #[error("library error: {0:?}")]
    Library(snk_library::LibraryError),

    #[error("screen recording permission denied")]
    ScreenRecordingPermissionDenied,
}

impl From<xcap::XCapError> for CaptureError {
    fn from(e: xcap::XCapError) -> Self {
        CaptureError::Os {
            message: e.to_string(),
        }
    }
}

impl From<image::ImageError> for CaptureError {
    fn from(e: image::ImageError) -> Self {
        CaptureError::Encode {
            message: e.to_string(),
        }
    }
}

impl From<snk_library::LibraryError> for CaptureError {
    fn from(e: snk_library::LibraryError) -> Self {
        CaptureError::Library(e)
    }
}

pub type Result<T> = std::result::Result<T, CaptureError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_include_relevant_fields() {
        assert!(CaptureError::NoMonitors.to_string().contains("no monitors"));
        assert!(CaptureError::WindowNotFound { id: 42 }
            .to_string()
            .contains("42"));
        assert!(CaptureError::Os {
            message: "boom".into()
        }
        .to_string()
        .contains("boom"));
        assert!(CaptureError::Encode {
            message: "bad png".into()
        }
        .to_string()
        .contains("bad png"));
        let lib_err = snk_library::LibraryError::NotFound { what: "x".into() };
        let wrapped = CaptureError::Library(lib_err);
        assert!(wrapped.to_string().contains("library"));
        assert!(CaptureError::ScreenRecordingPermissionDenied
            .to_string()
            .contains("screen recording"));
    }

    #[test]
    fn from_image_error_maps_to_encode() {
        let e = image::ImageError::Parameter(image::error::ParameterError::from_kind(
            image::error::ParameterErrorKind::DimensionMismatch,
        ));
        let conv: CaptureError = e.into();
        match conv {
            CaptureError::Encode { .. } => {}
            other => panic!("expected Encode, got {other:?}"),
        }
    }

    #[test]
    fn from_library_error_wraps() {
        let e = snk_library::LibraryError::NotFound {
            what: "capture x".into(),
        };
        let conv: CaptureError = e.into();
        match conv {
            CaptureError::Library(_) => {}
            other => panic!("expected Library, got {other:?}"),
        }
    }

    #[test]
    fn serde_uses_kind_discriminator_in_kebab_case() {
        let json = serde_json::to_string(&CaptureError::WindowNotFound { id: 1 }).unwrap();
        assert!(json.contains("\"kind\":\"window-not-found\""));
    }
}
