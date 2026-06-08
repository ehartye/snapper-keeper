//! snk-capture — screen capture entry points.
//!
//! Phase 1 supports primary-monitor full-screen capture only. Region overlay,
//! window capture, timed capture, and the floating post-capture toolbar come
//! in later phases.

pub mod commands;
pub mod error;
pub mod foreground;
pub mod grab;
pub mod orchestrate;
pub mod permissions;
pub mod plugin;
pub mod window_hider;

pub use error::{CaptureError, Result};
pub use foreground::{get_foreground_info, ForegroundInfo};
pub use grab::{
    grab_primary_monitor, grab_region, grab_window, list_capturable_windows, GrabResult, WindowInfo,
};
pub use plugin::init;
