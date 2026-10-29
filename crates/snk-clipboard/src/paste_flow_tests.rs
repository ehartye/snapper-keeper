use super::*;
use std::cell::Cell;

struct Desktop {
    valid: bool,
    foreground: bool,
    focus_after: usize,
    held_until: usize,
    polls: Cell<usize>,
    sent: Vec<PasteChord>,
}
impl Desktop {
    fn ready() -> Self {
        Self {
            valid: true,
            foreground: true,
            focus_after: 0,
            held_until: 0,
            polls: Cell::new(0),
            sent: vec![],
        }
    }
}
impl PasteDesktop for Desktop {
    fn target_valid(&self) -> bool {
        self.valid
    }
    fn target_foreground(&self) -> bool {
        self.foreground || self.polls.get() >= self.focus_after
    }
    fn keys_released(&self) -> bool {
        self.polls.get() >= self.held_until
    }
    fn focus_target(&mut self) {}
    fn pause(&self) {
        self.polls.set(self.polls.get() + 1);
    }
    fn send(&mut self, chord: PasteChord) -> std::result::Result<(), String> {
        self.sent.push(chord);
        Ok(())
    }
}

#[test]
fn waits_for_shortcut_and_selection_keys_before_sending_once() {
    let mut desktop = Desktop {
        held_until: 4,
        ..Desktop::ready()
    };
    paste_to_target(&mut desktop, PasteChord::CtrlV).unwrap();
    assert_eq!(desktop.polls.get(), 4);
    assert_eq!(desktop.sent, [PasteChord::CtrlV]);
}
#[test]
fn held_modifiers_time_out_without_sending() {
    let mut desktop = Desktop {
        held_until: usize::MAX,
        ..Desktop::ready()
    };
    assert!(paste_to_target(&mut desktop, PasteChord::CtrlV).is_err());
    assert!(desktop.sent.is_empty());
}
#[test]
fn focus_can_settle_before_pasting() {
    let mut desktop = Desktop {
        foreground: false,
        focus_after: 3,
        ..Desktop::ready()
    };
    paste_to_target(&mut desktop, PasteChord::ShiftInsert).unwrap();
    assert_eq!(desktop.polls.get(), 3);
    assert_eq!(desktop.sent, [PasteChord::ShiftInsert]);
}
#[test]
fn denied_focus_never_pastes_into_another_window() {
    let mut desktop = Desktop {
        foreground: false,
        focus_after: usize::MAX,
        ..Desktop::ready()
    };
    assert!(paste_to_target(&mut desktop, PasteChord::CtrlV).is_err());
    assert!(desktop.sent.is_empty());
}
#[test]
fn closed_or_reused_target_is_rejected() {
    let mut desktop = Desktop {
        valid: false,
        ..Desktop::ready()
    };
    assert!(paste_to_target(&mut desktop, PasteChord::CtrlV).is_err());
    assert!(desktop.sent.is_empty());
}
#[test]
fn terminal_classes_use_shift_insert_without_retriggering_history() {
    for class in [
        "CASCADIA_HOSTING_WINDOW_CLASS",
        "ConsoleWindowClass",
        "mintty",
    ] {
        assert_eq!(chord_for_class(class), PasteChord::ShiftInsert, "{class}");
    }
    for class in ["Chrome_WidgetWin_1", "Notepad", "SomeTerminalLookalike", ""] {
        assert_eq!(chord_for_class(class), PasteChord::CtrlV, "{class}");
    }
}
