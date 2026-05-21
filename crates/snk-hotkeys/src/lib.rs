//! snk-hotkeys — register global hotkeys and emit events when triggered.
//!
//! Phase 1 wires a fixed set of action ids → default chords. A later phase
//! reads bindings from `snk-library` (settings) and supports remapping.

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HotkeyAction {
    CaptureFullScreen,
    CaptureRegion,
    CaptureWindow,
    CaptureTimedFullScreen,
}

impl HotkeyAction {
    pub fn event_name(self) -> &'static str {
        match self {
            HotkeyAction::CaptureFullScreen => "hotkey:capture-full-screen",
            HotkeyAction::CaptureRegion => "hotkey:capture-region",
            HotkeyAction::CaptureWindow => "hotkey:capture-window",
            HotkeyAction::CaptureTimedFullScreen => "hotkey:capture-timed",
        }
    }

    pub fn default_chord(self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            HotkeyAction::CaptureFullScreen => "Cmd+Shift+3",
            HotkeyAction::CaptureRegion => "Cmd+Shift+4",
            HotkeyAction::CaptureWindow => "Cmd+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "Cmd+Shift+6",
        }
        #[cfg(not(target_os = "macos"))]
        match self {
            HotkeyAction::CaptureFullScreen => "CmdOrCtrl+Shift+3",
            HotkeyAction::CaptureRegion => "CmdOrCtrl+Shift+4",
            HotkeyAction::CaptureWindow => "CmdOrCtrl+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "CmdOrCtrl+Shift+6",
        }
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-hotkeys")
        .setup(|app, _api| {
            // Defer registration until a window's HWND exists and the message
            // pump is running — otherwise Windows' RegisterHotKey returns
            // error 1459 (interactive window station).
            let handle = app.app_handle().clone();
            app.listen_any("tauri://window-created", move |_event| {
                if let Err(e) = register_defaults(&handle) {
                    warn!(error = %e, "failed to register default hotkeys");
                }
            });
            Ok(())
        })
        .build()
}

fn register_defaults<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let actions = [
        HotkeyAction::CaptureFullScreen,
        HotkeyAction::CaptureRegion,
        HotkeyAction::CaptureWindow,
        HotkeyAction::CaptureTimedFullScreen,
    ];
    for action in actions {
        let chord = action.default_chord();
        let app2 = app.clone();
        app.global_shortcut()
            .on_shortcut(chord, move |_app, _sc, ev| {
                if matches!(ev.state(), ShortcutState::Pressed) {
                    let _ = app2.emit(action.event_name(), ());
                }
            })
            .map_err(|e| format!("register {chord}: {e}"))?;
        info!(%chord, action = ?action, "registered hotkey");
    }
    Ok(())
}
