//! snk-library — single owner of the SQLite persistence layer.

pub mod captures;
pub mod clipboard;
pub mod commands;
pub mod db;
pub mod error;
pub mod files;
pub mod migrate;
pub mod ocr;
pub mod plugin;
pub mod search;
pub mod settings;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use clipboard::{ClipboardItem, ClipboardItemKind, ListClipboardQuery, NewClipboardItem};
pub use db::Db;
pub use error::{LibraryError, Result};
pub use ocr::OcrText;
pub use plugin::{init, LibraryState};
pub use search::SearchResult;
