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
pub mod plugin;

pub use error::{CaptureError, Result};
pub use foreground::{get_foreground_info, ForegroundInfo};
pub use grab::{grab_primary_monitor, GrabResult};
pub use plugin::init;
