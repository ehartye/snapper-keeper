//! snk-library — single owner of the SQLite persistence layer.

pub mod captures;
pub mod db;
pub mod error;
pub mod files;
pub mod migrate;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use db::Db;
pub use error::{LibraryError, Result};
