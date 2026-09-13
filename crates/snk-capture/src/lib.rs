//! snk-capture — screen capture entry points.
//!
//! Full-screen and window captures run on a serialized blocking worker.
//! Region captures crop its token-bound preview snapshot without re-grabbing
//! the desktop or consulting display geometry again.

pub mod commands;
mod display;
pub mod error;
pub mod foreground;
pub mod grab;
pub mod orchestrate;
pub mod permissions;
pub mod plugin;
mod preview;
pub mod window_hider;
mod worker;

pub use error::{CaptureError, Result};
pub use foreground::{get_foreground_info, ForegroundInfo};
pub use grab::{
    grab_primary_monitor, grab_window, list_capturable_windows, GrabResult, WindowInfo,
};
pub use plugin::init;
