#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PasteChord {
    CtrlV,
    ShiftInsert,
}

pub(crate) fn chord_for_class(class: &str) -> PasteChord {
    match class {
        "CASCADIA_HOSTING_WINDOW_CLASS" | "ConsoleWindowClass" | "mintty" => {
            PasteChord::ShiftInsert
        }
        _ => PasteChord::CtrlV,
    }
}

pub(crate) trait PasteDesktop {
    fn target_valid(&self) -> bool;
    fn target_foreground(&self) -> bool;
    fn keys_released(&self) -> bool;
    fn focus_target(&mut self);
    fn pause(&self);
    fn send(&mut self, chord: PasteChord) -> std::result::Result<(), String>;
}

pub(crate) fn wait_for_release(desktop: &impl PasteDesktop) -> std::result::Result<(), String> {
    for _ in 0..100 {
        if desktop.keys_released() {
            return Ok(());
        }
        desktop.pause();
    }
    Err("Release the shortcut and selection keys, then try again.".into())
}

pub(crate) fn paste_to_target(
    desktop: &mut impl PasteDesktop,
    chord: PasteChord,
) -> std::result::Result<(), String> {
    if !desktop.target_valid() {
        return Err(
            "The destination closed. Reopen clipboard history from the destination.".into(),
        );
    }
    desktop.focus_target();
    for _ in 0..50 {
        if !desktop.target_valid() {
            break;
        }
        if desktop.target_foreground() && desktop.keys_released() {
            return desktop.send(chord);
        }
        desktop.pause();
    }
    Err(
        "Could not focus the destination or release the keys. Reopen history there and try again."
            .into(),
    )
}

#[cfg(test)]
#[path = "paste_flow_tests.rs"]
mod tests;
