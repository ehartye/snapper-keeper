use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::health::{ClipboardHealth, HealthSink};

/// Event emitted when the clipboard watcher cannot open the OS clipboard, so a
/// popup can render an offline banner. A `clipboard:available` companion is
/// emitted on recovery.
pub const CLIPBOARD_UNAVAILABLE_EVENT: &str = "clipboard:unavailable";
pub const CLIPBOARD_AVAILABLE_EVENT: &str = "clipboard:available";

/// Production `HealthSink`: updates the shared `ClipboardHealth` (read by the
/// `clipboard_status` command) and emits a Tauri event on each transition.
struct AppHealthSink<R: Runtime> {
    app: AppHandle<R>,
    health: ClipboardHealth,
}

impl<R: Runtime> HealthSink for AppHealthSink<R> {
    fn report_available(&self) {
        let status = self.health.mark_available();
        let _ = self.app.emit(CLIPBOARD_AVAILABLE_EVENT, status);
    }

    fn report_unavailable(&self, error: String) {
        let status = self.health.mark_unavailable(error);
        let _ = self.app.emit(CLIPBOARD_UNAVAILABLE_EVENT, status);
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-clipboard")
        .invoke_handler(tauri::generate_handler![
            crate::commands::paste_item,
            crate::commands::show_popup,
            crate::commands::detect_frontmost_app,
            crate::commands::clipboard_status,
            crate::commands::clipboard_permission_status,
            crate::commands::open_accessibility_settings,
        ])
        .setup(|app, _api| {
            let state: tauri::State<'_, snk_library::LibraryState> = app.state();
            let db = Arc::clone(&state.db);
            let root = state.root.clone();

            let health = ClipboardHealth::new();
            app.manage(health.clone());
            let sink: Arc<dyn HealthSink> = Arc::new(AppHealthSink {
                app: app.app_handle().clone(),
                health,
            });

            crate::watcher::start_watcher(db, root, sink);
            Ok(())
        })
        .build()
}
