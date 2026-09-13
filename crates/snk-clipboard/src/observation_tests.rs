use super::*;
use crate::watcher::{worker_step, SkipReason};
use snk_library::Db;

struct FakeSource {
    generation: isize,
    after: Option<isize>,
    pending: bool,
    sensitive: bool,
    change_sensitivity_after_read: bool,
    reads: usize,
    probes: usize,
    apps: usize,
    image: bool,
    text: String,
}
impl Default for FakeSource {
    fn default() -> Self {
        Self {
            generation: 0,
            after: None,
            pending: false,
            sensitive: false,
            change_sensitivity_after_read: false,
            reads: 0,
            probes: 0,
            apps: 0,
            image: false,
            text: "generation-fixture".into(),
        }
    }
}
impl ObservationSource for FakeSource {
    fn generation(&mut self) -> isize {
        self.generation
    }
    fn sensitive(&mut self) -> bool {
        self.probes += 1;
        self.sensitive
    }
    fn event(&mut self) -> Option<ClipboardEvent> {
        self.reads += 1;
        if let Some(after) = self.after.take() {
            self.generation = after;
        }
        if self.change_sensitivity_after_read {
            self.sensitive = !self.sensitive;
        }
        if self.pending {
            None
        } else if self.image {
            Some(ClipboardEvent::Image {
                bytes: vec![0; 1024 * 1024 * 4],
                width: 1024,
                height: 1024,
            })
        } else {
            Some(ClipboardEvent::Text(self.text.clone()))
        }
    }
    fn source_app(&mut self) -> Option<SourceApp> {
        self.apps += 1;
        None
    }
}
fn saved(
    _: ClipboardEvent,
    _: &mut WatcherState,
    _: &dyn SensitivityProbe,
    _: Option<SourceApp>,
) -> StepResult {
    StepResult::Saved {
        item_id: "saved".into(),
    }
}
fn ms(time: u64) -> Duration {
    Duration::from_millis(time)
}

#[test]
fn large_image_completed_generation_does_no_payload_privacy_source_or_database_work() {
    let mut source = FakeSource {
        image: true,
        ..Default::default()
    };
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(0), saved).is_some());
    for time in 1..=100 {
        assert!(poll_once(
            &mut source,
            &mut gate,
            &mut state,
            ms(time * 100),
            |_, _, _, _| panic!("database processing repeated")
        )
        .is_none());
    }
    assert_eq!((source.reads, source.probes, source.apps), (1, 1, 1));
}
#[test]
fn zero_lower_wrap_and_new_generation_preempt_backoff() {
    let mut source = FakeSource::default();
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    for generation in [0, 20, 3, isize::MAX, isize::MIN, 0] {
        source.generation = generation;
        state.last_hash = Some("previous".into());
        assert!(poll_once(
            &mut source,
            &mut gate,
            &mut state,
            ms(0),
            |_, state, _, _| {
                assert_eq!(state.last_hash, None);
                StepResult::Skipped(SkipReason::PersistFailed)
            }
        )
        .is_some());
    }
    assert_eq!(source.reads, 6);
}
#[test]
fn pending_and_failed_generation_retry_indefinitely_with_bounded_backoff() {
    let mut source = FakeSource {
        pending: true,
        ..Default::default()
    };
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(0), saved).is_none());
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(99), saved).is_none());
    assert_eq!(source.reads, 1);
    source.pending = false;
    let failure = |_, _: &mut WatcherState, _: &dyn SensitivityProbe, _: Option<SourceApp>| {
        StepResult::Skipped(SkipReason::PersistFailed)
    };
    assert_eq!(
        poll_once(&mut source, &mut gate, &mut state, ms(100), failure),
        Some(StepResult::Skipped(SkipReason::PersistFailed))
    );
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(299), saved).is_none());
    assert_eq!(source.reads, 2);
    for time in [
        300, 700, 1500, 2500, 3500, 4500, 5500, 6500, 7500, 8500, 9500, 10500,
    ] {
        assert!(poll_once(&mut source, &mut gate, &mut state, ms(time), failure).is_some());
        assert!(poll_once(&mut source, &mut gate, &mut state, ms(time + 1), saved).is_none());
    }
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(11500), saved).is_some());
}
#[test]
#[serial_test::serial(skip_set)]
fn torn_snapshot_does_not_consume_own_write_or_mutate_hash_or_persist() {
    let mut source = FakeSource {
        after: Some(1),
        text: "torn-own-write-fixture".into(),
        ..Default::default()
    };
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    state.last_hash = Some("previous-state".into());
    let hash = crate::skip_set::hash_content(source.text.as_bytes());
    crate::skip_set::mark_emitted(hash);
    assert!(poll_once(
        &mut source,
        &mut gate,
        &mut state,
        ms(0),
        |_, _, _, _| panic!("torn event reached worker")
    )
    .is_none());
    assert_eq!(state.last_hash.as_deref(), Some("previous-state"));
    assert!(crate::skip_set::should_skip(hash));
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(1), saved).is_some());
}
#[test]
fn privacy_is_frozen_before_content_read() {
    let tmp = tempfile::tempdir().unwrap();
    let db = Db::open(&tmp.path().join("db")).unwrap();
    let mut source = FakeSource {
        sensitive: true,
        change_sensitivity_after_read: true,
        ..Default::default()
    };
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    let result = poll_once(
        &mut source,
        &mut gate,
        &mut state,
        ms(0),
        |event, state, probe, app| worker_step(event, state, &db, tmp.path(), probe, app),
    );
    assert_eq!(result, Some(StepResult::Skipped(SkipReason::SensitiveFlag)));
    assert!(!source.sensitive);
    assert!(snk_library::clipboard::list(&db, Default::default())
        .unwrap()
        .is_empty());
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(100), saved).is_none());
}
#[test]
fn failed_retry_does_not_reset_worker_hash_again() {
    let mut source = FakeSource::default();
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    poll_once(
        &mut source,
        &mut gate,
        &mut state,
        ms(0),
        |_, state, _, _| {
            state.last_hash = Some("sentinel".into());
            StepResult::Skipped(SkipReason::PersistFailed)
        },
    );
    poll_once(
        &mut source,
        &mut gate,
        &mut state,
        ms(100),
        |_, state, _, _| {
            assert_eq!(state.last_hash.as_deref(), Some("sentinel"));
            saved(
                ClipboardEvent::Text(String::new()),
                state,
                &FrozenSensitivity(false),
                None,
            )
        },
    );
}
#[test]
#[serial_test::serial(skip_set)]
fn new_generation_same_content_reaches_existing_row_after_own_write() {
    let tmp = tempfile::tempdir().unwrap();
    let db = Db::open(&tmp.path().join("db")).unwrap();
    let mut source = FakeSource {
        text: "own-generation-integration".into(),
        ..Default::default()
    };
    crate::skip_set::mark_emitted(crate::skip_set::hash_content(source.text.as_bytes()));
    let mut gate = GenerationGate::default();
    let mut state = WatcherState::new();
    let process = |event, state: &mut WatcherState, probe: &dyn SensitivityProbe, app| {
        worker_step(event, state, &db, tmp.path(), probe, app)
    };
    assert_eq!(
        poll_once(&mut source, &mut gate, &mut state, ms(0), process),
        Some(StepResult::Skipped(SkipReason::OwnWrite))
    );
    assert!(poll_once(&mut source, &mut gate, &mut state, ms(100), process).is_none());
    source.generation += 1;
    let Some(StepResult::Saved { item_id }) =
        poll_once(&mut source, &mut gate, &mut state, ms(101), process)
    else {
        panic!("expected real external copy")
    };
    source.generation += 1;
    assert_eq!(
        poll_once(&mut source, &mut gate, &mut state, ms(102), process),
        Some(StepResult::DedupedTo {
            existing_id: item_id
        })
    );
    assert_eq!(
        snk_library::clipboard::list(&db, Default::default())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn payload_prefers_nonempty_text_and_falls_back_to_image_for_empty_or_unreadable_text() {
    struct Reader {
        text: Option<String>,
        image_reads: usize,
    }
    impl PayloadReader for Reader {
        fn text(&mut self) -> Option<String> {
            self.text.take()
        }
        fn image(&mut self) -> Option<ClipboardEvent> {
            self.image_reads += 1;
            Some(ClipboardEvent::Image {
                bytes: vec![255; 4],
                width: 1,
                height: 1,
            })
        }
    }
    let mut reader = Reader {
        text: Some("text".into()),
        image_reads: 0,
    };
    assert!(
        matches!(read_payload(&mut reader), Some(ClipboardEvent::Text(text)) if text == "text")
    );
    assert_eq!(reader.image_reads, 0);
    for text in [Some(String::new()), None] {
        reader.text = text;
        assert!(matches!(
            read_payload(&mut reader),
            Some(ClipboardEvent::Image { .. })
        ));
    }
    assert_eq!(reader.image_reads, 2);
}
