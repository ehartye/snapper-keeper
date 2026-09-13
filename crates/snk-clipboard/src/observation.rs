//! Coherent macOS pasteboard observations. `changeCount` is an ownership generation,
//! not writer identity; delayed content can become readable without another change.
use crate::{
    sensitivity::SensitivityProbe,
    source_app::SourceApp,
    watcher::{ClipboardEvent, SkipReason, StepResult, WatcherState},
};
use std::time::Duration;

pub(super) trait ObservationSource {
    fn generation(&mut self) -> isize;
    fn sensitive(&mut self) -> bool;
    fn event(&mut self) -> Option<ClipboardEvent>;
    fn source_app(&mut self) -> Option<SourceApp>;
}

pub(super) struct GenerationGate {
    current: Option<isize>,
    completed: bool,
    reset_hash: bool,
    retry_at: Duration,
    delay: Duration,
}
impl Default for GenerationGate {
    fn default() -> Self {
        Self {
            current: None,
            completed: false,
            reset_hash: true,
            retry_at: Duration::ZERO,
            delay: Duration::from_millis(100),
        }
    }
}
impl GenerationGate {
    fn admit(&mut self, generation: isize, now: Duration) -> bool {
        // Equality handles zero, wraparound and pasteboard resets without ordering assumptions.
        if self.current != Some(generation) {
            *self = Self {
                current: Some(generation),
                ..Self::default()
            };
        }
        !self.completed && now >= self.retry_at
    }
    fn retry(&mut self, now: Duration) {
        self.retry_at = now.saturating_add(self.delay);
        self.delay = (self.delay * 2).min(Duration::from_secs(1));
    }
}

struct FrozenSensitivity(bool);
impl SensitivityProbe for FrozenSensitivity {
    fn is_sensitive(&self) -> bool {
        self.0
    }
}

pub(super) fn poll_once(
    source: &mut impl ObservationSource,
    gate: &mut GenerationGate,
    state: &mut WatcherState,
    now: Duration,
    process: impl FnOnce(
        ClipboardEvent,
        &mut WatcherState,
        &dyn SensitivityProbe,
        Option<SourceApp>,
    ) -> StepResult,
) -> Option<StepResult> {
    let before = source.generation();
    if !gate.admit(before, now) {
        return None;
    }
    let sensitive = FrozenSensitivity(source.sensitive());
    let event = source.event();
    let app = source.source_app();
    // Never consume a skip token, reset the worker hash or persist a mixed generation.
    if source.generation() != before {
        return None;
    }
    let Some(event) = event else {
        gate.retry(now);
        return None;
    };
    if gate.reset_hash {
        state.last_hash = None;
        gate.reset_hash = false;
    }
    let result = process(event, state, &sensitive, app);
    match result {
        StepResult::Skipped(SkipReason::EmptyContent | SkipReason::PersistFailed) => {
            gate.retry(now)
        }
        _ => gate.completed = true,
    }
    Some(result)
}

pub(super) trait PayloadReader {
    fn text(&mut self) -> Option<String>;
    fn image(&mut self) -> Option<ClipboardEvent>;
}

pub(super) fn read_payload(reader: &mut impl PayloadReader) -> Option<ClipboardEvent> {
    if let Some(text) = reader.text().filter(|text| !text.is_empty()) {
        return Some(ClipboardEvent::Text(text));
    }
    reader
        .image()
        .filter(|event| matches!(event, ClipboardEvent::Image { bytes, .. } if !bytes.is_empty()))
}

#[cfg(test)]
#[path = "observation_tests.rs"]
mod tests;
