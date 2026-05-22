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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_include_relevant_fields() {
        assert!(AnnotateError::Image {
            message: "bad".into()
        }
        .to_string()
        .contains("bad"));
        assert!(AnnotateError::InvalidInput {
            reason: "empty".into()
        }
        .to_string()
        .contains("empty"));
        let lib_err = snk_library::LibraryError::NotFound { what: "x".into() };
        assert!(AnnotateError::Library(lib_err)
            .to_string()
            .contains("library"));
    }

    #[test]
    fn from_library_error_wraps() {
        let e = snk_library::LibraryError::NotFound { what: "x".into() };
        let conv: AnnotateError = e.into();
        match conv {
            AnnotateError::Library(_) => {}
            other => panic!("expected Library, got {other:?}"),
        }
    }

    #[test]
    fn from_image_error_maps_to_image() {
        let e = image::ImageError::Parameter(image::error::ParameterError::from_kind(
            image::error::ParameterErrorKind::Generic("oops".into()),
        ));
        let conv: AnnotateError = e.into();
        match conv {
            AnnotateError::Image { .. } => {}
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn serde_uses_kind_discriminator_in_kebab_case() {
        let json =
            serde_json::to_string(&AnnotateError::InvalidInput { reason: "x".into() }).unwrap();
        assert!(json.contains("\"kind\":\"invalid-input\""));
    }
}
