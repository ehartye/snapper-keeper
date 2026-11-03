# macOS clipboard popup restores the paste target

## Goal and outcome
On macOS, after the user presses Cmd+Shift+V in an input field of another app and picks a clipboard item in the popup, the item is pasted into that app, in the input field that had focus when the hotkey fired. Today the selection is silently lost because nothing returns focus to the originating app before Cmd+V is sent.

## Context and current behavior
- The hotkey `hotkey:clipboard-history` is handled in `app/src/windows/library/LibraryWindow.tsx` (lines 262-276), which calls `show_popup`, then shows and focuses the popup window.
- `show_popup` calls `paste::capture_target` in `crates/snk-clipboard/src/paste.rs`. Only the Windows build captures anything; on macOS it is a no-op, and `prepare_paste` returns an empty `PasteTarget`.
- `paste_item` in `crates/snk-clipboard/src/commands.rs` writes the clipboard, hides the popup, sleeps `PASTE_SETTLE`, then calls `synthesize_paste`. On macOS that posts Cmd+V through CGEvent (requires Accessibility permission) to whichever app is frontmost, with no reactivation of the originating app and no wait for the hotkey keys to be released.
- PR #267 added a shared, platform-neutral paste flow in `crates/snk-clipboard/src/paste_flow.rs` (trait `PasteDesktop`, `wait_for_release`, `paste_to_target`) used by the Windows implementation in `paste_windows.rs`. The module is compiled only for Windows and tests (`crates/snk-clipboard/src/lib.rs` line 13).
- `crates/snk-clipboard/Cargo.toml` already depends on `core-graphics`, `core-foundation`, `objc2`, `objc2-app-kit` (with `NSWorkspace` and `NSRunningApplication`) and `objc2-foundation` for macOS; `platform/macos.rs` already reads the frontmost app.
- Errors cross IPC as `ClipboardError` (`PasteFailed { reason }`, `AccessibilityRequired`, and others). The popup shows the Accessibility banner for `AccessibilityRequired`, shows `reason` for `PasteFailed`, and re-shows itself on failure (`app/src/windows/clipboard-popup/ClipboardPopup.tsx` lines 113-126 and 239-250).
- The user reproduced the failure with an input field in another app (browser, editor): the popup opens, an item is picked, and nothing is pasted.

## Scope and non-goals
### In scope
- A macOS implementation of the existing `PasteDesktop` seam that records the frontmost app when the hotkey fires and reactivates it before Cmd+V.
- Waiting for the hotkey keys to be released before writing the clipboard and before sending Cmd+V on macOS.
- Failure reporting for a closed or unfocusable destination using the existing `PasteFailed` error and popup behavior.

### Non-goals
- Replacing the popup with a non-activating NSPanel.
- Restoring a specific window or accessibility element inside the destination app; field-level focus relies on macOS preserving the app's own focused field across reactivation, and is confirmed manually in the Definition of done.
- Pasting into Snapper Keeper's own windows (Library search, Settings); behavior there stays as it is today.
- Caret-anchored popup positioning on macOS.
- Any change to Windows behavior, the Accessibility permission flow, image paste handling, or the clipboard watcher.
- New Tauri commands, events, or capability entries.

## Acceptance criteria

| ID | Requirement | Failure outcome | Files | Verification command | Depends on |
|---|---|---|---|---|---|
| AC-01 | When the clipboard hotkey fires on macOS while the popup is hidden and the frontmost app is not Snapper Keeper, the system shall store that app's process id as the paste target, replacing any earlier target. | The paste target stays empty or stale and the paste goes to the wrong app. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs, crates/snk-clipboard/src/paste.rs | cargo test -p snk-clipboard paste_macos::tests::captures_frontmost_app | none |
| AC-02 | When the clipboard hotkey fires on macOS while the popup is hidden and the frontmost app is Snapper Keeper, the system shall clear any stored paste target. | A stale target from an earlier invocation receives the paste. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::own_app_clears_target | AC-01 |
| AC-03 | When the clipboard hotkey fires on macOS while the popup is visible, the system shall keep the stored paste target unchanged. | The original destination is replaced by Snapper Keeper itself and the paste is lost. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::visible_popup_keeps_target | AC-01 |
| AC-04 | When `paste_item` runs on macOS, the system shall wait up to 1 second for the Command, Shift and V keys to be released before writing the clipboard. | The clipboard is written while keys are held and Cmd+Shift+V reaches the destination instead of Cmd+V. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs, crates/snk-clipboard/src/paste.rs | cargo test -p snk-clipboard paste_macos::tests::waits_for_key_release | none |
| AC-05 | When the keys are still held after 1 second, the system shall return `PasteFailed` with the reason "Release the shortcut and selection keys, then try again." | The user sees no explanation or a generic error and a paste is attempted with keys held. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::key_release_timeout_fails | AC-04 |
| AC-06 | When a stored target app is still running, the system shall reactivate that app after the popup is hidden. | The destination stays in the background and receives no paste. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::reactivates_running_target | AC-01 |
| AC-07 | When the stored target app has been reactivated, the system shall send Cmd+V only once that app is the frontmost app and the keys are released. | Cmd+V is sent while the popup or another app is frontmost and nothing is pasted. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::sends_only_when_target_frontmost | AC-06 |
| AC-08 | When the stored target app has terminated before paste, the system shall return `PasteFailed` with the reason "The destination closed. Reopen clipboard history from the destination." | The user sees no explanation for the failed paste. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::closed_target_returns_reason | AC-01 |
| AC-09 | When the stored target app has terminated before paste, the system shall send no keystroke. | A Cmd+V is sent to an unrelated app. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::closed_target_sends_nothing | AC-08 |
| AC-10 | When the stored target app is not frontmost within 500 ms of reactivation, the system shall return `PasteFailed` with the reason "Could not focus the destination or release the keys. Reopen history there and try again." | The user sees no explanation for the failed paste. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::focus_timeout_returns_reason | AC-06 |
| AC-11 | When the stored target app is not frontmost within 500 ms of reactivation, the system shall send no keystroke. | A Cmd+V is sent to whatever app is frontmost. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::focus_timeout_sends_nothing | AC-10 |
| AC-12 | When no paste target is stored on macOS, the system shall send Cmd+V to the current frontmost app after the keys are released. | Paste with no stored target regresses from today's behavior. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::no_target_sends_to_frontmost | AC-04 |
| AC-13 | When a paste is sent on macOS, the system shall send the Cmd+V chord regardless of the target app. | A terminal-style chord is sent on macOS and the paste fails. | crates/snk-clipboard/src/paste_flow.rs, crates/snk-clipboard/src/paste_macos.rs | cargo test -p snk-clipboard paste_macos::tests::sends_cmd_v | AC-07 |
| AC-14 | Where Accessibility permission is missing on macOS, the system shall return `AccessibilityRequired` from `prepare_paste`, before waiting for key release, regardless of key state. | The Accessibility banner no longer appears, or a held key changes the error shown. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs, crates/snk-clipboard/src/paste.rs | cargo test -p snk-clipboard paste_macos::tests::missing_accessibility_returns_required_first | none |
| AC-15 | The existing shared `paste_flow` tests shall continue to pass. | The shared flow regresses for Windows. | crates/snk-clipboard/src/paste_flow.rs, crates/snk-clipboard/src/paste_flow_tests.rs | cargo test -p snk-clipboard paste_flow | none |
| AC-16 | The Windows implementation files `paste_windows.rs` and `paste_windows_tests.rs` shall be unchanged by this work. | Windows paste behavior is altered by a macOS fix. | crates/snk-clipboard/src/paste_windows.rs, crates/snk-clipboard/src/paste_windows_tests.rs | git diff --quiet origin/main -- crates/snk-clipboard/src/paste_windows.rs crates/snk-clipboard/src/paste_windows_tests.rs | none |
| AC-17 | The paste command file `commands.rs` shall be unchanged by this work, preserving the clipboard-write, `skip_set::mark_emitted`, hide and settle ordering. | The watcher re-records the app's own paste or the write order regresses. | crates/snk-clipboard/src/commands.rs | git diff --quiet origin/main -- crates/snk-clipboard/src/commands.rs | none |
| AC-18 | The Rust sources shall be formatted per `cargo fmt`. | CI format check fails. | crates/snk-clipboard | cargo fmt -- --check | AC-01 |
| AC-19 | The workspace shall compile without clippy warnings. | CI lint check fails. | crates/snk-clipboard | cargo clippy --workspace --exclude snapper-keeper-app -- -D warnings | AC-01 |
| AC-20 | The workspace test suite shall pass. | CI test job fails. | crates/snk-clipboard | cargo test --workspace --exclude snapper-keeper-app --exclude snk-updater | AC-01 |
| AC-21 | When the clipboard hotkey fires on macOS while the popup is hidden and the system reports no frontmost app, the system shall clear any stored paste target. | A stale target from an earlier invocation receives the paste. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::no_frontmost_app_clears_target | AC-01 |
| AC-22 | When no paste target is stored on macOS, the system shall not attempt to reactivate any app. | A paste attempt activates an unrelated app. | crates/snk-clipboard/src/paste_macos.rs, crates/snk-clipboard/src/paste_macos_tests.rs | cargo test -p snk-clipboard paste_macos::tests::no_target_skips_reactivation | AC-12 |

## Constraints and invariants
- Cross-plugin rules hold: the change lives entirely inside the `snk-clipboard` crate with no new Tauri commands, events or capability entries.
- The macOS implementation uses the existing `PasteDesktop` trait and `paste_to_target` flow rather than a parallel code path; the shared flow compiles on macOS.
- The Accessibility check, the key-release wait and the stored-target check run in `prepare_paste` in that order, and the reactivation and send run inside `synthesize_paste`, so `paste_item` in `commands.rs` needs no change (AC-17). A `PasteFailed` returned by `prepare_paste` propagates out of `paste_item` before any clipboard write.
- The clipboard watcher still ignores the app's own writes (`skip_set::mark_emitted` stays before the clipboard write, AC-17).
- The popup's Accessibility banner and `permissions.rs` are unchanged; only the macOS paste path reports `AccessibilityRequired` (AC-14).
- Text and image pastes share `prepare_paste` and `synthesize_paste`, so both gain the restore behavior; no image-specific logic changes.
- New Rust files stay under 500 lines; macOS code is split from `paste.rs` into `paste_macos.rs` with tests in `paste_macos_tests.rs`.
- Test doubles stand in for NSWorkspace, NSRunningApplication and CGEventSourceKeyState so tests run without a desktop session or Accessibility permission.

## Interfaces and contracts
- None change publicly. `capture_target`, `prepare_paste`, `synthesize_paste` and `paste_item` keep their signatures; the `ClipboardError` wire shape and its `kind` discriminator are unchanged.
- The macOS target is stored in module-level state like the Windows target, is replaced by each fresh hotkey invocation, and is not cleared after a paste.

## Edge cases and errors
- Hotkey pressed again while the popup is already open: the stored target is preserved (AC-03). The popup being visible takes precedence over the frontmost-app check.
- Hotkey pressed with the popup hidden and Snapper Keeper frontmost: the target is cleared (AC-02).
- No stored target at paste time: AC-12 and AC-22.
- No frontmost app reported at hotkey time: AC-21.
- Destination app quits between hotkey and selection: AC-08 and AC-09.
- Destination refuses activation: AC-10 and AC-11.
- Keys still held after the wait: AC-05.
- Accessibility permission missing: AC-14; this check runs first in `prepare_paste`, so it takes precedence over a key-release failure (AC-05).

## Definition of done
- Every acceptance criterion passes its mapped verification.
- Independent review passes and the user approves the spec.
- The user installs a local build from the merged branch and confirms a pick from the popup pastes into an input field of another macOS app.
