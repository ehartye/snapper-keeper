//! snk-capture — screen capture entry points.
//!
//! Phase 1 supports primary-monitor full-screen capture only. Region overlay,
//! window capture, timed capture, and the floating post-capture toolbar come
//! in later phases.

pub mod error;
pub mod grab;

pub use error::{CaptureError, Result};
pub use grab::{grab_primary_monitor, GrabResult};
