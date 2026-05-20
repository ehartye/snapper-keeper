//! snk-library — single owner of the SQLite persistence layer.

pub mod captures;
pub mod commands;
pub mod db;
pub mod error;
pub mod files;
pub mod migrate;
pub mod plugin;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use db::Db;
pub use error::{LibraryError, Result};
pub use plugin::{init, LibraryState};
