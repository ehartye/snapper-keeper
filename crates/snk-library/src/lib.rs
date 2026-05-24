//! snk-library — single owner of the SQLite persistence layer.
//!
//! ## ID scheme
//!
//! All primary keys in this crate are **UUIDv7** strings produced via
//! [`uuid::Uuid::now_v7()`]. Two invariants of UUIDv7 are relied on at
//! multiple call sites (`captures::*`, `clipboard::*`, search ranking,
//! cross-plugin event payloads) and must NOT be broken by switching to
//! a different ID scheme:
//!
//! 1. **Time-ordered monotonicity.** UUIDv7's leading 48 bits are a
//!    Unix timestamp in milliseconds, so lexicographic ordering of two
//!    IDs produced on the same machine reflects creation order. SQL
//!    queries depend on this for `ORDER BY id` to mean "newest last"
//!    without a separate `created_at` index lookup, and for FTS
//!    rank-tie-breaking.
//!
//! 2. **No HTML/JSON special characters.** UUIDv7 is canonical hex
//!    with four hyphens (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
//!    Render paths assume IDs are safe to interpolate into HTML attrs
//!    and JSON strings without escaping. Switching to a scheme that
//!    can produce `<`, `>`, `"`, `'`, `&`, `\`, or NUL would require
//!    auditing every render and serializer.
//!
//! Changing the ID scheme is a project-wide audit — open a design doc
//! before doing it.

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
pub mod tags;

pub use captures::{Capture, ListCapturesQuery, NewCapture};
pub use clipboard::{ClipboardItem, ClipboardItemKind, ListClipboardQuery, NewClipboardItem};
pub use db::Db;
pub use error::{LibraryError, Result};
pub use ocr::OcrText;
pub use plugin::{init, LibraryState};
pub use search::SearchResult;
pub use tags::Tag;
