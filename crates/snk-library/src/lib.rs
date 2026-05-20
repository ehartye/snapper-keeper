//! snk-library — single owner of the SQLite persistence layer.

pub mod error;

pub use error::{LibraryError, Result};

// Tauri plugin entry point — wired in Task 9.
