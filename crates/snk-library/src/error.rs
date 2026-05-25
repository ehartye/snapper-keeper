use std::path::Path;

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LibraryError {
    #[error("database error: {message}")]
    Database { message: String, retryable: bool },

    #[error("io error at {path}: {reason}")]
    Io { path: String, reason: String },

    #[error("migration failed from {from} to {to}")]
    Migration {
        from: u32,
        to: u32,
        recoverable: bool,
    },

    #[error("not found: {what}")]
    NotFound { what: String },
}

impl LibraryError {
    /// Constructor for `LibraryError::Io` that preserves both the path
    /// and the full OS-level error message. Prefer this over relying on
    /// the blanket `From<io::Error>` impl at sites where the path is
    /// known: the blanket conversion can't capture path context and
    /// produces "io error at : <reason>" which is much harder to debug.
    pub fn io(path: impl AsRef<Path>, e: std::io::Error) -> Self {
        LibraryError::Io {
            path: path.as_ref().display().to_string(),
            reason: e.to_string(),
        }
    }
}

impl From<rusqlite::Error> for LibraryError {
    fn from(e: rusqlite::Error) -> Self {
        let retryable = matches!(
            e,
            rusqlite::Error::SqliteFailure(ref err, _)
                if err.code == rusqlite::ErrorCode::DatabaseBusy
                || err.code == rusqlite::ErrorCode::DatabaseLocked
        );
        LibraryError::Database {
            message: e.to_string(),
            retryable,
        }
    }
}

/// Blanket conversion for ergonomic `?` chains. Loses path context (no
/// way to recover it from a bare `io::Error`), but preserves the full
/// OS-level error message via `Display` rather than just the `ErrorKind`
/// name. Sites that have path context should use `LibraryError::io` for
/// strictly better diagnostics.
impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        LibraryError::Io {
            path: String::new(),
            reason: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, LibraryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_include_relevant_fields() {
        let db = LibraryError::Database {
            message: "boom".into(),
            retryable: false,
        };
        assert!(db.to_string().contains("boom"));

        let io = LibraryError::Io {
            path: "/x".into(),
            reason: "not found".into(),
        };
        let s = io.to_string();
        assert!(s.contains("/x"));
        assert!(s.contains("not found"));

        let mig = LibraryError::Migration {
            from: 1,
            to: 2,
            recoverable: true,
        };
        assert!(mig.to_string().contains("1"));
        assert!(mig.to_string().contains("2"));

        let nf = LibraryError::NotFound {
            what: "capture xyz".into(),
        };
        assert!(nf.to_string().contains("capture xyz"));
    }

    #[test]
    fn rusqlite_error_marks_busy_and_locked_as_retryable() {
        let busy = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy,
                extended_code: 0,
            },
            None,
        );
        let conv: LibraryError = busy.into();
        match conv {
            LibraryError::Database { retryable, .. } => assert!(retryable),
            other => panic!("expected Database, got {other:?}"),
        }

        let locked = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseLocked,
                extended_code: 0,
            },
            None,
        );
        match LibraryError::from(locked) {
            LibraryError::Database { retryable, .. } => assert!(retryable),
            other => panic!("expected Database, got {other:?}"),
        }
    }

    #[test]
    fn rusqlite_error_marks_other_failures_as_non_retryable() {
        // QueryReturnedNoRows isn't a SqliteFailure, so retryable should be false.
        let e: LibraryError = rusqlite::Error::QueryReturnedNoRows.into();
        match e {
            LibraryError::Database { retryable, .. } => assert!(!retryable),
            other => panic!("expected Database, got {other:?}"),
        }
    }

    #[test]
    fn std_io_error_maps_to_io_variant_preserving_os_detail() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let conv: LibraryError = io.into();
        match conv {
            LibraryError::Io { path, reason } => {
                // Display preserves both the kind ("permission denied") AND
                // the inner error string ("denied"), not just the kind name.
                // Previously the impl used e.kind().to_string() which dropped
                // the "denied" detail.
                assert!(reason.contains("denied"));
                assert!(path.is_empty(), "blanket From cannot recover path");
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn library_error_io_constructor_preserves_path_and_message() {
        let path = std::path::Path::new("/data/captures/x.png");
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied by acl");
        let conv = LibraryError::io(path, io);
        match conv {
            LibraryError::Io { path, reason } => {
                assert_eq!(path, "/data/captures/x.png");
                // Both the OS-level kind AND the inner detail land in reason.
                assert!(reason.contains("denied by acl"));
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn serde_uses_kind_discriminator_in_kebab_case() {
        let json = serde_json::to_string(&LibraryError::NotFound { what: "x".into() }).unwrap();
        assert!(json.contains("\"kind\":\"not-found\""));
    }
}
