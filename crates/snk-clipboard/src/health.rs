//! Clipboard watcher health: retry-with-backoff on open failure, a shared
//! status the `clipboard_status` command reports, and a `HealthSink` seam so
//! the watcher can announce availability transitions without depending on the
//! Tauri runtime generics (the Windows `CTX` static stores the sink as a
//! concrete trait object).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use arboard::Clipboard;
use serde::Serialize;
use tracing::error;
use ts_rs::TS;

/// Current health of the clipboard watcher, surfaced to the frontend so a
/// popup can render an offline banner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(
    export,
    export_to = "../../../packages/snk-clipboard/src/generated/clipboard-status.ts"
)]
pub struct ClipboardStatus {
    pub available: bool,
    pub last_error: Option<String>,
}

impl Default for ClipboardStatus {
    fn default() -> Self {
        // Optimistic until the watcher reports otherwise. The watcher opens
        // the OS clipboard within the first moments of startup and calls
        // mark_available()/mark_unavailable() immediately, so this default is
        // only ever observed in the brief window before the first attempt.
        Self {
            available: true,
            last_error: None,
        }
    }
}

/// Shared, Tauri-managed health state. The `clipboard_status` command reads a
/// snapshot; the watcher's sink writes transitions.
#[derive(Clone, Default)]
pub struct ClipboardHealth {
    inner: Arc<Mutex<ClipboardStatus>>,
}

impl ClipboardHealth {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> ClipboardStatus {
        self.inner.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub(crate) fn mark_available(&self) -> ClipboardStatus {
        let status = ClipboardStatus {
            available: true,
            last_error: None,
        };
        if let Ok(mut s) = self.inner.lock() {
            *s = status.clone();
        }
        status
    }

    pub(crate) fn mark_unavailable(&self, error: String) -> ClipboardStatus {
        let status = ClipboardStatus {
            available: false,
            last_error: Some(error),
        };
        if let Ok(mut s) = self.inner.lock() {
            *s = status.clone();
        }
        status
    }
}

/// Reports clipboard availability transitions. The production impl
/// (`plugin::AppHealthSink`) updates the shared `ClipboardHealth` and emits a
/// Tauri event; tests use a fake.
pub trait HealthSink: Send + Sync {
    fn report_available(&self);
    fn report_unavailable(&self, error: String);
}

/// Initial retry delay; doubles per attempt up to [`BACKOFF_CAP`].
pub(crate) const INITIAL_BACKOFF: Duration = Duration::from_millis(500);
const BACKOFF_CAP: Duration = Duration::from_secs(60);

/// Exponential backoff capped at 60s. Sequence from 500ms:
/// 500ms, 1s, 2s, 4s, … 32s, 60s, 60s, …
pub(crate) fn next_backoff(prev: Duration) -> Duration {
    prev.saturating_mul(2).min(BACKOFF_CAP)
}

/// Open the OS clipboard, retrying forever with exponential backoff on
/// failure. Reports each failure (so the UI can show an offline banner) and
/// reports availability once the open finally succeeds. Blocks the calling
/// watcher thread rather than letting it die — the previous behavior was a
/// silent thread exit with no retry and no user feedback.
pub(crate) fn open_clipboard_with_backoff(sink: &dyn HealthSink) -> Clipboard {
    let mut delay = INITIAL_BACKOFF;
    loop {
        match Clipboard::new() {
            Ok(c) => {
                sink.report_available();
                return c;
            }
            Err(e) => {
                error!(error = %e, ?delay, "failed to open clipboard; retrying after backoff");
                sink.report_unavailable(e.to_string());
                std::thread::sleep(delay);
                delay = next_backoff(delay);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_is_optimistically_available() {
        let s = ClipboardStatus::default();
        assert!(s.available);
        assert!(s.last_error.is_none());
    }

    #[test]
    fn next_backoff_doubles_then_caps_at_60s() {
        let mut d = INITIAL_BACKOFF;
        assert_eq!(d, Duration::from_millis(500));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(1));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(2));
        // Walk up to the cap.
        for _ in 0..10 {
            d = next_backoff(d);
        }
        assert_eq!(d, BACKOFF_CAP, "must saturate at 60s");
        assert_eq!(next_backoff(d), BACKOFF_CAP, "stays at the cap");
    }

    #[test]
    fn mark_unavailable_then_available_round_trips_through_snapshot() {
        let health = ClipboardHealth::new();
        assert!(health.snapshot().available, "starts available");

        let s = health.mark_unavailable("no display".into());
        assert_eq!(s, health.snapshot());
        assert!(!health.snapshot().available);
        assert_eq!(health.snapshot().last_error.as_deref(), Some("no display"));

        health.mark_available();
        assert!(health.snapshot().available);
        assert!(
            health.snapshot().last_error.is_none(),
            "recovery clears the error"
        );
    }

    /// A sink wired to a real `ClipboardHealth` (no AppHandle) mirrors what the
    /// production sink does to the shared state, minus the Tauri emit.
    struct StateOnlySink(ClipboardHealth);
    impl HealthSink for StateOnlySink {
        fn report_available(&self) {
            self.0.mark_available();
        }
        fn report_unavailable(&self, error: String) {
            self.0.mark_unavailable(error);
        }
    }

    #[test]
    fn sink_drives_shared_health_state() {
        let health = ClipboardHealth::new();
        let sink = StateOnlySink(health.clone());
        sink.report_unavailable("boom".into());
        assert!(!health.snapshot().available);
        sink.report_available();
        assert!(health.snapshot().available);
    }
}
