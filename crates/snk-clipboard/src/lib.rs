//! snk-clipboard — clipboard watcher, dedup, paste synthesis, caret resolution.

pub mod caret;
pub mod commands;
pub mod error;
pub mod hasher;
pub mod paste;
pub mod plugin;
pub mod watcher;

pub use error::{ClipboardError, Result};
pub use plugin::init;
