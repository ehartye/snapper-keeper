use std::borrow::Cow;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use tauri::{Runtime, State};
use tracing::info;
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSRunningApplication};
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;

use snk_library::clipboard::{self, ClipboardItemKind};
use snk_library::{settings, LibraryState};

use crate::caret;
use crate::paste;
use crate::skip_set;
use crate::source_app::{self, SourceApp, SourceAppKind};
use crate::{ClipboardError, Result};

/// Wait between writing to the clipboard and synthesizing Ctrl/Cmd+V so the
/// target app's clipboard listener has time to observe the new content before
/// the paste keystroke arrives.
const PASTE_SETTLE: Duration = Duration::from_millis(50);
const FOCUS_RESTORE_POLL: Duration = Duration::from_millis(20);
const FOCUS_RESTORE_ATTEMPTS: usize = 25;

const IMAGE_PASTE_ENABLED_KEY: &str = "clipboard.image_paste_enabled";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupContext {
    pub caret: crate::caret::CaretPosition,
    pub target_app: Option<SourceApp>,
}

fn wait_for_focus_restore_with<F, S>(target_app: &SourceApp, mut current: F, mut sleep: S) -> bool
where
    F: FnMut() -> Option<SourceApp>,
    S: FnMut(Duration),
{
    for _ in 0..FOCUS_RESTORE_ATTEMPTS {
        if current().as_ref().is_some_and(|app| {
            app.kind == target_app.kind && app.identifier_matches(&target_app.identifier)
        }) {
            return true;
        }
        sleep(FOCUS_RESTORE_POLL);
    }
    false
}

fn restore_paste_target<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    target_app: &SourceApp,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        if target_app.kind != SourceAppKind::MacosBundleId {
            return Ok(());
        }
        let bundle_id = target_app.identifier.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        window
            .run_on_main_thread(move || {
                let activated = if let Some(mtm) = MainThreadMarker::new() {
                    let app = NSApplication::sharedApplication(mtm);
                    let current = NSRunningApplication::currentApplication();
                    let bundle_id = NSString::from_str(&bundle_id);
                    let running =
                        NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);
                    if !running.is_empty() {
                        let target = running.objectAtIndex(0);
                        app.yieldActivationToApplication(&target);
                        target.activateFromApplication_options(
                            &current,
                            NSApplicationActivationOptions::empty(),
                        )
                    } else {
                        false
                    }
                } else {
                    false
                };
                let _ = tx.send(activated);
            })
            .map_err(|e| ClipboardError::PasteFailed {
                reason: format!("schedule focus restore: {e}"),
            })?;
        let activated = rx.recv_timeout(Duration::from_millis(250)).unwrap_or(false);
        if !activated {
            return Err(ClipboardError::PasteFailed {
                reason: format!("failed to reactivate {}", target_app.display_name),
            });
        }
        if !wait_for_focus_restore_with(target_app, source_app::current, thread::sleep) {
            return Err(ClipboardError::PasteFailed {
                reason: format!("timed out restoring focus to {}", target_app.display_name),
            });
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, target_app);
    Ok(())
}

#[tauri::command]
pub fn paste_item<R: Runtime>(
    state: State<'_, LibraryState>,
    window: tauri::WebviewWindow<R>,
    id: String,
    target_app: Option<SourceApp>,
) -> Result<()> {
    snk_library::authz::authorize(
        &window,
        "paste_item",
        snk_library::authz::PASTE_ITEM_WINDOWS,
        &id,
    )?;
    let item = clipboard::get(&state.db, &id)?;

    match item.kind {
        ClipboardItemKind::Text => {
            let text = item
                .text_content
                .as_deref()
                .ok_or_else(|| ClipboardError::PasteFailed {
                    reason: "text item has no text_content".into(),
                })?;

            let mut clip = Clipboard::new()?;
            // Mark this content as self-emitted so the watcher's skip_set
            // recognizes the next observed event with this hash and ignores
            // it (within SKIP_TTL). Replaces the old SKIP_NEXT AtomicBool.
            skip_set::mark_emitted(skip_set::hash_content(text.as_bytes()));
            clip.set_text(text)?;

            if let Some(target_app) = target_app.as_ref() {
                restore_paste_target(&window, target_app)?;
            }

            thread::sleep(PASTE_SETTLE);

            paste::synthesize_paste()?;
            clipboard::bump_timestamp(&state.db, &id)?;

            info!(id = %id, kind = "text", "pasted clipboard item");
            Ok(())
        }

        ClipboardItemKind::Image => {
            // Honor the image_paste_enabled setting (default: true).
            let enabled = match settings::get(&state.db, IMAGE_PASTE_ENABLED_KEY)? {
                Some(v) => v.as_bool().unwrap_or(true),
                None => true,
            };
            if !enabled {
                return Err(ClipboardError::PasteFailed {
                    reason: "image paste is disabled (clipboard.image_paste_enabled = false)"
                        .into(),
                });
            }

            let file_path = item.file_path.ok_or_else(|| ClipboardError::PasteFailed {
                reason: "image item has no file_path".into(),
            })?;

            let full_path = state.root.join(&file_path);
            let png_bytes = std::fs::read(&full_path).map_err(|e| ClipboardError::PasteFailed {
                reason: format!("read image file: {e}"),
            })?;

            // Decode PNG → raw RGBA so arboard can place it on the clipboard.
            let img =
                image::load_from_memory(&png_bytes).map_err(|e| ClipboardError::PasteFailed {
                    reason: format!("decode image: {e}"),
                })?;
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let rgba_bytes = rgba.into_raw();

            // Mark the RGBA bytes hash so the watcher skips its own
            // re-observation of what we're about to write.
            skip_set::mark_emitted(skip_set::hash_content(&rgba_bytes));

            let mut clip = Clipboard::new()?;
            clip.set_image(arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(rgba_bytes),
            })?;

            if let Some(target_app) = target_app.as_ref() {
                restore_paste_target(&window, target_app)?;
            }

            thread::sleep(PASTE_SETTLE);

            paste::synthesize_paste()?;
            clipboard::bump_timestamp(&state.db, &id)?;

            info!(id = %id, kind = "image", width, height, "pasted clipboard item");
            Ok(())
        }
    }
}

#[tauri::command]
pub fn show_popup<R: Runtime>(_app: tauri::AppHandle<R>) -> Result<PopupContext> {
    Ok(PopupContext {
        caret: caret::resolve_popup_position(),
        target_app: source_app::current(),
    })
}

#[tauri::command]
pub fn detect_frontmost_app<R: Runtime>(_app: tauri::AppHandle<R>) -> Option<SourceApp> {
    source_app::current()
}

/// Report whether the clipboard watcher currently has the OS clipboard open,
/// and the last open error if it is offline. Lets the popup render a banner
/// instead of silently showing an empty history.
#[tauri::command]
pub fn clipboard_status(
    health: State<'_, crate::health::ClipboardHealth>,
) -> crate::health::ClipboardStatus {
    health.snapshot()
}

/// Report whether the process currently holds the OS permission auto-paste
/// needs (macOS Accessibility). Lets the popup warn up-front instead of having
/// the paste keystroke silently swallowed. Always granted off macOS.
#[tauri::command]
pub fn clipboard_permission_status() -> crate::permissions::PermissionStatus {
    crate::permissions::status()
}

/// Open the OS settings pane where the user grants auto-paste permission.
#[tauri::command]
pub fn open_accessibility_settings() -> Result<()> {
    crate::permissions::open_accessibility_settings()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_for_focus_restore_stops_when_target_app_returns() {
        let target = SourceApp {
            identifier: "com.apple.TextEdit".into(),
            display_name: "TextEdit".into(),
            kind: SourceAppKind::MacosBundleId,
        };
        let mut calls = 0usize;
        let mut sleeps = 0usize;
        let restored = wait_for_focus_restore_with(
            &target,
            || {
                calls += 1;
                if calls < 3 {
                    Some(SourceApp {
                        identifier: "com.snapper-keeper.app".into(),
                        display_name: "Snapper Keeper".into(),
                        kind: SourceAppKind::MacosBundleId,
                    })
                } else {
                    Some(target.clone())
                }
            },
            |_| sleeps += 1,
        );
        assert!(restored);
        assert_eq!(calls, 3);
        assert_eq!(sleeps, 2);
    }

    #[test]
    fn wait_for_focus_restore_times_out_when_target_never_returns() {
        let target = SourceApp {
            identifier: "com.apple.TextEdit".into(),
            display_name: "TextEdit".into(),
            kind: SourceAppKind::MacosBundleId,
        };
        let mut sleeps = 0usize;
        let restored = wait_for_focus_restore_with(
            &target,
            || {
                Some(SourceApp {
                    identifier: "com.snapper-keeper.app".into(),
                    display_name: "Snapper Keeper".into(),
                    kind: SourceAppKind::MacosBundleId,
                })
            },
            |_| sleeps += 1,
        );
        assert!(!restored);
        assert_eq!(sleeps, FOCUS_RESTORE_ATTEMPTS);
    }
}
