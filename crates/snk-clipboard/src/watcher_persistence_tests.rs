use super::*;

struct FailingStore {
    fail_at: std::cell::Cell<&'static str>,
    existing: bool,
    inserts: std::cell::Cell<usize>,
}
impl Store for FailingStore {
    fn blocked(&self, _: &SourceApp) -> bool {
        false
    }
    fn find(&self, _: &str) -> snk_library::Result<Option<String>> {
        self.check("find")?;
        Ok(self.existing.then(|| "existing-row".into()))
    }
    fn bump(&self, _: &str) -> snk_library::Result<()> {
        self.check("bump")
    }
    fn insert(&self, _: NewClipboardItem) -> snk_library::Result<String> {
        self.inserts.set(self.inserts.get() + 1);
        self.check("insert")?;
        Ok("new-row".into())
    }
    fn evict(&self) {}
}
impl FailingStore {
    fn new(fail: &'static str, existing: bool) -> Self {
        Self {
            fail_at: std::cell::Cell::new(fail),
            existing,
            inserts: std::cell::Cell::new(0),
        }
    }
    fn check(&self, operation: &str) -> snk_library::Result<()> {
        if self.fail_at.get() == operation {
            Err(snk_library::LibraryError::NotFound {
                what: operation.into(),
            })
        } else {
            Ok(())
        }
    }
}
#[test]
fn database_failures_do_not_acknowledge_or_fall_through_to_insert() {
    let tmp = tempfile::tempdir().unwrap();
    for (failure, existing) in [("find", false), ("bump", true), ("insert", false)] {
        let store = FailingStore::new(failure, existing);
        let mut state = WatcherState::new();
        let step = |state: &mut WatcherState| {
            worker_step_with_store(
                ClipboardEvent::Text("transient-db-failure".into()),
                state,
                &store,
                tmp.path(),
                &FakeProbe { answer: false },
                None,
            )
        };
        assert_eq!(
            step(&mut state),
            StepResult::Skipped(SkipReason::PersistFailed)
        );
        assert_eq!(state.last_hash, None);
        assert_eq!(store.inserts.get(), usize::from(failure == "insert"));
        store.fail_at.set("");
        let result = step(&mut state);
        if existing {
            assert_eq!(
                result,
                StepResult::DedupedTo {
                    existing_id: "existing-row".into()
                }
            );
        } else {
            assert!(matches!(result, StepResult::Saved { .. }));
        }
        assert!(state.last_hash.is_some());
    }
}
#[test]
fn failed_image_inserts_leave_no_files_across_retries() {
    let tmp = tempfile::tempdir().unwrap();
    let store = FailingStore::new("insert", false);
    let mut state = WatcherState::new();
    for _ in 0..20 {
        assert_eq!(
            worker_step_with_store(
                ClipboardEvent::Image {
                    bytes: red_2x2_rgba(),
                    width: 2,
                    height: 2
                },
                &mut state,
                &store,
                tmp.path(),
                &FakeProbe { answer: false },
                None
            ),
            StepResult::Skipped(SkipReason::PersistFailed)
        );
        assert_eq!(state.last_hash, None);
        assert_eq!(state.pending_image, None);
    }
    fn count_files(path: &Path) -> usize {
        std::fs::read_dir(path)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    count_files(&path)
                } else {
                    1
                }
            })
            .sum()
    }
    assert_eq!(count_files(tmp.path()), 0);
}
#[test]
fn failed_cleanup_prevents_allocating_more_image_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = files::clipboard_image_relative_path(&uuid::Uuid::now_v7());
    // A directory at the exact file path causes remove_file to fail on every OS.
    std::fs::create_dir_all(tmp.path().join(&relative)).unwrap();
    let mut state = WatcherState::new();
    state.pending_image = Some(relative.clone());
    let store = FailingStore::new("insert", false);
    for _ in 0..20 {
        assert_eq!(
            worker_step_with_store(
                ClipboardEvent::Image {
                    bytes: red_2x2_rgba(),
                    width: 2,
                    height: 2
                },
                &mut state,
                &store,
                tmp.path(),
                &FakeProbe { answer: false },
                None
            ),
            StepResult::Skipped(SkipReason::PersistFailed)
        );
        assert_eq!(state.pending_image.as_ref(), Some(&relative));
    }
    assert_eq!(store.inserts.get(), 0);
    assert_eq!(
        std::fs::read_dir(tmp.path().join(relative.parent().unwrap()))
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn repeated_external_copy_keeps_existing_row_identity() {
    let (tmp, db) = fresh_db();
    let mut state = WatcherState::new();
    let event = || ClipboardEvent::Text("external-recopy-identity".into());
    let first = worker_step(
        event(),
        &mut state,
        &db,
        tmp.path(),
        &FakeProbe { answer: false },
        None,
    );
    let StepResult::Saved { item_id } = first else {
        panic!("expected saved")
    };
    state.last_hash = None;
    assert_eq!(
        worker_step(
            event(),
            &mut state,
            &db,
            tmp.path(),
            &FakeProbe { answer: false },
            None
        ),
        StepResult::DedupedTo {
            existing_id: item_id
        }
    );
    assert_eq!(
        snk_library::clipboard::list(&db, Default::default())
            .unwrap()
            .len(),
        1
    );
}
