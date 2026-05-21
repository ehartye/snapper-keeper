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

impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        LibraryError::Io {
            path: String::new(),
            reason: e.kind().to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, LibraryError>;
