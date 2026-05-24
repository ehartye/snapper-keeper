//! snk-hotkeys — register global hotkeys and emit events when triggered.
//!
//! Bindings are currently a fixed `HotkeyAction` → default-chord map.
//! User-configurable bindings (reading from `snk-library` settings) are
//! tracked separately; see open issues if you need to wire that in.

use std::sync::atomic::{AtomicBool, Ordering};

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
    ClipboardHistory,
}

impl HotkeyAction {
    pub fn event_name(self) -> &'static str {
        match self {
            HotkeyAction::CaptureFullScreen => "hotkey:capture-full-screen",
            HotkeyAction::CaptureRegion => "hotkey:capture-region",
            HotkeyAction::CaptureWindow => "hotkey:capture-window",
            HotkeyAction::CaptureTimedFullScreen => "hotkey:capture-timed",
            HotkeyAction::ClipboardHistory => "hotkey:clipboard-history",
        }
    }

    pub fn default_chord(self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            HotkeyAction::CaptureFullScreen => "Cmd+Shift+3",
            HotkeyAction::CaptureRegion => "Cmd+Shift+4",
            HotkeyAction::CaptureWindow => "Cmd+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "Cmd+Shift+6",
            HotkeyAction::ClipboardHistory => "Cmd+Shift+V",
        }
        #[cfg(not(target_os = "macos"))]
        match self {
            HotkeyAction::CaptureFullScreen => "CmdOrCtrl+Shift+3",
            HotkeyAction::CaptureRegion => "CmdOrCtrl+Shift+4",
            HotkeyAction::CaptureWindow => "CmdOrCtrl+Shift+5",
            HotkeyAction::CaptureTimedFullScreen => "CmdOrCtrl+Shift+6",
            HotkeyAction::ClipboardHistory => "CmdOrCtrl+Shift+V",
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
            let registered = AtomicBool::new(false);
            app.listen_any("tauri://window-created", move |_event| {
                if registered.swap(true, Ordering::SeqCst) {
                    return;
                }
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
        HotkeyAction::ClipboardHistory,
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
