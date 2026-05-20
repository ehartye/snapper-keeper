//! snk-library — single owner of the SQLite persistence layer.

pub mod db;
pub mod error;

pub use db::Db;
pub use error::{LibraryError, Result};
