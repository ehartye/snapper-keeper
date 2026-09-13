//! Uses a dedicated named pasteboard, never the user's general clipboard.
use super::*;
use crate::observation::{poll_once, GenerationGate, PayloadReader};
use crate::watcher::{SkipReason, StepResult};
use objc2::rc::Retained;
use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSArray, NSString};
use std::time::Duration;

struct NamedBoard(Retained<NSPasteboard>);
impl Drop for NamedBoard {
    fn drop(&mut self) {
        // objc2-app-kit omits this legacy selector. It takes no arguments and returns void;
        // only this fixture's uniquely named pasteboard is released from the pasteboard server.
        unsafe {
            let _: () = objc2::msg_send![&*self.0, releaseGlobally];
        }
    }
}
struct NamedReader<'a> {
    board: &'a NSPasteboard,
    reads: usize,
}
impl PayloadReader for NamedReader<'_> {
    fn text(&mut self) -> Option<String> {
        self.reads += 1;
        self.board
            .stringForType(&NSString::from_str("public.utf8-plain-text"))
            .map(|value| value.to_string())
    }
    fn image(&mut self) -> Option<ClipboardEvent> {
        None
    }
}

#[test]
fn named_pasteboard_delayed_data_retries_same_generation_and_recopy_deduplicates() {
    objc2::rc::autoreleasepool(|_| {
        let board = NamedBoard(NSPasteboard::pasteboardWithUniqueName());
        let text_type = NSString::from_str("public.utf8-plain-text");
        let types = NSArray::from_slice(&[&*text_type]);
        // No provider/owner object is supplied; the fixture explicitly fulfills the declaration.
        unsafe {
            board.0.declareTypes_owner(&types, None);
        }
        let generation = board.0.changeCount();
        let tmp = tempfile::tempdir().unwrap();
        let db = Db::open(&tmp.path().join("db")).unwrap();
        let mut reader = NamedReader {
            board: &board.0,
            reads: 0,
        };
        let mut source = MacSource {
            clipboard: &mut reader,
            pasteboard: &board.0,
        };
        let mut state = WatcherState::new();
        let mut gate = GenerationGate::default();
        let process =
            |event,
             state: &mut WatcherState,
             probe: &dyn crate::sensitivity::SensitivityProbe,
             app| { worker_step(event, state, &db, tmp.path(), probe, app) };
        assert!(poll_once(&mut source, &mut gate, &mut state, Duration::ZERO, process).is_none());
        assert!(board
            .0
            .setString_forType(&NSString::from_str("named delayed data"), &text_type));
        assert_eq!(
            board.0.changeCount(),
            generation,
            "fulfilling a declared type must not advance ownership"
        );
        let Some(StepResult::Saved { item_id }) = poll_once(
            &mut source,
            &mut gate,
            &mut state,
            Duration::from_millis(100),
            process,
        ) else {
            panic!("delayed payload was not retried")
        };
        for time in 2..=101 {
            assert!(poll_once(
                &mut source,
                &mut gate,
                &mut state,
                Duration::from_millis(time * 100),
                process
            )
            .is_none());
        }
        assert_eq!(source.clipboard.reads, 2);
        board.0.clearContents();
        assert!(board
            .0
            .setString_forType(&NSString::from_str("named delayed data"), &text_type));
        assert_ne!(board.0.changeCount(), generation);
        assert_eq!(
            poll_once(
                &mut source,
                &mut gate,
                &mut state,
                Duration::from_millis(10200),
                process
            ),
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
    });
}

#[test]
fn named_pasteboard_privacy_uses_the_observed_board() {
    objc2::rc::autoreleasepool(|_| {
        let board = NamedBoard(NSPasteboard::pasteboardWithUniqueName());
        let text_type = NSString::from_str("public.utf8-plain-text");
        let concealed = NSString::from_str("org.nspasteboard.ConcealedType");
        let types = NSArray::from_slice(&[&*text_type, &*concealed]);
        unsafe {
            board.0.declareTypes_owner(&types, None);
        }
        assert!(board
            .0
            .setString_forType(&NSString::from_str("private named data"), &text_type));
        let tmp = tempfile::tempdir().unwrap();
        let db = Db::open(&tmp.path().join("db")).unwrap();
        let mut reader = NamedReader {
            board: &board.0,
            reads: 0,
        };
        let mut source = MacSource {
            clipboard: &mut reader,
            pasteboard: &board.0,
        };
        let result = poll_once(
            &mut source,
            &mut GenerationGate::default(),
            &mut WatcherState::new(),
            Duration::ZERO,
            |event, state, probe, app| worker_step(event, state, &db, tmp.path(), probe, app),
        );
        assert_eq!(result, Some(StepResult::Skipped(SkipReason::SensitiveFlag)));
        assert!(snk_library::clipboard::list(&db, Default::default())
            .unwrap()
            .is_empty());
    });
}
