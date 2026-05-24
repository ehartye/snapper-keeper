//! snk-clipboard — clipboard watcher, dedup, paste synthesis, caret resolution.

pub mod caret;
pub mod commands;
pub mod error;
pub mod hasher;
pub mod paste;
mod platform;
pub mod plugin;
pub mod source_app;
pub mod watcher;

pub use error::{ClipboardError, Result};
pub use plugin::init;
