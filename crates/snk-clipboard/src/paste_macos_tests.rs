use super::{capture, prepare, synthesize, MacDesktop, MacSystem};
use crate::paste_flow::{PasteChord, PasteDesktop};
use crate::ClipboardError;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

const PID: i32 = 42;
const RELEASE_ERROR: &str = "Release the shortcut and selection keys, then try again.";
const CLOSED_ERROR: &str = "The destination closed. Reopen clipboard history from the destination.";
const FOCUS_ERROR: &str =
    "Could not focus the destination or release the keys. Reopen history there and try again.";
const CHANGED_ERROR: &str = "The destination or keyboard state changed before paste. Try again.";

struct TestSystem {
    accessible: bool,
    keys: RefCell<VecDeque<bool>>,
    keys_default: bool,
    running: RefCell<VecDeque<bool>>,
    running_default: bool,
    frontmost: RefCell<VecDeque<Option<i32>>>,
    frontmost_default: Option<i32>,
    calls: RefCell<Vec<String>>,
    send_error: Option<&'static str>,
    keys_blocked_until_pause: bool,
    focus_blocked_until_pause: bool,
    paused: Cell<bool>,
}

impl Default for TestSystem {
    fn default() -> Self {
        Self {
            accessible: true,
            keys: RefCell::default(),
            keys_default: true,
            running: RefCell::default(),
            running_default: true,
            frontmost: RefCell::default(),
            frontmost_default: Some(PID),
            calls: RefCell::default(),
            send_error: None,
            keys_blocked_until_pause: false,
            focus_blocked_until_pause: false,
            paused: Cell::new(false),
        }
    }
}

impl TestSystem {
    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    fn log(&self, call: impl Into<String>) {
        self.calls.borrow_mut().push(call.into());
    }

    fn advance_event_loop(&self) {
        self.paused.set(true);
    }
}

impl MacSystem for TestSystem {
    fn accessibility_granted(&self) -> bool {
        self.log("accessibility");
        self.accessible
    }

    fn keys_released(&self) -> bool {
        self.log("keys");
        if self.keys_blocked_until_pause && !self.paused.get() {
            return false;
        }
        self.keys
            .borrow_mut()
            .pop_front()
            .unwrap_or(self.keys_default)
    }

    fn frontmost_pid(&self) -> Option<i32> {
        self.log("frontmost");
        if self.focus_blocked_until_pause && !self.paused.get() {
            return Some(5);
        }
        self.frontmost
            .borrow_mut()
            .pop_front()
            .unwrap_or(self.frontmost_default)
    }

    fn own_pid(&self) -> i32 {
        100
    }

    fn is_running(&self, pid: i32) -> bool {
        self.log(format!("running:{pid}"));
        self.running
            .borrow_mut()
            .pop_front()
            .unwrap_or(self.running_default)
    }

    fn activate(&self, pid: i32) {
        self.log(format!("activate:{pid}"));
    }

    fn pause(&self) {
        self.advance_event_loop();
    }

    fn send_cmd_v(&self) -> std::result::Result<(), String> {
        self.log("cmd-v");
        self.send_error.map_or(Ok(()), |error| Err(error.into()))
    }
}

fn assert_failure(result: crate::Result<()>, expected: &str) {
    match result {
        Err(ClipboardError::PasteFailed { reason }) => assert_eq!(reason, expected),
        other => panic!("expected PasteFailed, got {other:?}"),
    }
}

fn assert_not_sent(sys: &TestSystem) {
    assert!(!sys.calls().iter().any(|call| call == "cmd-v"));
}

#[test]
fn captures_frontmost_app() {
    let mut stored = Some(10);
    capture(false, Some(20), 30, &mut stored);
    assert_eq!(stored, Some(20));
}

#[test]
fn own_app_clears_target() {
    let mut stored = Some(10);
    capture(false, Some(30), 30, &mut stored);
    assert_eq!(stored, None);
}

#[test]
fn visible_popup_keeps_target() {
    let mut stored = Some(10);
    capture(true, Some(30), 30, &mut stored);
    assert_eq!(stored, Some(10));
    capture(true, None, 30, &mut stored);
    assert_eq!(stored, Some(10));
}

#[test]
fn no_frontmost_app_clears_target() {
    let mut stored = Some(10);
    capture(false, None, 30, &mut stored);
    assert_eq!(stored, None);
}

#[test]
fn waits_for_key_release() {
    let sys = TestSystem {
        keys: RefCell::new([false, false, true].into()),
        ..Default::default()
    };
    assert_eq!(prepare(&sys, Some(PID)).unwrap(), Some(PID));
    assert_eq!(sys.calls(), ["accessibility", "keys", "keys", "keys"]);
    assert_not_sent(&sys);
}

#[test]
fn release_wait_advances_system_events() {
    let sys = TestSystem {
        keys_blocked_until_pause: true,
        ..Default::default()
    };
    assert_eq!(prepare(&sys, Some(PID)).unwrap(), Some(PID));
}

#[test]
fn focus_wait_advances_system_events() {
    let sys = TestSystem {
        focus_blocked_until_pause: true,
        ..Default::default()
    };
    synthesize(&sys, Some(PID)).unwrap();
    assert_eq!(sys.calls().last().map(String::as_str), Some("cmd-v"));
}

#[test]
fn key_release_timeout_fails() {
    let sys = TestSystem {
        keys_default: false,
        ..Default::default()
    };
    assert_failure(prepare(&sys, None).map(|_| ()), RELEASE_ERROR);
    assert_eq!(
        sys.calls().iter().filter(|call| *call == "keys").count(),
        100
    );
    assert_not_sent(&sys);
}

#[test]
fn reactivates_running_target() {
    let sys = TestSystem::default();
    synthesize(&sys, Some(PID)).unwrap();
    assert_eq!(
        sys.calls(),
        [
            "running:42",
            "activate:42",
            "running:42",
            "frontmost",
            "keys",
            "running:42",
            "frontmost",
            "keys",
            "cmd-v",
        ]
    );
}

#[test]
fn sends_only_when_target_frontmost() {
    let sys = TestSystem {
        frontmost: RefCell::new([Some(5), Some(PID), Some(PID)].into()),
        keys: RefCell::new([false, true, true].into()),
        ..Default::default()
    };
    synthesize(&sys, Some(PID)).unwrap();
    let calls = sys.calls();
    assert_eq!(calls.last().map(String::as_str), Some("cmd-v"));
    assert_eq!(calls.iter().filter(|call| *call == "frontmost").count(), 4);

    for (running, frontmost, keys) in [
        (vec![true, true, false], vec![Some(PID)], vec![true]),
        (vec![true, true, true], vec![Some(PID), Some(5)], vec![true]),
        (
            vec![true, true, true],
            vec![Some(PID), Some(PID)],
            vec![true, false],
        ),
    ] {
        let sys = TestSystem {
            running: RefCell::new(running.into()),
            frontmost: RefCell::new(frontmost.into()),
            keys: RefCell::new(keys.into()),
            ..Default::default()
        };
        assert_failure(synthesize(&sys, Some(PID)), CHANGED_ERROR);
        assert_not_sent(&sys);
    }
}

#[test]
fn closed_target_returns_reason() {
    let sys = TestSystem {
        running_default: false,
        ..Default::default()
    };
    assert_failure(synthesize(&sys, Some(PID)), CLOSED_ERROR);
}

#[test]
fn closed_target_sends_nothing() {
    let sys = TestSystem {
        running_default: false,
        ..Default::default()
    };
    assert_failure(synthesize(&sys, Some(PID)), CLOSED_ERROR);
    assert_not_sent(&sys);
    assert_eq!(sys.calls(), ["running:42"]);
}

#[test]
fn focus_timeout_returns_reason() {
    let sys = TestSystem {
        frontmost_default: Some(5),
        ..Default::default()
    };
    assert_failure(synthesize(&sys, Some(PID)), FOCUS_ERROR);
}

#[test]
fn focus_timeout_sends_nothing() {
    let sys = TestSystem {
        frontmost_default: Some(5),
        ..Default::default()
    };
    assert_failure(synthesize(&sys, Some(PID)), FOCUS_ERROR);
    assert_not_sent(&sys);
    assert_eq!(
        sys.calls()
            .iter()
            .filter(|call| *call == "frontmost")
            .count(),
        50
    );
}

#[test]
fn no_target_sends_to_frontmost() {
    let sys = TestSystem::default();
    assert_eq!(prepare(&sys, None).unwrap(), None);
    synthesize(&sys, None).unwrap();
    assert_eq!(sys.calls(), ["accessibility", "keys", "cmd-v"]);

    let sys = TestSystem {
        send_error: Some("injection failed"),
        ..Default::default()
    };
    assert_failure(synthesize(&sys, None), "injection failed");
}

#[test]
fn no_target_skips_reactivation() {
    let sys = TestSystem::default();
    synthesize(&sys, None).unwrap();
    assert_eq!(sys.calls(), ["cmd-v"]);
}

#[test]
fn sends_cmd_v() {
    let sys = TestSystem::default();
    let mut desktop = MacDesktop {
        sys: &sys,
        pid: PID,
    };
    desktop.send(PasteChord::ShiftInsert).unwrap();
    assert_eq!(sys.calls(), ["running:42", "frontmost", "keys", "cmd-v"]);
}

#[test]
fn missing_accessibility_returns_required_first() {
    let sys = TestSystem {
        accessible: false,
        keys_default: false,
        ..Default::default()
    };
    assert!(matches!(
        prepare(&sys, Some(PID)),
        Err(ClipboardError::AccessibilityRequired)
    ));
    assert_eq!(sys.calls(), ["accessibility"]);
    assert_not_sent(&sys);
}
