use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "kebab-case")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_include_relevant_fields() {
        assert!(ClipboardError::Access {
            message: "denied".into()
        }
        .to_string()
        .contains("denied"));
        assert!(ClipboardError::PasteFailed {
            reason: "too short".into()
        }
        .to_string()
        .contains("too short"));
        assert!(ClipboardError::NotFound {
            what: "item x".into()
        }
        .to_string()
        .contains("item x"));
        let lib_err = snk_library::LibraryError::NotFound { what: "y".into() };
        assert!(ClipboardError::Library(lib_err)
            .to_string()
            .contains("library"));
    }

    #[test]
    fn from_library_error_wraps() {
        let e = snk_library::LibraryError::NotFound { what: "z".into() };
        let conv: ClipboardError = e.into();
        match conv {
            ClipboardError::Library(_) => {}
            other => panic!("expected Library, got {other:?}"),
        }
    }

    #[test]
    fn from_arboard_error_maps_to_access() {
        let e = arboard::Error::ContentNotAvailable;
        let conv: ClipboardError = e.into();
        match conv {
            ClipboardError::Access { .. } => {}
            other => panic!("expected Access, got {other:?}"),
        }
    }

    #[test]
    fn serde_uses_kind_discriminator_in_kebab_case() {
        let json = serde_json::to_string(&ClipboardError::NotFound { what: "x".into() }).unwrap();
        assert!(json.contains("\"kind\":\"not-found\""));
    }
}
