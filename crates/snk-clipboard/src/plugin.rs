use std::sync::Arc;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("snk-clipboard")
        .invoke_handler(tauri::generate_handler![
            crate::commands::paste_item,
            crate::commands::show_popup,
            crate::commands::detect_frontmost_app,
        ])
        .setup(|app, _api| {
            let state: tauri::State<'_, snk_library::LibraryState> = app.state();
            let db = Arc::clone(&state.db);
            let root = state.root.clone();
            crate::watcher::start_watcher(db, root);
            Ok(())
        })
        .build()
}
